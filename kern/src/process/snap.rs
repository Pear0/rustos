use crate::process::{State, Process};

#[derive(Debug, Clone, Copy)]
pub enum SnapState {
    /// The process is ready to be scheduled.
    Ready,
    /// The process is waiting on an event to occur before it can be scheduled.
    Waiting,
    /// The process is currently running.
    Running,
    /// The process is currently dead (ready to be reclaimed).
    Dead,
}

impl From<&State> for SnapState {
    fn from(state: &State) -> Self {
        use SnapState::*;
        match state {
            State::Ready => Ready,
            State::Waiting(_) => Waiting,
            State::Running => Running,
            State::Dead => Dead,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnapProcess {
    pub tpidr: u64,
    pub state: SnapState,
}

impl From<&Process> for SnapProcess {
    fn from(proc: &Process) -> Self {
        SnapProcess {
            tpidr: proc.context.tpidr,
            state: (&proc.state).into(),
        }
    }
}