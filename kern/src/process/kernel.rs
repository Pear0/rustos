use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use kernel_api::{OsError, OsResult};

use aarch64::SPSR_EL1;
use pigrate_core::bundle::MemoryBundle;
use pigrate_core::bundle::ProcessBundle;
use shim::io;
use shim::path::Path;

use crate::{FILESYSTEM, VMM};
use crate::fs::handle::{Sink, Source};
use crate::kernel::KERNEL_SCHEDULER;
use crate::param::{PAGE_SIZE, USER_IMG_BASE};
use crate::process::{Id, Process, ProcessImpl, State};
use crate::process::address_space::{KernelRegionKind, Region};
use crate::process::fd::FileDescriptor;
use crate::sync::Completion;
use crate::traps::{Frame, KernelTrapFrame};

use crate::vm::{VirtualAddr, UserPageTable};
use alloc::format;

pub struct KernProcessCtx {
    pub pid: Id,
}

impl KernProcessCtx {
    /// Will panic if file descriptors are not assigned.
    pub fn get_stdio_or_panic(&self) -> (Arc<Source>, Arc<Sink>) {
        KERNEL_SCHEDULER.crit_process(self.pid, |f| {
            let f = f.unwrap();
            (f.detail.file_descriptors[0].read.as_ref().unwrap().clone(), f.detail.file_descriptors[1].write.as_ref().unwrap().clone())
        })
    }
}

type KernProcess = Box<dyn FnOnce(KernProcessCtx) + Send>;


pub struct KernelImpl {
    pub file_descriptors: Vec<FileDescriptor>,

    pub dead_completions: Vec<Arc<Completion<Id>>>,

    kernel_proc_entry: Option<KernProcess>,
}

impl ProcessImpl for KernelImpl {
    type Frame = KernelTrapFrame;
    type RegionKind = KernelRegionKind;
    type PageTable = UserPageTable;

    fn new() -> OsResult<Self> {
        Ok(Self {
            file_descriptors: Vec::new(),
            dead_completions: Vec::new(),
            kernel_proc_entry: None,
        })
    }

    fn create_idle_processes(count: usize) -> Vec<Process<Self>> {
        let mut idle_tasks = Vec::new();
        idle_tasks.reserve_exact(count);
        for i in 0..count {
            let name = format!("idle_task{}", i);
            let proc = Process::<Self>::kernel_process_old(name, || {
                loop {
                    aarch64::wfe();
                    // trigger context switch immediately after WFE so we dont take a full
                    // scheduler slice.
                    kernel_api::syscall::sched_yield();
                }
            }).expect("failed to create idle task");
            idle_tasks.push(proc);
        }
        idle_tasks
    }

    fn on_process_killed(proc: &mut Process<Self>) {
        for comp in proc.detail.dead_completions.drain(..) {
            comp.complete(proc.context.get_id());
        }
    }
}

pub type KernelProcess = Process<KernelImpl>;

impl Process<KernelImpl> {
    pub fn kernel_process_old(name: String, f: fn() -> !) -> OsResult<Self> {
        use crate::VMM;

        let mut p = Self::new(name)?;

        p.context.SP_EL0 = p.stack.top().as_u64();
        p.context.ELR_EL1 = f as u64;

        p.context.SPSR_EL1 |= SPSR_EL1::M & 0b0100;

        p.context.TTBR0_EL1 = VMM.get_baddr().as_u64();
        // kernel thread still gets a vmap because it's easy
        p.context.TTBR1_EL1 = p.vmap.get_baddr().as_u64();

        Ok(p)
    }

    fn kernel_process_bootstrap() -> ! {
        let pid: Id = kernel_api::syscall::getpid();

        let entry = KERNEL_SCHEDULER.crit_process(pid, |proc| {
            proc.map(|proc| proc.detail.kernel_proc_entry.take())
        });

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

    pub fn kernel_process_boxed(name: String, f: KernProcess) -> OsResult<Self> {
        let mut proc = Self::kernel_process_old(name, Self::kernel_process_bootstrap)?;
        proc.detail.kernel_proc_entry = Some(f);
        Ok(proc)
    }

    pub fn kernel_process<F: FnOnce(KernProcessCtx) + Send + 'static>(name: String, f: F) -> OsResult<Self> {
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
    pub fn load<P: AsRef<Path>>(pn: P) -> OsResult<Self> {
        use crate::VMM;

        let mut p = Self::do_load(pn)?;

        p.context.SP_EL0 = Self::get_stack_top().as_u64();
        p.context.ELR_EL1 = USER_IMG_BASE as u64;

        p.context.TTBR0_EL1 = VMM.get_baddr().as_u64();
        p.context.TTBR1_EL1 = p.vmap.get_baddr().as_u64();

        Ok(p)
    }

    /// Creates a process and open a file with given path.
    /// Allocates one page for stack with read/write permission, and N pages with read/write/execute
    /// permission to load file's contents.
    fn do_load<P: AsRef<Path>>(pn: P) -> OsResult<Self> {
        use fat32::traits::*;
        use shim::io::Read;
        let mut proc = Self::new(pn.as_ref().to_str().ok_or(OsError::InvalidArgument)?.to_owned())?;

        proc.vmap.add_region(Region::new(Self::get_stack_base(), PAGE_SIZE, KernelRegionKind::Normal));

        let image_base = Self::get_image_base();

        let mut file = FILESYSTEM.open(pn)?.into_file().ok_or(OsError::InvalidArgument)?;

        let mut base = image_base;
        'page_loop: loop {
            if image_base == base {
                proc.vmap.add_region(Region::new(image_base, PAGE_SIZE, KernelRegionKind::Normal));
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

    pub fn from_bundle(bundle: &ProcessBundle) -> OsResult<Self> {
        let mut proc = Self::new(bundle.name.clone())?;

        proc.context.decode_from_bytes(&bundle.frame).map_err(|_| OsError::InvalidArgument)?;

        // set kernel specific values that don't make sense to use from the bundle.
        proc.context.TPIDR_EL0 = 0; // will get a new process id when scheduled.
        proc.context.TTBR0_EL1 = VMM.get_baddr().as_u64();
        proc.context.TTBR1_EL1 = proc.vmap.get_baddr().as_u64();

        for (raw_va, data) in bundle.memory.generic_pages.iter() {
            let va = VirtualAddr::from(*raw_va);

            proc.vmap.add_region(Region::new(va, PAGE_SIZE, KernelRegionKind::Normal));
            let page = proc.vmap.get_page_mut(va).expect("could not deref bad va");

            if page.len() != data.len() {
                return Err(OsError::BadAddress);
            }

            page.copy_from_slice(data.as_slice());
        }

        Ok(proc)
    }

    pub fn set_stdio(&mut self, source: Arc<Source>, sink: Arc<Sink>) {
        if self.detail.file_descriptors.len() >= 1 {
            self.detail.file_descriptors[0] = FileDescriptor::read(source);
        } else {
            self.detail.file_descriptors.push(FileDescriptor::read(source));
        }

        if self.detail.file_descriptors.len() >= 2 {
            self.detail.file_descriptors[1] = FileDescriptor::write(sink);
        } else {
            self.detail.file_descriptors.push(FileDescriptor::write(sink));
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
}

