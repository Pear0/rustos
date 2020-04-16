use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::ops::Add;
use core::ops::Deref;
use core::time::Duration;
use alloc::sync::Arc;

use kernel_api::{OsError, OsResult};

use aarch64;
use aarch64::SPSR_EL1;
use shim::path::Path;

use crate::{FILESYSTEM, smp, VMM, SCHEDULER};
use crate::param::*;
use crate::process::{Stack, State, TimeRatio, TimeRing};
use crate::traps::TrapFrame;
use crate::vm::*;
use crate::sync::Completion;
use shim::{io, ioerr};
use crate::pigrate::bundle::{ProcessBundle, MemoryBundle};
use crate::process::fd::FileDescriptor;
use crate::fs::handle::{Source, Sink};
use crate::process::address_space::{AddressSpaceManager, Region, RegionKind};

/// Type alias for the type of a process ID.
pub type Id = u64;

#[derive(Clone, Copy)]
pub struct CoreAffinity([bool; smp::MAX_CORES]);

impl CoreAffinity {
    pub fn all() -> Self {
        CoreAffinity([true; smp::MAX_CORES])
    }

    pub fn set_all(&mut self) {
        self.0 = [true; smp::MAX_CORES];
    }

    pub fn set_only(&mut self, core: usize) {
        self.0 = [false; smp::MAX_CORES];
        if core < self.0.len() {
            self.0[core] = true;
        }
    }

    pub fn check(&self, core: usize) -> bool {
        core < self.0.len() && self.0[core]
    }
}

impl fmt::Debug for CoreAffinity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut num: u32 = 0;
        for e in &self.0 {
            num <<= 1;
            if *e {
                num |= 1;
            }
        }
        f.write_fmt(format_args!("CoreAffinity({:b})", num))
    }
}

pub struct KernProcessCtx {
    pid: Id,
}

impl KernProcessCtx {

    /// Will panic if file descriptors are not assigned.
    pub fn get_stdio_or_panic(&self) -> (Arc<Source>, Arc<Sink>) {
        SCHEDULER.crit_process(self.pid, |f| {
            let f = f.unwrap();
            (f.file_descriptors[0].read.as_ref().unwrap().clone(), f.file_descriptors[1].write.as_ref().unwrap().clone())
        })
    }

}

type KernProcess = Box<dyn FnOnce(KernProcessCtx) + Send>;

/// A structure that represents the complete state of a process.
pub struct Process {
    /// The saved trap frame of a process.
    pub context: Box<TrapFrame>,
    /// The memory allocation used for the process's stack.
    pub stack: Stack,
    /// The page table describing the Virtual Memory of the process
    pub vmap: Box<AddressSpaceManager>,
    /// The scheduling state of the process.
    state: State,

    pub name: String,

    pub cpu_time: Duration,

    pub ready_ratio: TimeRatio,
    pub running_ratio: TimeRatio,
    pub waiting_ratio: TimeRatio,

    pub running_slices: TimeRing,

    pub task_switches: usize,

    pub affinity: CoreAffinity,

    pub request_suspend: bool,
    request_kill: bool,

    pub file_descriptors: Vec<FileDescriptor>,

    pub dead_completions: Vec<Arc<Completion<Id>>>,

    kernel_proc_entry: Option<KernProcess>,
}

impl Process {
    /// Creates a new process with a zeroed `TrapFrame` (the default), a zeroed
    /// stack of the default size, and a state of `Ready`.
    ///
    /// If enough memory could not be allocated to start the process, returns
    /// `None`. Otherwise returns `Some` of the new `Process`.
    pub fn new(name: String) -> OsResult<Process> {
        let vmap = Box::new(AddressSpaceManager::new());
        let stack = Stack::new().ok_or(OsError::NoMemory)?;
        let context = Box::new(TrapFrame::default());

        Ok(Process {
            context,
            stack,
            vmap,
            state: State::Ready,
            name,
            cpu_time: Duration::from_millis(0),
            ready_ratio: TimeRatio::new(),
            running_ratio: TimeRatio::new(),
            waiting_ratio: TimeRatio::new(),
            running_slices: TimeRing::new(),
            affinity: CoreAffinity::all(),
            task_switches: 0,
            request_suspend: false,
            request_kill: false,
            file_descriptors: Vec::new(),
            dead_completions: Vec::new(),
            kernel_proc_entry: None,
        })
    }

    pub fn kernel_process_old(name: String, f: fn() -> !) -> OsResult<Process> {
        use crate::VMM;

        let mut p = Process::new(name)?;

        p.context.sp = p.stack.top().as_u64();
        p.context.elr = f as u64;

        p.context.spsr |= SPSR_EL1::M & 0b0100;

        p.context.ttbr0 = VMM.get_baddr().as_u64();
        // kernel thread still gets a vmap because it's easy
        p.context.ttbr1 = p.vmap.get_baddr().as_u64();

        Ok(p)
    }

