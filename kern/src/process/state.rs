use alloc::boxed::Box;
use alloc::sync::Arc;
use core::fmt;

use crate::process::Process;
use core::time::Duration;
use crate::sync::Waitable;
use crate::process::process::ProcessImpl;

/// Type of a function used to determine if a process is ready to be scheduled
/// again. The scheduler calls this function when it is the process's turn to
/// execute. If the function returns `true`, the process is scheduled. If it
/// returns `false`, the process is not scheduled, and this function will be
/// called on the next time slice.
pub type EventPollFn<T> = Box<dyn FnMut(&mut Process<T>) -> bool + Send>;

#[derive(Clone, Default)]
pub struct RunContext {
    pub core_id: usize,
    pub scheduled_at: Duration,
}

/// The scheduling state of a process.
pub enum State<T: ProcessImpl> {
    /// The process is ready to be scheduled.
    Ready,
    /// The process is waiting on an event to occur before it can be scheduled.
    Waiting(EventPollFn<T>),
    /// The process is currently running.
    Running(RunContext),
    /// The process is currently dead (ready to be reclaimed).
    Dead,

    /// Waiting on a Waitable. This is explicit because it provides much more introspection
    /// than an arbitrary function.
    WaitingObj(Arc<dyn Waitable>),

    /// The process is suspended.
    Suspended,
}

impl<T: ProcessImpl> fmt::Debug for State<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            State::Ready => write!(f, "State::Ready"),
            State::Running(_) => write!(f, "State::Running"),
            State::Waiting(_) => write!(f, "State::Waiting"),
            State::Dead => write!(f, "State::Dead"),
            State::WaitingObj(_) => write!(f, "State::WaitingObj"),
            State::Suspended => write!(f, "State::Suspended"),
        }
    }
}
