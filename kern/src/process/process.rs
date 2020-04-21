use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::ops::Add;
use core::ops::Deref;
use core::time::Duration;
use alloc::sync::Arc;
use alloc::borrow::ToOwned;
use shim::path::Path;

use aarch64;

use crate::param::*;
use crate::process::{Stack, State, TimeRatio, TimeRing};
use crate::traps::TrapFrame;
use crate::vm::*;
use kernel_api::{OsError, OsResult};
use crate::process::state::EventPollFn;
use crate::FILESYSTEM;
use aarch64::vmsa::EntryPerm::USER_RW;
use aarch64::SPSR_EL1;

/// Type alias for the type of a process ID.
pub type Id = u64;

/// A structure that represents the complete state of a process.
pub struct Process {
    /// The saved trap frame of a process.
    pub context: Box<TrapFrame>,
    /// The memory allocation used for the process's stack.
    pub stack: Stack,
    /// The page table describing the Virtual Memory of the process
    pub vmap: Box<UserPageTable>,
    /// The scheduling state of the process.
    pub state: State,

    pub name: String,

    pub cpu_time: Duration,

    pub ready_ratio: TimeRatio,
    pub running_ratio: TimeRatio,
    pub waiting_ratio: TimeRatio,

    pub running_slices: TimeRing,

    pub task_switches: usize,
}

impl Process {
    /// Creates a new process with a zeroed `TrapFrame` (the default), a zeroed
    /// stack of the default size, and a state of `Ready`.
    ///
    /// If enough memory could not be allocated to start the process, returns
    /// `None`. Otherwise returns `Some` of the new `Process`.
    pub fn new(name: String) -> OsResult<Process> {
        let mut vmap = Box::new(UserPageTable::new());
        let stack = Stack::new().ok_or(OsError::NoMemory)?;
        let mut context = Box::new(TrapFrame::default());

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
            task_switches: 0,
        })
    }

    pub fn kernel_process_old(name: String, f: fn() -> !) -> OsResult<Process> {
        use crate::VMM;

        let mut p = Process::new(name)?;

        p.context.sp = p.stack.top().as_u64();
        p.context.elr = f as u64;

        p.context.spsr |= (SPSR_EL1::M & 0b0100);

        p.context.ttbr0 = VMM.get_baddr().as_u64();
        // kernel thread still gets a vmap because it's easy
        p.context.ttbr1 = p.vmap.get_baddr().as_u64();

        Ok(p)
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

        proc.vmap.alloc(Process::get_stack_base(), PagePerm::RW);

        let mut file = FILESYSTEM.open(pn)?.into_file().ok_or(OsError::InvalidArgument)?;

        let mut pages_required = file.size() / PAGE_SIZE as u64;
        if file.size() % PAGE_SIZE as u64 != 0 {
            pages_required += 1;
        }

        let mut base = Process::get_image_base();
        'page_loop: loop {
            let mut buf = proc.vmap.alloc(base, PagePerm::RWX);

            while buf.len() > 0 {
                let read_amount = file.read(buf)?;
                if read_amount == 0 {
                    break 'page_loop;
                }
                buf = &mut buf[read_amount..];
            }

            base = base.add(VirtualAddr::from(PAGE_SIZE));

        }

        Ok(proc)
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
            State::Waiting(_) => self.waiting_ratio.set_active_with_time(false, now),
            _ => {},
        }

        match &new_state {
            State::Ready => self.ready_ratio.set_active_with_time(true, now),
            State::Running(_) => self.running_ratio.set_active_with_time(true, now),
            State::Waiting(_) => self.waiting_ratio.set_active_with_time(true, now),
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

        match self.state {
            State::Ready => true,
            _ => false,
        }
    }
}
