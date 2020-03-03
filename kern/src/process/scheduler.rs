use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use core::borrow::{Borrow, BorrowMut};
use core::fmt;

use aarch64::*;

use crate::mutex::Mutex;
use crate::param::{PAGE_MASK, PAGE_SIZE, TICK, USER_IMG_BASE};
use crate::process::{Id, Process, State};
use crate::traps::TrapFrame;
use crate::{VMM, IRQ, SCHEDULER};
use crate::shell;
use crate::console::kprint;
use crate::console::kprintln;
use core::time::Duration;

/// Process scheduler for the entire machine.
#[derive(Debug)]
pub struct GlobalScheduler(Mutex<Option<Scheduler>>);

extern "C" {
    fn context_restore();

    fn _start();
}


impl GlobalScheduler {
    /// Returns an uninitialized wrapper around a local scheduler.
    pub const fn uninitialized() -> GlobalScheduler {
        GlobalScheduler(Mutex::new(None))
    }

    /// Enter a critical region and execute the provided closure with the
    /// internal scheduler.
    pub fn critical<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut Scheduler) -> R,
    {
        let mut guard = self.0.lock();
        f(guard.as_mut().expect("scheduler uninitialized"))
    }


    /// Adds a process to the scheduler's queue and returns that process's ID.
    /// For more details, see the documentation on `Scheduler::add()`.
    pub fn add(&self, mut process: Process) -> Option<Id> {
        process.context.ttbr0 = VMM.get_baddr().as_u64();
        process.context.ttbr1 = process.vmap.get_baddr().as_u64();
        process.context.elr = USER_IMG_BASE as u64;

        self.critical(move |scheduler| scheduler.add(process))
    }

    /// Performs a context switch using `tf` by setting the state of the current
    /// process to `new_state`, saving `tf` into the current process, and
    /// restoring the next process's trap frame into `tf`. For more details, see
    /// the documentation on `Scheduler::schedule_out()` and `Scheduler::switch_to()`.
    pub fn switch(&self, new_state: State, tf: &mut TrapFrame) -> Id {
        self.critical(|scheduler| scheduler.schedule_out(new_state, tf));
        self.switch_to(tf)
    }

    pub fn switch_to(&self, tf: &mut TrapFrame) -> Id {
        loop {
            let rtn = self.critical(|scheduler| scheduler.switch_to(tf));
            if let Some(id) = rtn {
                return id;
            }

            // FIXME lol??????? I don't want to DIE
            unsafe {
                let mut v = aarch64::regs::CNTKCTL_EL1.get();
                let m = aarch64::regs::CNTKCTL_EL1::EVNTI;

                v &= !m;
                // seems to wait around 134μs
                v |= (13) << m.trailing_zeros();

                v |= aarch64::regs::CNTKCTL_EL1::EVNTEN;

                aarch64::regs::CNTKCTL_EL1.set(v);
            }

            aarch64::wfe();

        }
    }

    /// Kills currently running process and returns that process's ID.
    /// For more details, see the documentaion on `Scheduler::kill()`.
    #[must_use]
    pub fn kill(&self, tf: &mut TrapFrame) -> Option<Id> {
        self.critical(|scheduler| scheduler.kill(tf))
    }

    /// Starts executing processes in user space using timer interrupt based
    /// preemptive scheduling. This method should not return under normal conditions.
    pub fn start(&self) -> ! {
        // let mut proc = Process::new().unwrap();
        // proc.state = State::Running;
        // proc.context.elr = my_thread as u64;
        // proc.context.sp = proc.stack.top().as_u64();
        // proc.context.spsr = 0;

        let el = unsafe { aarch64::current_el() };
        kprintln!("Current EL: {}", el);

        IRQ.register(pi::interrupt::Interrupt::Timer1, Box::new(|tf| {
            pi::timer::tick_in(TICK);
            // kprintln!("TICK");
            SCHEDULER.switch(State::Ready, tf);
        }));

        pi::timer::tick_in(TICK);
        pi::interrupt::Controller::new().enable(pi::interrupt::Interrupt::Timer1);


        // Bootstrap the first process

        let mut bootstrap_frame: TrapFrame = Default::default();
        self.switch_to(&mut bootstrap_frame);

        let st = (&mut bootstrap_frame) as *mut TrapFrame as u64;
        let start = _start as u64;

        unsafe {
            asm!("  mov x0, $0
                    mov sp, x0"
                    :: "r"(st)
                    :: "volatile");
        }

        unsafe { context_restore(); }

        unsafe {
            asm!("  mov x0, $0
                    mov sp, x0"
                    :: "r"(start)
                    :: "volatile");
        }

        aarch64::eret();

        loop {}
    }

    /// Initializes the scheduler and add userspace processes to the Scheduler
    pub unsafe fn initialize(&self) {
        let lock = &mut self.0.lock();
        if lock.is_none() {
            lock.replace(Scheduler::new());
        }
    }

    // The following method may be useful for testing Phase 3:
    //
    // * A method to load a extern function to the user process's page table.
    //
    pub fn test_phase_3(&self, proc: &mut Process){
        use crate::vm::{VirtualAddr, PagePerm};

        let len = 50;

        let mut page = proc.vmap.alloc(
            VirtualAddr::from(USER_IMG_BASE as u64), PagePerm::RWX);

        let text = unsafe {
            core::slice::from_raw_parts(test_user_process as *const u8, len)
        };

        page[0..len].copy_from_slice(text);
    }
}

