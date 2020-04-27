use alloc::boxed::Box;
use core::time::Duration;

mod address_space;
pub mod fd;
mod kernel;
mod mailbox;
mod process;
mod scheduler;
mod snap;
mod stack;
mod state;

pub use crate::param::TICK;

pub use self::kernel::*;
pub use self::process::{Id, Process, ProcessImpl};
pub use self::scheduler::GlobalScheduler;
pub use self::stack::Stack;
pub use self::state::{EventPollFn, State};

#[derive(Clone, Debug)]
pub struct TimeRatio {
    window: Duration,
    captured: Duration,
    active: Duration,

    // used to dynamically create measurements
    last_update: Duration, // absolute
    is_active: bool,
}

impl TimeRatio {
    pub const RESOLUTION: u32 = 1000;
    const INTERNAL_RES: u32 = 10_000_000;

    fn now() -> Duration {
        pi::timer::current_time()
    }

    pub fn new_window(window: Duration) -> Self {
        Self {
            window,
            captured: Duration::default(),
            active: Duration::default(),
            last_update: Self::now(),
            is_active: false,
        }
    }

    pub fn new() -> Self {
        Self::new_window(Duration::from_secs(2))
    }

    pub fn measure(&mut self, mut delta: Duration, active: bool) {
        if self.captured < self.window {
            let fill = core::cmp::min(delta, self.window - self.captured);
            self.captured += fill;
            if active {
                self.active += fill;
            }
            delta -= fill;
        }

        if delta > self.window {
            // simple case, no blending
            self.active = if active { self.window } else { Duration::default() };
        } else if delta > Duration::default() {
            // since delta is less than window, ticks < RESOLUTION
            let ticks = (delta.as_micros() * (Self::INTERNAL_RES as u128) / self.window.as_micros()) as u32;

            let mut total = self.active * (Self::INTERNAL_RES - ticks);
            if active {
                total += self.window * ticks;
            }
            self.active = total / Self::INTERNAL_RES;

        }
    }

    /// Allows imperative usage. This calls measure for the previous interval,
    /// so methods like get_average() do not account for the current ongoing measurement.
    pub fn set_active_with_time(&mut self, active: bool, now: Duration) {
        let delta = now - self.last_update;
        self.last_update = now;
        self.measure(delta, self.is_active);
        self.is_active = active;
    }

    pub fn set_active(&mut self, active: bool) {
        self.set_active_with_time(active, Self::now());
    }

    /// Measured in ticks of RESOLUTION of the time window
    pub fn get_average(&self) -> u32 {
        if self.captured.as_micros() != 0 {
            (self.active.as_micros() * (Self::RESOLUTION as u128) / self.captured.as_micros()) as u32
        } else {
            0
        }
    }
}

pub struct TimeRing {
    items: Box<[Duration; TimeRing::RING_SIZE]>,
    ptr: usize,
    len: usize,
}

impl TimeRing {
    const RING_SIZE: usize = 256;

    pub fn new() -> Self {
        Self {
            items: Box::new([Duration::default(); TimeRing::RING_SIZE]),
            ptr: 0,
            len: 0,
        }
    }

    pub fn record(&mut self, val: Duration) {
        self.items[self.ptr] = val;
        self.len = self.ptr + 1;
        self.ptr = (self.ptr + 1) % Self::RING_SIZE;
    }

    pub fn average(&self) -> Duration {
        if self.len == 0 {
            return Duration::default();
        }
        let mut total = Duration::default();
        for i in 0..self.len {
            total += self.items[i];
        }
        total / (self.len as u32)
    }

}