    fn kernel_process_bootstrap() -> ! {
        let pid: Id = kernel_api::syscall::getpid();

        let entry = SCHEDULER.crit_process(pid, |proc| {
            proc.map(|proc| proc.kernel_proc_entry.take() ) });

        let entry = match entry {
            None => {
                error!("kernel process pid={} could not find itself!", pid);
                kernel_api::syscall::exit();
            }
            Some(None) => {
                error!("kernel_process_bootstrap() pid={} launched with no kernel_proc_entry!", pid);
                kernel_api::syscall::exit();
            }
            Some(Some(entry)) => entry,
        };

        entry(KernProcessCtx { pid });
        kernel_api::syscall::exit();
    }

    pub fn kernel_process_boxed(name: String, f: KernProcess) -> OsResult<Process> {
        let mut proc = Self::kernel_process_old(name, Self::kernel_process_bootstrap)?;
        proc.kernel_proc_entry = Some(f);
        Ok(proc)
    }

    pub fn kernel_process<F: FnOnce(KernProcessCtx) + Send + 'static>(name: String, f: F) -> OsResult<Process> {
        Self::kernel_process_boxed(name, Box::new(f))
    }

    /// Load a program stored in the given path by calling `do_load()` method.
    /// Set trapframe `context` corresponding to the its page table.
    /// `sp` - the address of stack top
    /// `elr` - the address of image base.
    /// `ttbr0` - the base address of kernel page table
    /// `ttbr1` - the base address of user page table
    /// `spsr` - `F`, `A`, `D` bit should be set.
    ///
    /// Returns Os Error if do_load fails.
    pub fn load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        use crate::VMM;

        let mut p = Process::do_load(pn)?;

        p.context.sp = Process::get_stack_top().as_u64();
        p.context.elr = USER_IMG_BASE as u64;

        p.context.ttbr0 = VMM.get_baddr().as_u64();
        p.context.ttbr1 = p.vmap.get_baddr().as_u64();

        Ok(p)
    }

    /// Creates a process and open a file with given path.
    /// Allocates one page for stack with read/write permission, and N pages with read/write/execute
    /// permission to load file's contents.
    fn do_load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        use fat32::traits::*;
        use shim::io::Read;
        let mut proc = Process::new(pn.as_ref().to_str().ok_or(OsError::InvalidArgument)?.to_owned())?;

        proc.vmap.add_region(Region::new(Process::get_stack_base(), PAGE_SIZE, RegionKind::Normal));

        let image_base = Process::get_image_base();

        let mut file = FILESYSTEM.open(pn)?.into_file().ok_or(OsError::InvalidArgument)?;

        let mut base = image_base;
        'page_loop: loop {
            if image_base == base {
                proc.vmap.add_region(Region::new(image_base, PAGE_SIZE, RegionKind::Normal));
            } else {
                proc.vmap.expand_region(image_base, PAGE_SIZE);
            }

            let mut buf = proc.vmap.get_page_mut(base).expect("tried to deref bad page");

            while buf.len() > 0 {
                let read_amount = file.read(buf)?;
                if read_amount == 0 {
                    break 'page_loop;
                }
                buf = &mut buf[read_amount..];
            }

            base = base + VirtualAddr::from(PAGE_SIZE);
        }

        Ok(proc)
    }

    pub fn from_bundle(bundle: &ProcessBundle) -> OsResult<Process> {
        let mut proc = Process::new(bundle.name.clone())?;

        proc.context.decode_from_bytes(&bundle.frame).map_err(|_| OsError::InvalidArgument)?;

        // set kernel specific values that don't make sense to use from the bundle.
        proc.context.tpidr = 0; // will get a new process id when scheduled.
        proc.context.ttbr0 = VMM.get_baddr().as_u64();
        proc.context.ttbr1 = proc.vmap.get_baddr().as_u64();

        for (raw_va, data) in bundle.memory.generic_pages.iter() {
            let va = VirtualAddr::from(*raw_va);

            proc.vmap.add_region(Region::new(va, PAGE_SIZE, RegionKind::Normal));
            let page = proc.vmap.get_page_mut(va).expect("could not deref bad va");

            if page.len() != data.len() {
                return Err(OsError::BadAddress);
            }

            page.copy_from_slice(data.as_slice());
        }

        Ok(proc)
    }

    pub fn set_stdio(&mut self, source: Arc<Source>, sink: Arc<Sink>) {
        if self.file_descriptors.len() >= 1 {
            self.file_descriptors[0] = FileDescriptor::read(source);
        } else {
            self.file_descriptors.push(FileDescriptor::read(source));
        }

        if self.file_descriptors.len() >= 2 {
            self.file_descriptors[1] = FileDescriptor::write(sink);
        } else {
            self.file_descriptors.push(FileDescriptor::write(sink));
        }
    }

    fn memory_to_bundle(&self) -> io::Result<MemoryBundle> {
        // State does not impl PartialEq
        // assert_eq!(self.state, State::Suspended);

        let mut bundle = MemoryBundle::default();

        for (va, pa) in self.vmap.table.iter_mapped_pages() {
            let mut page_copy: Vec<u8> = Vec::with_capacity(PAGE_SIZE);
            page_copy.extend_from_slice(unsafe { core::slice::from_raw_parts(pa.as_ptr(), PAGE_SIZE) });
            bundle.generic_pages.insert(va.as_u64(), page_copy);
        }

        Ok(bundle)
    }

    pub fn to_bundle(&self) -> io::Result<ProcessBundle> {
        if let State::Suspended = self.state {
            let mut bundle = ProcessBundle::default();
            bundle.name.push_str(self.name.as_str());
            bundle.memory = self.memory_to_bundle()?;
            bundle.frame.extend_from_slice(self.context.as_bytes());

            Ok(bundle)
        } else {
            ioerr!(WouldBlock, "process not suspended")
        }
    }

    pub fn dump<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        writeln!(w, "Name: {}", self.name);

        writeln!(w, "Frame:");
        writeln!(w, "{:?}", self.context);

        writeln!(w, "Memory Regions:");
        for region in self.vmap.regions.iter() {
            writeln!(w, "  {:x?}", region);
        }

        writeln!(w, "Memory Mapping:");
        for (va, pa) in self.vmap.table.iter_mapped_pages() {
            writeln!(w, "  {:x?} -> {:x?}", va, pa);
        }

        Ok(())
    }

    /// Returns the highest `VirtualAddr` that is supported by this system.
    pub fn get_max_va() -> VirtualAddr {
        VirtualAddr::from(u64::max_value())
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// memory space.
    pub fn get_image_base() -> VirtualAddr {
        VirtualAddr::from(USER_IMG_BASE as u64)
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// process's stack.
    pub fn get_stack_base() -> VirtualAddr {
        VirtualAddr::from(u64::max_value() & PAGE_MASK as u64)
    }

    /// Returns the `VirtualAddr` represents the top of the user process's
    /// stack.
    pub fn get_stack_top() -> VirtualAddr {
        VirtualAddr::from(u64::max_value() & (!0xFu64))
    }

    pub fn has_request_kill(&self) -> bool {
        self.request_kill
    }

    pub fn request_kill(&mut self) {
        self.request_kill = true;
    }

    pub fn get_state(&self) -> &State {
        &self.state
    }

    pub fn set_state(&mut self, new_state: State) {
        let now = pi::timer::current_time();

        match &self.state {
            State::Ready => self.ready_ratio.set_active_with_time(false, now),
            State::Running(ctx) => {
                self.running_ratio.set_active_with_time(false, now);
                let delta = now - ctx.scheduled_at;
                self.cpu_time += delta;
                self.running_slices.record(delta);
            },
            State::Waiting(_) | State::WaitingObj(_) => self.waiting_ratio.set_active_with_time(false, now),
            _ => {},
        }

        match &new_state {
            State::Ready => self.ready_ratio.set_active_with_time(true, now),
            State::Running(_) => self.running_ratio.set_active_with_time(true, now),
            State::Waiting(_) | State::WaitingObj(_) => self.waiting_ratio.set_active_with_time(true, now),
            _ => {},
        }

        self.state = new_state;
    }


    /// Returns `true` if this process is ready to be scheduled.
    ///
    /// This functions returns `true` only if one of the following holds:
    ///
    ///   * The state is currently `Ready`.
    ///
    ///   * An event being waited for has arrived.
    ///
    ///     If the process is currently waiting, the corresponding event
    ///     function is polled to determine if the event being waiting for has
    ///     occured. If it has, the state is switched to `Ready` and this
    ///     function returns `true`.
    ///
    /// Returns `false` in all other cases.
    pub fn is_ready(&mut self) -> bool {
        if let State::Waiting(h) = &mut self.state {
            let mut copy = core::mem::replace(h, Box::new(|_| false));
            if copy(self) {
                self.set_state(State::Ready);
            } else {

                // this will always succeed. Cannot re-use h due to lifetimes of passing self
                // into copy()
                if let State::Waiting(h) = &mut self.state {
                    core::mem::replace(h, copy);
                }
            }
        }

        if let State::WaitingObj(obj) = &mut self.state {
            if obj.done_waiting() {
                self.set_state(State::Ready);
            }
        }

        // check ready and suspend last. This allows us to go from waiting to Suspended
        // in one tick via fallthrough.

        if let State::Suspended = &self.state {
            if !self.request_suspend {
                self.set_state(State::Ready);
            }
        }

        if let State::Ready = &self.state {
            if self.request_suspend {
                self.set_state(State::Suspended);
            }
        }

        match self.state {
            State::Ready => true,
            _ => false,
        }
    }
}
