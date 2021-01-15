use crate::process::{Process, State, KernelProcess, HyperProcess};
use alloc::string::String;
use core::time::Duration;
use crate::process::process::{CoreAffinity, ProcessImpl};
use core::fmt;

#[derive(Debug, Clone, Copy)]
pub enum SnapState {
    /// The process is ready to be scheduled.
    Ready,
    /// The process is waiting on an event to occur before it can be scheduled.
    Waiting,
    /// The process is currently running.
    Running(usize),
    /// The process is currently dead (ready to be reclaimed).
    Dead,

    WaitingObj,

    Suspended,
}

impl<T: ProcessImpl> From<&State<T>> for SnapState {
    fn from(state: &State<T>) -> Self {
        use SnapState::*;
        match state {
            State::Ready => Ready,
            State::Waiting(_) => Waiting,
            State::Running(ctx) => Running(ctx.core_id),
            State::Dead => Dead,
            State::WaitingObj(_) => WaitingObj,
            State::Suspended => Suspended
        }
    }
}

#[derive(Clone)]
pub struct SnapProcess {
    pub tpidr: u64,
    pub state: SnapState,
    pub name: String,
    pub stack_top: u64,
    pub cpu_time: Duration,
    pub cpu_usage: u32,
    pub waiting_usage: u32,
    pub ready_usage: u32,
    pub avg_run_slice: Duration,
    pub task_switches: usize,
    pub affinity: CoreAffinity,
    pub lr: u64,
    pub core: isize,
}

impl From<&KernelProcess> for SnapProcess {
    fn from(proc: &KernelProcess) -> Self {

        let mut cpu_time = proc.cpu_time;
        if let State::Running(ctx) = proc.get_state() {
            cpu_time += crate::timing::clock_time_phys() - ctx.scheduled_at;
        }

        SnapProcess {
            tpidr: proc.context.TPIDR_EL0,
            state: proc.get_state().into(),
            name: proc.name.clone(),
            stack_top: proc.stack.top().as_u64(),
            cpu_time,
            cpu_usage: proc.running_ratio.get_average(),
            waiting_usage: proc.waiting_ratio.get_average(),
            ready_usage: proc.ready_ratio.get_average(),
            avg_run_slice: proc.running_slices.average(),
            task_switches: proc.task_switches,
            affinity: proc.affinity,
            lr: proc.context.ELR_EL1,
            core: -1,
        }
    }
}

impl From<&HyperProcess> for SnapProcess {
    fn from(proc: &HyperProcess) -> Self {

        let mut cpu_time = proc.cpu_time;
        if let State::Running(ctx) = proc.get_state() {
            cpu_time += crate::timing::clock_time_phys() - ctx.scheduled_at;
        }

        SnapProcess {
            tpidr: proc.context.TPIDR_EL2,
            state: proc.get_state().into(),
            name: proc.name.clone(),
            stack_top: proc.stack.top().as_u64(),
            cpu_time,
            cpu_usage: proc.running_ratio.get_average(),
            waiting_usage: proc.waiting_ratio.get_average(),
            ready_usage: proc.ready_ratio.get_average(),
            avg_run_slice: proc.running_slices.average(),
            task_switches: proc.task_switches,
            affinity: proc.affinity,
            lr: proc.context.ELR_EL2,
            core: -1,
        }
    }
}

impl fmt::Debug for SnapProcess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SnapProcess")
            .field("tpidr", &self.tpidr)
            .field("state", &self.state)
            .field("name", &self.name)
            .field("stack_top", &format_args!("0x{:x}", self.stack_top))
            .field("cpu_time", &self.cpu_time)
            .field("cpu_usage", &format_args!("{}.{}%", self.cpu_usage / 10, self.cpu_usage % 10)) // TODO assumes resolution 1000
            .field("waiting_usage", &format_args!("{}.{}%", self.waiting_usage / 10, self.waiting_usage % 10)) // TODO assumes resolution 1000
            .field("ready_usage", &format_args!("{}.{}%", self.ready_usage / 10, self.ready_usage % 10)) // TODO assumes resolution 1000
            .field("avg_run_slice", &self.avg_run_slice)
            .field("task_switches", &self.task_switches)
            .field("affinity", &self.affinity)
            .field("lr", &format_args!("0x{:x}", self.lr))
            .field("core", &self.core)
            .finish()
    }
}