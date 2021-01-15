use dsx::alloc::boxed::Box;
use dsx::alloc::collections::VecDeque;
use dsx::alloc::sync::Arc;
use dsx::alloc::vec::Vec;
use dsx::collections::spsc_queue::{SpscQueue, SpscQueueReader, SpscQueueWriter};
use dsx::core::cell::UnsafeCell;
use dsx::core::marker::PhantomData;
use dsx::core::ops::Deref;
use dsx::core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use dsx::sync::mutex::{LightMutex, LockableMutex};

use crate::{Process, SchedInfo, Scheduler};

enum Mail<T: SchedInfo> {
    Nil,
    AddProcess(ProcessInfo<T>),
}

struct ProcessInfo<T: SchedInfo> {
    process: Box<T::Process>,

    /// If a process is special, like the idle task, it is not killable.
    is_special: bool,
}

impl<T: SchedInfo> ProcessInfo<T> {}

struct Inner<T: SchedInfo> {
    pub info: T,
    mailboxes: Vec<LightMutex<SpscQueueWriter<Mail<T>>>>,
    last_id: AtomicUsize,
}

impl<T: SchedInfo> Inner<T> {
    pub(crate) fn create_arc(info: T, mailboxes: Vec<LightMutex<SpscQueueWriter<Mail<T>>>>) -> Arc<Self> {
        Arc::new(Self {
            info,
            mailboxes,
            last_id: AtomicUsize::new(1),
        })
    }

    pub fn next_process_id(&self) -> usize {
        self.last_id.fetch_add(1, Ordering::Relaxed) + 1
    }
}

struct CoreScheduler<T: SchedInfo> {
    core_id: usize,
    did_bootstrap: AtomicBool,
    inner: Arc<Inner<T>>,
    incoming_mailbox: SpscQueueReader<Mail<T>>,
    current_proc: Option<ProcessInfo<T>>,
    idle_proc: Option<ProcessInfo<T>>,
    run_queue: VecDeque<ProcessInfo<T>>,
    wait_queue: VecDeque<ProcessInfo<T>>,
    _phantom: PhantomData<T>,
}

impl<T: SchedInfo> CoreScheduler<T> {
    pub(crate) fn create(core_id: usize, inner: Arc<Inner<T>>,
                         mailbox: SpscQueueReader<Mail<T>>) -> Self {
        Self {
            core_id,
            did_bootstrap: AtomicBool::new(false),
            inner,
            incoming_mailbox: mailbox,
            current_proc: None,
            idle_proc: None,
            run_queue: VecDeque::new(),
            wait_queue: VecDeque::new(),
            _phantom: PhantomData,
        }
    }

    fn process_mail(&mut self) {
        while let Some(mail) = self.incoming_mailbox.try_dequeue() {
            match mail {
                Mail::AddProcess(mut proc) => {
                    if let Some(id) = proc.process.get_send_to_core() {
                        assert_eq!(self.core_id, id);
                        proc.process.set_send_to_core(None);
                    }

                    if proc.process.check_ready() {
                        self.run_queue.push_back(proc);
                    } else {
                        self.wait_queue.push_back(proc);
                    }
                }
                Mail::Nil => {}
            }
        }
    }

    fn check_waiting_processes(&mut self) {
        for _ in 0..self.wait_queue.len() {
            let mut proc = self.wait_queue.pop_front().unwrap();
            if proc.process.check_ready() {
                self.run_queue.push_back(proc);
            } else {
                self.wait_queue.push_back(proc);
            }
        }
    }

    /// Return Some(core_id) process was sent to if moved.
    /// Return None if process was not moved, either because the chosen
    /// valid core is this core or there are no valid cores.
    fn affinity_mismatch(&mut self, mut proc: ProcessInfo<T>) -> Option<usize> {
        if let Some(dest) = proc.process.affinity_valid_core() {
            proc.process.set_send_to_core(Some(dest));
            return self.send_to_core(proc);
        }

        self.wait_queue.push_back(proc);
        None
    }

    fn send_to_core(&mut self, mut proc: ProcessInfo<T>) -> Option<usize> {
        if let Some(dest) = proc.process.get_send_to_core() {
            if dest != self.core_id {
                let mut mailbox = self.inner.mailboxes[dest].lock();
                match mailbox.try_enqueue(Mail::AddProcess(proc)) {
                    Ok(()) => return Some(dest),
                    Err(Mail::AddProcess(p)) => {
                        // failed to enqueue, put the process back for later code to use.
                        proc = p;
                    }
                    Err(_) => unreachable!("returned object will always be AddProcess()"),
                }
            } else {
                proc.process.set_send_to_core(None);
            }
        }

        self.run_queue.push_back(proc);
        None
    }

