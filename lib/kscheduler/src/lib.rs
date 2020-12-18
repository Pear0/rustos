#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::marker::PhantomPinned;
use core::pin::Pin;
use core::sync::atomic::AtomicUsize;

use hashbrown::HashMap;

use dsx::sync::mutex::LightMutex;

macro_rules! const_assert_size {
    ($expr:tt, $size:tt) => {
    const _: fn(a: $expr) -> [u8; $size] = |a| unsafe { core::mem::transmute::<$expr, [u8; $size]>(a) };
    };
}

pub trait SchedInfo : Sized {
    type Frame : Frame;

    type State;

    type Process : Process<Self::Frame, Self::State>;

    fn get_idle_task(&mut self) -> &mut Self::Process;

    fn running_state(&self) -> Self::State;

    fn dead_state(&self) -> Self::State;


    fn on_process_killed(&self, _proc: Self::Process) {
    }

}

pub trait Frame : Clone {
    fn get_id(&self) -> usize;
}

pub trait Process<F, S> {
    fn get_frame(&mut self) -> &mut F;

    fn set_id(&mut self, id: usize);

    fn get_id(&self) -> usize;

    fn set_state(&mut self, state: S);

    fn get_state(&self) -> &S;

    fn should_kill(&self) -> bool;

    fn get_priority(&self) -> usize;

    fn check_ready(&mut self) -> bool;

    fn affinity_match(&self) -> bool {
        true
    }

    fn on_task_switch(&mut self) {
    }

}

pub trait Scheduler<T: SchedInfo> {

    fn add_process(&mut self, proc: T::Process) -> Option<usize>;

    fn schedule_out(&mut self, state: T::State, tf: &mut T::Frame);

    fn schedule_in(&mut self, tf: &mut T::Frame) -> usize;

    fn kill(&mut self, tf: &mut T::Frame) -> Option<usize>;

    fn switch(&mut self, state: T::State, tf: &mut T::Frame) -> usize {
        self.schedule_out(state, tf);
        self.schedule_in(tf)
    }

    fn get_process_mut(&mut self, id: usize) -> Option<&mut T::Process>;

    fn iter_process_mut<F>(&mut self, func: F) where F: FnMut(&mut T::Process);

}

pub struct ListScheduler<T: SchedInfo> {
    pub info: T,
    processes: VecDeque<T::Process>,
    last_id: Option<usize>,
}

impl<T: SchedInfo> ListScheduler<T> {
    pub fn new(info: T) -> Self {
        Self {
            info,
            processes: VecDeque::new(),
            last_id: Some(1), // id zero is reserved for idle task
        }
    }

    fn next_id(&mut self) -> Option<usize> {
        let next = self.last_id?.checked_add(1)?;
        self.last_id = Some(next);
        Some(next)
    }

    fn schedule_idle_task(&mut self, tf: &mut T::Frame) -> usize {
        let state = self.info.running_state();
        let proc = self.info.get_idle_task();

        proc.set_state(state);
        *tf = proc.get_frame().clone();

        proc.get_id()
    }

    /// Finds the next process to switch to, brings the next process to the
    /// front of the `processes` queue, changes the next process's state to
    /// `Running`, and performs context switch by restoring the next process`s
    /// trap frame into `tf`.
    ///
    /// If there is no process to switch to, returns `None`. Otherwise, returns
    /// `Some` of the next process`s process ID.
    fn switch_to(&mut self, tf: &mut T::Frame) -> Option<usize> {

        // is_ready() is &mut so it doesn't work with .find() 😡😡😡
        let mut proc: Option<(usize, &mut T::Process)> = None;
        for entry in self.processes.iter_mut().enumerate() {
            if !entry.1.affinity_match() {
                continue;
            }

            // if our currently selected process has a better priority, keep it.
            if let Some((_, proc)) = &proc {
                if proc.get_priority() >= entry.1.get_priority() {
                    continue;
                }
            }

            if entry.1.check_ready() {
                proc = Some(entry);
            }
        }

        let (idx, proc) = proc?;

        proc.set_state(self.info.running_state());
        *tf = proc.get_frame().clone();

        let id = proc.get_id();

        // something is very bad if the entry we found is no longer here.
        let owned = self.processes.remove(idx).unwrap();
        self.processes.push_front(owned);

        // reset_timer();

        Some(id)
    }

}

impl<T: SchedInfo> Scheduler<T> for ListScheduler<T> {
    /// Adds a process to the scheduler's queue and returns that process's ID if
    /// a new process can be scheduled. The process ID is newly allocated for
    /// the process and saved in its `trap_frame`. If no further processes can
    /// be scheduled, returns `None`.
    ///
    /// It is the caller's responsibility to ensure that the first time `switch`
    /// is called, that process is executing on the CPU.
    fn add_process(&mut self, mut process: T::Process) -> Option<usize> {
        let id = self.next_id()?;
        process.set_id(id);
        self.processes.push_back(process);
        Some(id)
    }

    /// Finds the currently running process, sets the current process's state
    /// to `new_state`, prepares the context switch on `tf` by saving `tf`
    /// into the current process, and push the current process back to the
    /// end of `processes` queue.
    ///
    /// If the `processes` queue is empty or there is no current process,
    /// returns `false`. Otherwise, returns `true`.
    fn schedule_out(&mut self, mut new_state: T::State, tf: &mut T::Frame) {
        let proc: Option<(usize, &mut T::Process)>;
        if self.info.get_idle_task().get_id() == tf.get_id() {
            proc = Some((usize::max_value(), self.info.get_idle_task()));
        } else {
            proc = self.processes.iter_mut().enumerate()
                .find(|(_, p)| p.get_id() == tf.get_id());
        }

        match proc {
            None => {},
            Some((idx, proc)) => {
                if proc.should_kill() {
                    self.kill(tf);
                } else {
                    proc.on_task_switch();
                    // proc.task_switches += 1;
                    proc.set_state(new_state);
                    *proc.get_frame() = tf.clone();

                    // special processes like idle task aren't stored in self.processes
                    if idx != usize::max_value() {
                        // something is very bad if the entry we found is no longer here.
                        let owned = self.processes.remove(idx).expect("could not find process in self.processes");
                        self.processes.push_back(owned);
                    }
                }
            }
        }
    }

    fn schedule_in(&mut self, tf: &mut T::Frame) -> usize {
        if let Some(id) = self.switch_to(tf) {
            id
        } else {
            self.schedule_idle_task(tf)
        }
    }

    /// Kills currently running process by scheduling out the current process
    /// as `Dead` state. Removes the dead process from the queue, drop the
    /// dead process's instance, and returns the dead process's process ID.
    fn kill(&mut self, tf: &mut T::Frame) -> Option<usize> {
        let proc = self.processes.iter_mut().enumerate()
            .find(|(_, p)| p.get_id() == tf.get_id());
        match proc {
            None => None,
            Some((idx, proc)) => {
                proc.set_state(self.info.dead_state());
                *proc.get_frame() = tf.clone();

                // something is very bad if the entry we found is no longer here.
                let proc = self.processes.remove(idx).unwrap();

                let id = proc.get_id();

                self.info.on_process_killed(proc);

                Some(id)
            }
        }
    }

    fn get_process_mut(&mut self, id: usize) -> Option<&mut <T as SchedInfo>::Process> {
        for proc in self.processes.iter_mut() {
            if proc.get_id() == id {
                return Some(proc);
            }
        }

        None
    }

    fn iter_process_mut<F>(&mut self, mut func: F) where F: FnMut(&mut T::Process) {
        for proc in self.processes.iter_mut() {
            func(proc);
        }
    }
}













#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
