use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt;
use core::fmt::Debug;
use core::ops::Add;
use core::ops::Deref;
use core::time::Duration;

use kernel_api::{OsError, OsResult};

use aarch64;
use aarch64::SPSR_EL1;
use shim::{io, ioerr};
use shim::path::Path;

use crate::{smp, VMM};
use crate::fs::handle::{Sink, Source};
use crate::kernel::KERNEL_SCHEDULER;
use crate::param::*;
use crate::pigrate::bundle::{MemoryBundle, ProcessBundle};
use crate::process::{Stack, State, TimeRatio, TimeRing};
use crate::process::address_space::{AddressSpaceManager, Region, KernelRegionKind};
use crate::process::fd::FileDescriptor;
use crate::sync::Completion;
use crate::traps::{Frame, KernelTrapFrame};
use crate::vm::*;

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

#[repr(u8)]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Normal = 50,
    Highest = 100,
}


pub trait ProcessImpl: Sized + Send {
    type Frame: Frame + kscheduler::Frame + Default + Clone + Debug + Send;
    type RegionKind: Debug + Send;
    type PageTable: GuestPageTable;

    fn new() -> OsResult<Self>;

    fn create_idle_processes(count: usize) -> Vec<Process<Self>>;

    fn on_process_killed(proc: &mut Process<Self>) {}

    fn dump<W: io::Write>(w: &mut W, proc: &Process<Self>) {}
}

/// A structure that represents the complete state of a process.
pub struct Process<T: ProcessImpl> {
    /// The saved trap frame of a process.
    pub context: Box<T::Frame>,
    /// The memory allocation used for the process's stack.
    pub stack: Stack,
    /// The page table describing the Virtual Memory of the process
    pub vmap: Box<AddressSpaceManager<T>>,
    /// The scheduling state of the process.
    pub(crate) state: State<T>,

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

    pub priority: usize,

    pub detail: T,
}

impl<T: ProcessImpl> Process<T> {
    /// Creates a new process with a zeroed `TrapFrame` (the default), a zeroed
    /// stack of the default size, and a state of `Ready`.
    ///
    /// If enough memory could not be allocated to start the process, returns
    /// `None`. Otherwise returns `Some` of the new `Process`.
    pub fn new(name: String) -> OsResult<Self> {
        let vmap = Box::new(AddressSpaceManager::new());
        let stack = Stack::new().ok_or(OsError::NoMemory)?;
        let context = Box::new(T::Frame::default());

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
            priority: Priority::Normal as usize,
            detail: T::new()?,
        })
    }

    pub fn dump<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        writeln!(w, "Name: {}", self.name);

        writeln!(w, "Frame:");
        writeln!(w, "{:?}", self.context);

        writeln!(w, "Memory Regions:");
        for region in self.vmap.regions.iter() {
            writeln!(w, "  {:x?}", region);
        }

        // writeln!(w, "Memory Mapping:");
        // for (va, pa) in self.vmap.table.iter_mapped_pages() {
        //     writeln!(w, "  {:x?} -> {:x?}", va, pa);
        // }

        T::dump(w, self);

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

    pub fn get_state(&self) -> &State<T> {
        &self.state
    }

    pub fn set_state(&mut self, new_state: State<T>) {
        let now = crate::timing::clock_time_phys();

        match &self.state {
            State::Ready => self.ready_ratio.set_active_with_time(false, now),
            State::Running(ctx) => {
                self.running_ratio.set_active_with_time(false, now);
                let delta = now - ctx.scheduled_at;
                self.cpu_time += delta;
                self.running_slices.record(delta);
            }
            State::Waiting(_) | State::WaitingObj(_) => self.waiting_ratio.set_active_with_time(false, now),
            _ => {}
        }

        match &new_state {
            State::Ready => self.ready_ratio.set_active_with_time(true, now),
            State::Running(_) => self.running_ratio.set_active_with_time(true, now),
            State::Waiting(_) | State::WaitingObj(_) => self.waiting_ratio.set_active_with_time(true, now),
            _ => {}
        }

        self.state = new_state;
    }

    pub fn current_cpu_time(&self) -> Duration {
        let mut amt = self.cpu_time;
        if let State::Running(ctx) = &self.state {
            amt += (crate::timing::clock_time_phys() - ctx.scheduled_at);
        }
        amt
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

impl<T: ProcessImpl> kscheduler::Process<T::Frame, State<T>> for Process<T> {
    fn get_frame(&mut self) -> &mut T::Frame {
        &mut self.context
    }

    fn set_id(&mut self, id: usize) {
        self.context.set_id(id as u64);
    }

    fn get_id(&self) -> usize {
        self.context.get_id() as usize
    }

    fn set_state(&mut self, state: State<T>) {
        Process::<T>::set_state(self, state);
    }

    fn get_state(&self) -> &State<T> {
        Process::<T>::get_state(self)
    }

    fn should_kill(&self) -> bool {
        self.has_request_kill()
    }

    fn get_priority(&self) -> usize {
        self.priority
    }

    fn check_ready(&mut self) -> bool {
        self.is_ready()
    }

    fn affinity_match(&self) -> bool {
        self.affinity.check(smp::core())
    }

    fn affinity_valid_core(&self) -> Option<usize> {
        self.affinity.0.iter()
            .enumerate()
            .find(|(_, &b)| b)
            .map(|(idx, _)| idx)
    }

    fn on_task_switch(&mut self) {
        self.task_switches += 1;
    }
}

