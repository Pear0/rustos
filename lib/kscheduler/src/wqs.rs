use hashbrown::HashMap;

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

pub struct WakeRequest<T: SchedInfo> {
    pub core_id: usize,
    pub proc_id: usize,

    /// If not None, execute on the Process and wake only if func returns true.
    pub func: Option<Box<dyn FnOnce(&mut T::Process) -> bool + Send>>,
}


enum Mail<T: SchedInfo> {
    #[allow(dead_code)] Nil,
    AddProcess(ProcessInfo<T>),

    /// Request a process be woken up with a WakeRequest.
    WakeRequest(WakeRequest<T>),

    /// When received, a core will check all of its waiting processes for any that
    /// are now ready. WakeRequest should be preferred over WakeAllRequest when the
    /// waiting processes are known.
    ///
    /// A wait handle implementation may remember up to 4 waiting processes for WakeRequests
    /// then switch to WakeAllRequest if more than 4 processes attempt to wait on the handle.
    WakeAllRequest,
}

/// This enum represents a process in the waiting queue.
enum WaitingEntry<T: SchedInfo> {
    /// The normal process state.
    Process(ProcessInfo<T>),

    /// The Tombstone state is used when a process will be removed from the queue.
    /// It should only be used when an entry is about to be deleted such as when using
    /// HashMap::retain().
    Tombstone,
}


#[allow(dead_code)]
impl<T: SchedInfo> WaitingEntry<T> {
    pub fn as_process(&self) -> &ProcessInfo<T> {
        match self {
            Self::Process(proc) => proc,
            Self::Tombstone => panic!("cannot access tombstoned entry"),
        }
    }

    pub fn as_process_mut(&mut self) -> &mut ProcessInfo<T> {
        match self {
            Self::Process(proc) => proc,
            Self::Tombstone => panic!("cannot access tombstoned entry"),
        }
    }

    pub fn into_process(self) -> ProcessInfo<T> {
        match self {
            Self::Process(proc) => proc,
            Self::Tombstone => panic!("cannot access tombstoned entry"),
        }
    }
}

struct ProcessInfo<T: SchedInfo> {
    process: Box<T::Process>,

    /// If a process is an idle task, is not killable.
    is_idle_task: bool,
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
    wait_queue: HashMap<usize, WaitingEntry<T>>,
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
            wait_queue: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    fn add_to_wait_queue(&mut self, proc: ProcessInfo<T>) {
        assert!(!proc.is_idle_task);
        let id = proc.process.get_id();
        assert!(id > 0);

        if let Some(_) = self.wait_queue.insert(id, WaitingEntry::Process(proc)) {
            panic!("attempted to insert two processes with pid {} into wait queue", id);
        }
    }

    fn wake_own_process(&mut self, mut req: WakeRequest<T>) {
        assert_eq!(req.core_id, self.core_id);

        if let Some(entry) = self.wait_queue.get_mut(&req.proc_id) {
            let proc = &mut *entry.as_process_mut().process;

            let do_wake = req.func.take()
                .map(|func| func(proc)) // apply function
                .unwrap_or(true); // if no function, always wake.

            if do_wake {
                let mut proc = self.wait_queue.remove(&req.proc_id).unwrap().into_process();

                assert!(proc.process.check_ready());

                self.run_queue.push_back(proc);
            }
        }
    }

    fn wake_process(&mut self, req: WakeRequest<T>) {
        if self.core_id == req.core_id {
            self.wake_own_process(req);
        } else {
            let core_id = req.core_id;
            let mut queue = self.inner.mailboxes[core_id].lock();
            if let Err(_) = queue.try_enqueue(Mail::WakeRequest(req)) {
                panic!("failed to send WakeRequest(_) to core {}", core_id);
            }
        }
    }

    fn broadcast_wake_all_processes(&mut self) {
        self.check_waiting_processes();

        let core_id = self.core_id;
        for (i, mailbox) in self.inner.mailboxes.iter().enumerate() {
            if core_id != i {
                let mut queue = mailbox.lock();
                if let Err(_) = queue.try_enqueue(Mail::WakeAllRequest) {
                    panic!("failed to send WakeAllRequest to core {}", core_id);
                }
            }
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
                        self.add_to_wait_queue(proc);
                    }
                }
                Mail::WakeRequest(req) => {
                    self.wake_own_process(req);
                }
                Mail::WakeAllRequest => {
                    self.check_waiting_processes();
                }
                Mail::Nil => {}
            }
        }
    }

    fn check_waiting_processes(&mut self) {
        let Self { wait_queue, run_queue, .. } = self;

        wait_queue.retain(|_, proc| {
            if proc.as_process_mut().process.check_ready() {
                run_queue.push_back(core::mem::replace(proc, WaitingEntry::Tombstone).into_process());
                false // remove
            } else {
                true // keep
            }
        });
    }

    /// Return Some(core_id) process was sent to if moved.
    /// Return None if process was not moved, either because the chosen
    /// valid core is this core or there are no valid cores.
    fn affinity_mismatch(&mut self, mut proc: ProcessInfo<T>) -> Option<usize> {
        if let Some(dest) = proc.process.affinity_valid_core() {
            proc.process.set_send_to_core(Some(dest));
            return self.send_to_core(proc);
        }

        self.add_to_wait_queue(proc);
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
                // already on the right core
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
                self.add_to_wait_queue(proc);
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
            is_idle_task: false,
        };
        proc.process.set_id(id);

        if proc.process.check_ready() {
            self.run_queue.push_back(proc);
        } else {
            self.add_to_wait_queue(proc);
        }

        Some(id)
    }

    fn schedule_out(&mut self, state: T::State, tf: &mut T::Frame) {
        let mut proc = self.current_proc.take().expect("current_proc must be scheduled for schedule_out()");
        if !proc.is_idle_task && proc.process.should_kill() {
            self.kill(tf);
            return;
        }

        proc.process.on_task_switch();
        proc.process.set_state(state);
        *proc.process.get_frame() = tf.clone();

        // special processes like idle task aren't stored in self.processes
        if proc.is_idle_task {
            let old = self.idle_proc.replace(proc);
            assert!(old.is_none());
        } else if proc.process.check_ready() {
            self.run_queue.push_back(proc);
        } else {
            self.add_to_wait_queue(proc);
        }
    }

    fn schedule_in(&mut self, tf: &mut T::Frame) -> usize {
        assert!(self.current_proc.is_none());
        if let Some(id) = self.switch_to(tf) {
            return id;
        }

        self.check_waiting_processes();

        if let Some(id) = self.switch_to(tf) {
            return id;
        }

        self.schedule_idle_task(tf)
    }

    fn kill(&mut self, tf: &mut T::Frame) -> Option<usize> {
        let mut proc = self.current_proc.take().expect("current_proc must be scheduled for schedule_out()");
        if proc.is_idle_task {
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

        for proc in self.wait_queue.values_mut() {
            let proc = proc.as_process_mut();
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

        for proc in self.wait_queue.values_mut() {
            let proc = proc.as_process_mut();
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
                is_idle_task: true,
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

    pub fn wake_process(&self, req: WakeRequest<T>) {
        unsafe { self.current_core() }.wake_process(req);
    }

    pub fn broadcast_wake_all_processes(&self) {
        unsafe { self.current_core() }.broadcast_wake_all_processes();
    }
}

impl<T: SchedInfo> Scheduler<T> for &WaitQueueScheduler<T> {
    fn add_process(&mut self, proc: T::Process) -> Option<usize> {
        let id = self.inner.next_process_id();
        let mut proc = ProcessInfo::<T> {
            process: Box::new(proc),
            is_idle_task: false,
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