    fn schedule_idle_task(&mut self, tf: &mut T::Frame) -> usize {
        let state = self.inner.info.running_state();
        let mut proc = self.idle_proc.take().unwrap();

        proc.process.set_state(state);
        *tf = proc.process.get_frame().clone();

        let id = proc.process.get_id();

        assert!(self.current_proc.is_none());
        self.current_proc.replace(proc);

        id
    }

    fn switch_to(&mut self, tf: &mut T::Frame) -> Option<usize> {
        while let Some(mut proc) = self.run_queue.pop_front() {
            if matches!(proc.process.get_send_to_core(), Some(_)) {
                self.send_to_core(proc);
                continue;
            }

            if !proc.process.affinity_match() {
                self.affinity_mismatch(proc);
                continue;
            }

            if !proc.process.check_ready() {
                self.wait_queue.push_back(proc);
                continue;
            }

            // assign process...

            proc.process.set_state(self.inner.info.running_state());
            *tf = proc.process.get_frame().clone();

            let id = proc.process.get_id();

            assert!(self.current_proc.is_none());
            self.current_proc.replace(proc);

            return Some(id);
        }

        None
    }
}

impl<T: SchedInfo> Scheduler<T> for CoreScheduler<T> {
    fn add_process(&mut self, proc: T::Process) -> Option<usize> {
        let id = self.inner.next_process_id();
        let mut proc = ProcessInfo::<T> {
            process: Box::new(proc),
            is_special: false,
        };
        proc.process.set_id(id);

        if proc.process.check_ready() {
            self.run_queue.push_back(proc);
        } else {
            self.wait_queue.push_back(proc);
        }

        Some(id)
    }

    fn schedule_out(&mut self, state: T::State, tf: &mut T::Frame) {
        let mut proc = self.current_proc.take().expect("current_proc must be scheduled for schedule_out()");
        if !proc.is_special && proc.process.should_kill() {
            self.kill(tf);
            return;
        }

        proc.process.on_task_switch();
        proc.process.set_state(state);
        *proc.process.get_frame() = tf.clone();

        // special processes like idle task aren't stored in self.processes
        if proc.is_special {
            let old = self.idle_proc.replace(proc);
            assert!(old.is_none());
        } else if proc.process.check_ready() {
            self.run_queue.push_back(proc);
        } else {
            self.wait_queue.push_back(proc);
        }
    }

    fn schedule_in(&mut self, tf: &mut T::Frame) -> usize {
        assert!(self.current_proc.is_none());
        if let Some(id) = self.switch_to(tf) {
            id
        } else {
            self.schedule_idle_task(tf)
        }
    }

    fn kill(&mut self, tf: &mut T::Frame) -> Option<usize> {
        let mut proc = self.current_proc.take().expect("current_proc must be scheduled for schedule_out()");
        if proc.is_special {
            return None;
        }

        proc.process.set_state(self.inner.info.dead_state());
        *proc.process.get_frame() = tf.clone();

        let id = proc.process.get_id();

        self.inner.info.on_process_killed(*proc.process);

        Some(id)
    }

    fn switch(&mut self, state: T::State, tf: &mut T::Frame) -> usize {
        self.process_mail();
        self.check_waiting_processes();

        self.schedule_out(state, tf);
        self.schedule_in(tf)
    }

    fn with_process_mut<R, F>(&mut self, id: usize, func: F) -> R
        where F: FnOnce(Option<&mut T::Process>) -> R
    {
        if let Some(proc) = &mut self.current_proc {
            if proc.process.get_id() == id {
                return func(Some(&mut proc.process));
            }
        }

        if let Some(proc) = &mut self.idle_proc {
            if proc.process.get_id() == id {
                return func(Some(&mut proc.process));
            }
        }

        for proc in self.run_queue.iter_mut() {
            if proc.process.get_id() == id {
                return func(Some(&mut proc.process));
            }
        }

        for proc in self.wait_queue.iter_mut() {
            if proc.process.get_id() == id {
                return func(Some(&mut proc.process));
            }
        }

        func(None)
    }

    fn iter_process_mut<F>(&mut self, mut func: F) where F: FnMut(&mut T::Process) {
        if let Some(proc) = &mut self.current_proc {
            func(&mut proc.process);
        }

        if let Some(proc) = &mut self.idle_proc {
            func(&mut proc.process);
        }

        for proc in self.run_queue.iter_mut() {
            func(&mut proc.process);
        }

        for proc in self.wait_queue.iter_mut() {
            func(&mut proc.process);
        }
    }

