use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use alloc::format;
use alloc::vec::Vec;
use core::borrow::{Borrow, BorrowMut};
use core::time::Duration;

use aarch64::{CNTP_CTL_EL0, MPIDR_EL1, SP, SPSR_EL1};
use pi::{interrupt, timer};
use pi::interrupt::CoreInterrupt;

use crate::{IRQ, SCHEDULER, smp};
use crate::cls::CoreLocal;
use crate::mutex::Mutex;
use crate::param::{TICK, USER_IMG_BASE};
use crate::process::{Id, Process, State};
use crate::process::snap::SnapProcess;
use crate::process::state::RunContext;
use crate::traps::TrapFrame;

/// Process scheduler for the entire machine.
pub struct GlobalScheduler(Mutex<Option<Scheduler>>);

extern "C" {
    fn context_restore();

    fn _start();
}

fn reset_timer() {
    use aarch64::regs::*;
    unsafe { CNTV_TVAL_EL0.set(100000) };
}

impl GlobalScheduler {
    /// Returns an uninitialized wrapper around a local scheduler.
    pub const fn uninitialized() -> GlobalScheduler {
        GlobalScheduler(mutex_new!(None))
    }

    /// Enter a critical region and execute the provided closure with the
    /// internal scheduler.
    pub fn critical<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut Scheduler) -> R,
    {
        smp::no_interrupt(|| {
            let mut guard = m_lock!(self.0);
            let r = f(guard.as_mut().expect("scheduler uninitialized"));
            core::mem::drop(guard);
            r
        })
    }

    pub fn crit_process<F, R>(&self, id: Id, f: F) -> R
        where
            F: FnOnce(Option<&mut Process>) -> R,
    {
        self.critical(|scheduler| {
            let mut process: Option<&mut Process> = None;
            for proc in scheduler.processes.iter_mut() {
                if proc.context.tpidr == (id as u64) {
                    process = Some(proc);
                    break;
                }
            }
            f(process)
        })
    }


    /// Adds a process to the scheduler's queue and returns that process's ID.
    /// For more details, see the documentation on `Scheduler::add()`.
    pub fn add(&self, process: Process) -> Option<Id> {
        self.critical(move |scheduler| scheduler.add(process))
    }

    fn switch_to_locked(scheduler: &mut Scheduler, tf: &mut TrapFrame) -> Id {
        if let Some(id) = scheduler.switch_to(tf) {
            id
        } else {
            scheduler.schedule_idle_task(tf)
        }
    }

    /// Performs a context switch using `tf` by setting the state of the current
    /// process to `new_state`, saving `tf` into the current process, and
    /// restoring the next process's trap frame into `tf`. For more details, see
    /// the documentation on `Scheduler::schedule_out()` and `Scheduler::switch_to()`.
    pub fn switch(&self, new_state: State, tf: &mut TrapFrame) -> Id {
        self.critical(|scheduler| {
            scheduler.schedule_out(new_state, tf);
            Self::switch_to_locked(scheduler, tf)
        })
    }

    pub fn switch_to(&self, tf: &mut TrapFrame) -> Id {
        self.critical(|scheduler| Self::switch_to_locked(scheduler, tf))
    }

    /// Kills currently running process and returns that process's ID.
    /// For more details, see the documentaion on `Scheduler::kill()`.
    #[must_use]
    pub fn kill(&self, tf: &mut TrapFrame) -> Option<Id> {
        self.critical(|scheduler| scheduler.kill(tf))
    }

    pub fn bootstrap(&self) -> ! {
        let mut bootstrap_frame: TrapFrame = Default::default();
        self.switch_to(&mut bootstrap_frame);

        let st = (&mut bootstrap_frame) as *mut TrapFrame as u64;
        // let start = bootstrap_frame.sp;
        // let start = _start as u64;

        // let old_sp = SP.get();

        let old_sp = crate::smp::core_stack_top();
        kprintln!("old_sp: {}", old_sp);

        unsafe {
            asm!("  mov x28, $0
                    mov x29, $1
                    mov sp, x28
                    bl context_restore
                    mov sp, x29
                    mov x28, 0
                    mov x29, 0
                    eret"
                    :: "r"(st), "r"(old_sp)
                    :: "volatile");
        }

        loop {}
    }

    /// Starts executing processes in user space using timer interrupt based
    /// preemptive scheduling. This method should not return under normal conditions.
    pub fn start(&self) -> ! {
        let el = unsafe { aarch64::current_el() };
        kprintln!("Current EL: {}", el);

        kprintln!("Enabling timer");
        // timer::tick_in(TICK);
        // interrupt::Controller::new().enable(pi::interrupt::Interrupt::Timer1);

        use aarch64::regs::*;

        let core = smp::core();

        unsafe { ((0x4000_0040 + 4 * core) as *mut u32).write_volatile(0b1010) };
        aarch64::dsb();

        // let v = unsafe { CNTVCT_EL0.get() };
        unsafe { CNTV_TVAL_EL0.set(100000 * 10) };
        unsafe { CNTV_CTL_EL0.set((CNTV_CTL_EL0.get() & !CNTV_CTL_EL0::IMASK) | CNTV_CTL_EL0::ENABLE) };


        // Bootstrap the first process
        self.bootstrap();
    }

    /// Initializes the scheduler and add userspace processes to the Scheduler
    pub unsafe fn initialize(&self) {
        use aarch64::regs::*;
        let lock = &mut m_lock!(self.0);
        if lock.is_none() {
            lock.replace(Scheduler::new());
        }

        let core = crate::smp::core();
        IRQ.register_core(core, CoreInterrupt::CNTVIRQ, Box::new(|tf| {
            SCHEDULER.switch(State::Ready, tf);

            // somewhat redundant, .switch() is suppose to do this.
            reset_timer();
        }));

    }
}

