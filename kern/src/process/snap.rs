use crate::process::{Process, State};
use alloc::string::String;
use core::time::Duration;
use crate::process::process::CoreAffinity;
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

    Suspended,
}

impl From<&State> for SnapState {
    fn from(state: &State) -> Self {
        use SnapState::*;
        match state {
            State::Ready => Ready,
            State::Waiting(_) => Waiting,
            State::Running(ctx) => Running(ctx.core_id),
            State::Dead => Dead,
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
    pub task_switches: usize,
    pub affinity: CoreAffinity,
    pub lr: u64,
}

impl From<&Process> for SnapProcess {
    fn from(proc: &Process) -> Self {

        let mut cpu_time = proc.cpu_time;
        if let State::Running(ctx) = &proc.state {
            cpu_time = pi::timer::current_time() - ctx.scheduled_at;
        }

        SnapProcess {
            tpidr: proc.context.tpidr,
            state: (&proc.state).into(),
            name: proc.name.clone(),
            stack_top: proc.stack.top().as_u64(),
            cpu_time,
            task_switches: proc.task_switches,
            affinity: proc.affinity,
            lr: proc.context.elr,
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
            .field("task_switches", &self.task_switches)
            .field("affinity", &self.affinity)
            .field("lr", &format_args!("0x{:x}", self.lr))
            .finish()
    }
}