#[derive(Debug)]
pub struct Scheduler {
    processes: VecDeque<Process>,
    last_id: Option<Id>,
}

impl Scheduler {
    /// Returns a new `Scheduler` with an empty queue.
    fn new() -> Scheduler {
        Scheduler {
            processes: VecDeque::new(),
            last_id: Some(1),
        }
    }

    fn next_id(&mut self) -> Option<Id> {
        let next = self.last_id?.checked_add(1)?;
        self.last_id = Some(next);
        Some(next)
    }

    /// Adds a process to the scheduler's queue and returns that process's ID if
    /// a new process can be scheduled. The process ID is newly allocated for
    /// the process and saved in its `trap_frame`. If no further processes can
    /// be scheduled, returns `None`.
    ///
    /// It is the caller's responsibility to ensure that the first time `switch`
    /// is called, that process is executing on the CPU.
    fn add(&mut self, mut process: Process) -> Option<Id> {
        let id = self.next_id()?;
        process.context.tpidr = id;
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
    fn schedule_out(&mut self, new_state: State, tf: &mut TrapFrame) -> bool {
        let proc = self.processes.iter_mut().enumerate()
            .find(|(_, p)| p.context.tpidr == tf.tpidr);
        match proc {
            None => false,
            Some((idx, proc)) => {
                proc.state = new_state;
                *(proc.context.borrow_mut()) = *tf;

                // something is very bad if the entry we found is no longer here.
                let owned = self.processes.remove(idx).unwrap();
                self.processes.push_back(owned);

                true
            }
        }
    }

    /// Finds the next process to switch to, brings the next process to the
    /// front of the `processes` queue, changes the next process's state to
    /// `Running`, and performs context switch by restoring the next process`s
    /// trap frame into `tf`.
    ///
    /// If there is no process to switch to, returns `None`. Otherwise, returns
    /// `Some` of the next process`s process ID.
    fn switch_to(&mut self, tf: &mut TrapFrame) -> Option<Id> {

        // is_ready() is &mut so it doesn't work with .find() 😡😡😡
        let mut proc: Option<(usize, &mut Process)> = None;
        for entry in self.processes.iter_mut().enumerate() {
            if entry.1.is_ready() {
                proc = Some(entry);
                break;
            }
        }

        let (idx, proc) = proc?;

        proc.state = State::Running;
        *tf = *proc.context.borrow();

        let id = proc.context.tpidr;

        // something is very bad if the entry we found is no longer here.
        let owned = self.processes.remove(idx).unwrap();
        self.processes.push_front(owned);

        Some(id)
    }

    /// Kills currently running process by scheduling out the current process
    /// as `Dead` state. Removes the dead process from the queue, drop the
    /// dead process's instance, and returns the dead process's process ID.
    fn kill(&mut self, tf: &mut TrapFrame) -> Option<Id> {
        let proc = self.processes.iter_mut().enumerate()
            .find(|(_, p)| p.context.tpidr == tf.tpidr);
        match proc {
            None => None,
            Some((idx, proc)) => {
                proc.state = State::Dead;
                *(proc.context.borrow_mut()) = *tf;

                // something is very bad if the entry we found is no longer here.
                let proc = self.processes.remove(idx).unwrap();

                Some(proc.context.tpidr)
            }
        }
    }
}

pub extern "C" fn test_user_process() -> ! {
    loop {
        let ms = 10000;
        let error: u64;
        let elapsed_ms: u64;

        unsafe {
            asm!("mov x0, $2
              brk 7
              svc 1
              mov $0, x0
              mov $1, x7"
                 : "=r"(elapsed_ms), "=r"(error)
                 : "r"(ms)
                 : "x0", "x7"
                 : "volatile");
        }
    }
}