pub struct Scheduler {
    processes: VecDeque<Process>,
    last_id: Option<Id>,
    idle_task: Vec<Process>,
}

impl Scheduler {
    /// Returns a new `Scheduler` with an empty queue.
    fn new() -> Scheduler {
        let mut idle_tasks = Vec::new();
        idle_tasks.reserve_exact(smp::MAX_CORES);
        for i in 0..smp::MAX_CORES {
            let name = format!("idle_task{}", i);
            let proc = Process::kernel_process_old(name, || {
                loop {
                    aarch64::wfe();
                    // trigger context switch immediately after WFE so we dont take a full
                    // scheduler slice.
                    kernel_api::syscall::sched_yield();
                }
            }).expect("failed to create idle task");
            idle_tasks.push(proc);
        }

        Scheduler {
            processes: VecDeque::new(),
            last_id: Some(1), // id zero is reserved for idle task
            idle_task: idle_tasks,
        }
    }

    fn next_id(&mut self) -> Option<Id> {
        let next = self.last_id?.checked_add(1)?;
        self.last_id = Some(next);
        Some(next)
    }

    fn schedule_idle_task(&mut self, tf: &mut TrapFrame) -> Id {
        let core = smp::core();
        let proc = &mut self.idle_task[core];
        let id = Scheduler::load_frame(tf, proc);
        reset_timer();
        id
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
    fn schedule_out(&mut self, mut new_state: State, tf: &mut TrapFrame) -> bool {
        let core = smp::core();
        let proc: Option<(usize, &mut Process)>;
        if self.idle_task[core].context.tpidr == tf.tpidr {
            proc = Some((usize::max_value(), &mut self.idle_task[core]));
        } else {
            proc = self.processes.iter_mut().enumerate()
                .find(|(_, p)| p.context.tpidr == tf.tpidr);
        }

        match proc {
            None => false,
            Some((idx, proc)) => {
                if proc.has_request_kill() {
                    self.kill(tf);
                } else {
                    proc.task_switches += 1;
                    proc.set_state(new_state);
                    *(proc.context.borrow_mut()) = *tf;

                    // special processes like idle task aren't stored in self.processes
                    if idx != usize::max_value() {
                        // something is very bad if the entry we found is no longer here.
                        let owned = self.processes.remove(idx).expect("could not find process in self.processes");
                        self.processes.push_back(owned);
                    }
                }

                true
            }
        }
    }

    fn load_frame(tf: &mut TrapFrame, proc: &mut Process) -> Id {
        let core_id = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize };
        let now = pi::timer::current_time();

        proc.set_state(State::Running(RunContext { core_id, scheduled_at: now }));
        *tf = *proc.context.borrow();

        proc.context.tpidr
    }

    /// Finds the next process to switch to, brings the next process to the
    /// front of the `processes` queue, changes the next process's state to
    /// `Running`, and performs context switch by restoring the next process`s
    /// trap frame into `tf`.
    ///
    /// If there is no process to switch to, returns `None`. Otherwise, returns
    /// `Some` of the next process`s process ID.
    fn switch_to(&mut self, tf: &mut TrapFrame) -> Option<Id> {
        let core = smp::core();

        // is_ready() is &mut so it doesn't work with .find() 😡😡😡
        let mut proc: Option<(usize, &mut Process)> = None;
        for entry in self.processes.iter_mut().enumerate() {
            if entry.1.affinity.check(core) && entry.1.is_ready() {
                proc = Some(entry);
                break;
            }
        }

        let (idx, proc) = proc?;

        let id = Scheduler::load_frame(tf, proc);

        // kprintln!("switching to: {}", id);

        // something is very bad if the entry we found is no longer here.
        let owned = self.processes.remove(idx).unwrap();
        self.processes.push_front(owned);

        reset_timer();

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
                proc.set_state(State::Dead);
                *(proc.context.borrow_mut()) = *tf;

                for comp in proc.dead_completions.drain(..) {
                    comp.complete(proc.context.tpidr);
                }

                // something is very bad if the entry we found is no longer here.
                let proc = self.processes.remove(idx).unwrap();

                Some(proc.context.tpidr)
            }
        }
    }

    pub fn get_process_snaps(&mut self, snaps: &mut Vec<SnapProcess>) {
        for core in &self.idle_task {
            snaps.push(SnapProcess::from(core));
        }

        for proc in self.processes.iter() {
            snaps.push(SnapProcess::from(proc));
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