    fn initialize_core(&mut self) {
        assert!(!self.did_bootstrap.compare_and_swap(false, true, Ordering::Relaxed));

        // take ownership of the idle process...
        {
            let mut lock = self.inner.info.get_idle_task().lock();
            let process = lock.take().unwrap();
            self.idle_proc.replace(ProcessInfo {
                process: Box::new(process),
                is_special: true,
            });
        }
    }
}

pub struct WaitQueueScheduler<T: SchedInfo> {
    inner: Arc<Inner<T>>,
    cores: Vec<UnsafeCell<CoreScheduler<T>>>,
}

impl<T: SchedInfo> WaitQueueScheduler<T> {
    pub fn new(info: T, num_cores: usize) -> Self {
        let mut q_reader = Vec::with_capacity(num_cores);
        let mut q_writer = Vec::with_capacity(num_cores);

        for _ in 0..num_cores {
            let (reader, writer) = SpscQueue::<Mail<T>>::new(128);
            q_reader.push(reader);
            q_writer.push(LightMutex::new(writer));
        }

        let inner = Inner::<T>::create_arc(info, q_writer);

        let cores: Vec<_> = q_reader.into_iter()
            .enumerate()
            .map(|(core_id, mailbox)| {
                UnsafeCell::new(CoreScheduler::<T>::create(core_id, inner.clone(), mailbox))
            })
            .collect();

        Self {
            inner,
            cores,
        }
    }

    unsafe fn current_core(&self) -> &mut CoreScheduler<T> {
        let core_id = self.inner.info.current_core();
        &mut *self.cores[core_id].get()
    }
}

impl<T: SchedInfo> Scheduler<T> for &WaitQueueScheduler<T> {
    fn add_process(&mut self, proc: T::Process) -> Option<usize> {
        let id = self.inner.next_process_id();
        let mut proc = ProcessInfo::<T> {
            process: Box::new(proc),
            is_special: false,
        };
        proc.process.set_id(id);

        let core_id = self.inner.info.current_core();
        let mut mailbox = self.inner.mailboxes[core_id].lock();

        match mailbox.try_enqueue(Mail::AddProcess(proc)) {
            Ok(()) => Some(id),
            Err(_) => None,
        }
    }

    fn schedule_out(&mut self, state: T::State, tf: &mut T::Frame) {
        unsafe { self.current_core().schedule_out(state, tf) }
    }

    fn schedule_in(&mut self, tf: &mut T::Frame) -> usize {
        unsafe { self.current_core().schedule_in(tf) }
    }

    fn kill(&mut self, tf: &mut T::Frame) -> Option<usize> {
        unsafe { self.current_core().kill(tf) }
    }

    fn switch(&mut self, state: T::State, tf: &mut T::Frame) -> usize {
        unsafe { self.current_core().switch(state, tf) }
    }

    fn with_process_mut<R, F>(&mut self, id: usize, func: F) -> R
        where F: FnOnce(Option<&mut T::Process>) -> R {
        unsafe { self.current_core().with_process_mut(id, func) }
    }

    fn iter_process_mut<F>(&mut self, func: F) where F: FnMut(&mut T::Process) {
        unsafe { self.current_core().iter_process_mut(func) }
    }

    fn initialize_core(&mut self) {
        unsafe { self.current_core().initialize_core() }
    }
}

impl<T: SchedInfo> Scheduler<T> for WaitQueueScheduler<T> {
    fn add_process(&mut self, proc: T::Process) -> Option<usize> {
        <&WaitQueueScheduler<T>>::add_process(&mut &*self, proc)
    }

    fn schedule_out(&mut self, state: T::State, tf: &mut T::Frame) {
        <&WaitQueueScheduler<T>>::schedule_out(&mut &*self, state, tf)
    }

    fn schedule_in(&mut self, tf: &mut T::Frame) -> usize {
        <&WaitQueueScheduler<T>>::schedule_in(&mut &*self, tf)
    }

    fn kill(&mut self, tf: &mut T::Frame) -> Option<usize> {
        <&WaitQueueScheduler<T>>::kill(&mut &*self, tf)
    }

    fn switch(&mut self, state: T::State, tf: &mut T::Frame) -> usize {
        <&WaitQueueScheduler<T>>::switch(&mut &*self, state, tf)
    }

    fn with_process_mut<R, F>(&mut self, id: usize, func: F) -> R
        where F: FnOnce(Option<&mut T::Process>) -> R {
        <&WaitQueueScheduler<T>>::with_process_mut(&mut &*self, id, func)
    }

    fn iter_process_mut<F>(&mut self, func: F) where F: FnMut(&mut T::Process) {
        <&WaitQueueScheduler<T>>::iter_process_mut(&mut &*self, func)
    }

    fn initialize_core(&mut self) {
        <&WaitQueueScheduler<T>>::initialize_core(&mut &*self)
    }
}


