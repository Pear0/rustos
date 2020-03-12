use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use core::borrow::{Borrow, BorrowMut};
use core::time::Duration;
use alloc::format;

use aarch64::{MPIDR_EL1, SP, SPSR_EL1, CNTP_CTL_EL0};
use pi::{interrupt, timer};

use crate::{IRQ, SCHEDULER, smp};
use crate::cls::CoreLocal;
use crate::console::{kprint, kprintln};
use crate::mutex::{mutex_new, Mutex};
use crate::param::{TICK, USER_IMG_BASE};
use crate::process::{Id, Process, State};
use crate::process::snap::SnapProcess;
use crate::traps::TrapFrame;
use crate::process::state::RunContext;
use pi::interrupt::CoreInterrupt;
use crate::mutex::m_lock;

/// Process scheduler for the entire machine.
pub struct GlobalScheduler(Mutex<Option<Scheduler>>);

extern "C" {
    fn context_restore();

    fn _start();
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


    /// Adds a process to the scheduler's queue and returns that process's ID.
    /// For more details, see the documentation on `Scheduler::add()`.
    pub fn add(&self, process: Process) -> Option<Id> {
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
        self.critical(|scheduler| {
            if let Some(id) = scheduler.switch_to(tf) {
                id
            } else {
                scheduler.schedule_idle_task(tf)
            }
        })

        // loop {
            // let rtn = self.critical(|scheduler| scheduler.switch_to(tf));
            // if let Some(id) = rtn {
            //     return id;
            // }

            // FIXME lol??????? I don't want to DIE
            // unsafe {
            //     let mut v = aarch64::regs::CNTKCTL_EL1.get();
            //     let m = aarch64::regs::CNTKCTL_EL1::EVNTI;
            //
            //     v &= !m;
            //     // seems to wait around 134μs
            //     v |= (13) << m.trailing_zeros();
            //
            //     v |= aarch64::regs::CNTKCTL_EL1::EVNTEN;
            //
            //     aarch64::regs::CNTKCTL_EL1.set(v);
            // }


            // self.mask_next_tick(true);
            // unsafe { aarch64::sti() }
            // aarch64::wfe();
            // unsafe { aarch64::cli() }
            // self.mask_next_tick(false);
        // }
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

        // unsafe {
        //     asm!("  mov x0, $0
        //             mov sp, x0"
        //             :: "r"(st)
        //             :: "volatile");
        // }
        //
        // unsafe { context_restore(); }

        unsafe {
            asm!("  mov x28, $0
                    mov x29, $1
                    mov sp, x28
                    bl context_restore
                    mov sp, x29
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
        unsafe { CNTV_CTL_EL0.set((CNTV_CTL_EL0.get() & !CNTV_CTL_EL0::IMASK ) | CNTV_CTL_EL0::ENABLE) } ;
        
        
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

            // let v = unsafe { CNTVCT_EL0.get() };
            unsafe { CNTV_TVAL_EL0.set(100000) };
            
            // kprintln!("foo");

        }));

        // let v = unsafe { CNTPCT_EL0.get() };
        // unsafe { CNTP_CVAL_EL0.set(v + 10000) };

        // IRQ.register(pi::interrupt::Interrupt::Timer1, Box::new(|tf| {
        //     timer::tick_in(TICK);
        //     // aarch64::sev(); // tick other threads
        //
        //     // let core_id = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize };
        //     // kprint!("IRQ: {}", core_id);
        //
        //     if !SCHEDULER.is_mask_next_tick() {
        //         // kprint!("s");
        //         SCHEDULER.switch(State::Ready, tf);
        //     }
        // }));

    }

    // The following method may be useful for testing Phase 3:
    //
    // * A method to load a extern function to the user process's page table.
    //
    pub fn test_phase_3(&self, proc: &mut Process) {
        use crate::vm::{VirtualAddr, PagePerm};

        let len = 50;

        let page = proc.vmap.alloc(
            VirtualAddr::from(USER_IMG_BASE as u64), PagePerm::RWX);

        let text = unsafe {
            core::slice::from_raw_parts(test_user_process as *const u8, len)
        };

        page[0..len].copy_from_slice(text);
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
                }
            }).expect("failed to create idle task");
            idle_tasks.push(proc);
        }

        Scheduler {
            processes: VecDeque::new(),
            last_id: Some(1),
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
        Scheduler::load_frame(tf, proc)
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
                if let State::Running(ctx) = &proc.state {
                    let delta = pi::timer::current_time() - ctx.scheduled_at;
                    proc.cpu_time += delta;
                }

                proc.task_switches += 1;
                proc.state = new_state;
                *(proc.context.borrow_mut()) = *tf;

                // something is very bad if the entry we found is no longer here.
                let owned = self.processes.remove(idx).unwrap();
                self.processes.push_back(owned);

                true
            }
        }
    }

    fn load_frame(tf: &mut TrapFrame, proc: &mut Process) -> Id {
        let core_id = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize };
        let now = pi::timer::current_time();

        proc.state = State::Running(RunContext { core_id, scheduled_at: now });
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

    pub fn get_process_snaps(&mut self, snaps: &mut Vec<SnapProcess>) {
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

