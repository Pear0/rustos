use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use alloc::format;
use alloc::vec::Vec;
use core::borrow::{Borrow, BorrowMut};
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;

use enumset::EnumSet;
use hashbrown::HashSet;

use aarch64::{CNTP_CTL_EL0, MPIDR_EL1, SP, SPSR_EL1};
use dsx::sync::mutex::{LightMutex, LockableMutex};
use dsx::sync::sema::SingleSetSemaphore;
use karch::capability::ExecCapability;
use kscheduler::{Process as KProcess, SchedInfo, Scheduler as KScheduler};
use pi::{interrupt, timer};
use pi::interrupt::CoreInterrupt;

use crate::{BootVariant, EXEC_CONTEXT, smp, timing};
use crate::arm::{GenericCounterImpl, HyperPhysicalCounter, VirtualCounter};
use crate::cls::CoreLocal;
use crate::hyper::HYPER_TIMER;
use crate::kernel::{KERNEL_CORES, KERNEL_TIMER};
use crate::mutex::Mutex;
use crate::param::{TICK, USER_IMG_BASE};
use crate::process::{HyperImpl, Id, KernelImpl, Process, State};
use crate::process::process::ProcessImpl;
use crate::process::snap::SnapProcess;
use crate::process::state::RunContext;
use crate::traps::{Frame, HyperTrapFrame, IRQ_RECURSION_DEPTH, KernelTrapFrame};

/// Process scheduler for the entire machine.
pub struct GlobalScheduler<T: ProcessImpl>(SingleSetSemaphore<Scheduler<T>>);

extern "C" {
    fn context_restore();

    fn _start();
}

fn reset_timer() {
    use aarch64::regs::*;

    match BootVariant::get_variant() {
        BootVariant::Kernel => {

            // VirtualCounter::set_timer_duration(Duration::from_millis(10))
        }
        BootVariant::Hypervisor => HyperPhysicalCounter::set_timer_duration(Duration::from_millis(5)),
        _ => panic!("somehow got unknown boot variant"),
    }
}

impl<T: ProcessImpl> GlobalScheduler<T> {
    /// Returns an uninitialized wrapper around a local scheduler.
    pub const fn uninitialized() -> Self {
        GlobalScheduler(SingleSetSemaphore::new())
    }

    /// Enter a critical region and execute the provided closure with the
    /// internal scheduler.
    #[track_caller]
    #[inline(always)]
    pub fn critical<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut &Scheduler<T>) -> R,
    {
        // take guard only if we are not in an exception context.
        // this way the profiling timer can inspect exception context scheduler behavior.
        // TODO refactor so that a scheduler guard does not need to block interrupts at all.
        let int_guard = smp::interrupt_guard_outside_exc();

        assert!(EXEC_CONTEXT.has_capabilities(EnumSet::only(ExecCapability::Allocation)));

        let r = EXEC_CONTEXT.lock_capability(EnumSet::only(ExecCapability::Scheduler), || {
            let mut guard = &*self.0;
            f(&mut guard)
        });

        // drop(int_guard);
        r
    }

    #[track_caller]
    #[inline(always)]
    pub fn crit_process<F, R>(&self, id: Id, f: F) -> R
        where
            F: FnOnce(Option<&mut Process<T>>) -> R,
    {
        self.critical(|scheduler| {
            scheduler.with_process_mut(id as usize, f)
        })
    }


    /// Adds a process to the scheduler's queue and returns that process's ID.
    /// For more details, see the documentation on `Scheduler::add()`.
    pub fn add(&self, process: Process<T>) -> Option<Id> {
        self.critical(move |scheduler| scheduler.add_process(process).map(|x| x as Id))
    }

    /// Performs a context switch using `tf` by setting the state of the current
    /// process to `new_state`, saving `tf` into the current process, and
    /// restoring the next process's trap frame into `tf`. For more details, see
    /// the documentation on `Scheduler::schedule_out()` and `Scheduler::switch_to()`.
    pub fn switch(&self, new_state: State<T>, tf: &mut T::Frame) -> Id {
        self.critical(|scheduler| {
            scheduler.switch(new_state, tf) as Id
        })
    }

    pub fn switch_to(&self, tf: &mut T::Frame) -> Id {
        self.critical(|scheduler| scheduler.schedule_in(tf) as Id)
    }

    /// Kills currently running process and returns that process's ID.
    /// For more details, see the documentaion on `Scheduler::kill()`.
    #[must_use]
    pub fn kill(&self, tf: &mut T::Frame) -> Option<Id> {
        self.critical(|scheduler| scheduler.kill(tf).map(|x| x as Id))
    }

    pub fn bootstrap(&self) -> ! {
        self.critical(|s| s.initialize_core());

        let mut bootstrap_frame: T::Frame = Default::default();
        self.switch_to(&mut bootstrap_frame);

        let st = (&mut bootstrap_frame) as *mut T::Frame as u64;

        let old_sp = crate::smp::core_stack_top();
        // kprintln!("old_sp: {}", old_sp);

        unsafe {
            llvm_asm!("  mov x28, $0
                    mov x29, $1
                    mov sp, x28
                    bl kernel_context_restore
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
        // let el = unsafe { aarch64::current_el() };
        // kprintln!("Current EL: {}", el);
        //
        // kprintln!("Enabling timer");
        // timer::tick_in(TICK);
        // interrupt::Controller::new().enable(pi::interrupt::Interrupt::Timer1);

        // use aarch64::regs::*;

        // let core = smp::core();
        // unsafe { ((0x4000_0040 + 4 * core) as *mut u32).write_volatile(0b1010) };
        aarch64::dsb();

        // let v = unsafe { CNTVCT_EL0.get() };
        // unsafe { CNTV_TVAL_EL0.set(100000 * 10) };
        // unsafe { CNTV_CTL_EL0.set((CNTV_CTL_EL0.get() & !CNTV_CTL_EL0::IMASK) | CNTV_CTL_EL0::ENABLE) };


        // Bootstrap the first process
        self.bootstrap();
    }
}

impl GlobalScheduler<KernelImpl> {
    /// Initializes the scheduler and add userspace processes to the Scheduler
    pub unsafe fn initialize_kernel(&self) {
        use crate::kernel::{KERNEL_IRQ, KERNEL_SCHEDULER};
        use aarch64::regs::*;
        if !SingleSetSemaphore::<Scheduler<KernelImpl>>::is_initialized(&self.0) {
            SingleSetSemaphore::<Scheduler<KernelImpl>>::set_racy(&self.0, Scheduler::new(MySchedInfo::new(), *KERNEL_CORES));
        }

        let core = crate::smp::core();
        // KERNEL_IRQ.register_core(core, CoreInterrupt::CNTVIRQ, Box::new(|tf| {
        //     KERNEL_SCHEDULER.switch(State::Ready, tf);
        //
        //     // somewhat redundant, .switch() is suppose to do this.
        //     reset_timer();
        // }));

        let skip_ticks = AtomicU32::new(10);
        KERNEL_TIMER.add(5, timing::time_to_cycles::<VirtualCounter>(Duration::from_millis(10)), Box::new(move |ctx| {
            if !EXEC_CONTEXT.has_capabilities(ExecCapability::Allocation | ExecCapability::Scheduler) {
                ctx.defer_timer();
            } else if IRQ_RECURSION_DEPTH.get() > 1 {
                ctx.no_reschedule();
            } else {
                if skip_ticks.load(Ordering::Relaxed) <= 0 {
                    KERNEL_SCHEDULER.switch(State::Ready, ctx.data);
                } else {
                    skip_ticks.fetch_sub(1, Ordering::Relaxed);
                }
            }
        }));

        EXEC_CONTEXT.add_capabilities(EnumSet::only(ExecCapability::Scheduler));
    }

    pub fn iter_all_processes<F>(&self, mut func: F) where F: FnMut(usize, &mut Process<KernelImpl>) {
        let mut seen_procs = HashSet::<usize>::with_capacity(32);

        smp::with_each_core(|core_id| {
            self.critical(|sched| {
                sched.iter_process_mut(|proc| {
                    if proc.get_id() == 0 {
                        func(core_id, proc);
                    } else if !seen_procs.contains(&(proc.get_id())) {
                        func(core_id, proc);
                        seen_procs.insert(proc.get_id());
                    }
                });
            });
        });
    }

    pub fn get_all_process_snaps(&self, snaps: &mut Vec<SnapProcess>) {
        self.iter_all_processes(|core_id, proc| {
            let mut snap = SnapProcess::from(&*proc);
            snap.core = core_id as isize;
            snaps.push(snap);
        });
    }

    pub fn get_core_process_snaps(&self, snaps: &mut Vec<SnapProcess>) {
        self.critical(|sched| {
            sched.iter_process_mut(|proc| {
                let mut snap = SnapProcess::from(&*proc);
                snap.core = smp::core() as isize;
                snaps.push(snap);
            });
        });
    }
}

impl GlobalScheduler<HyperImpl> {
    /// Initializes the scheduler and add userspace processes to the Scheduler
    pub unsafe fn initialize_hyper(&self) {
        use crate::hyper::{HYPER_IRQ, HYPER_SCHEDULER};
        use aarch64::regs::*;
        if !SingleSetSemaphore::<Scheduler<HyperImpl>>::is_initialized(&self.0) {
            SingleSetSemaphore::<Scheduler<HyperImpl>>::set_racy(&self.0, Scheduler::new(MySchedInfo::new(), 1));
        }

        HYPER_TIMER.critical(|timer| {
            timer.add(5, timing::time_to_cycles::<HyperPhysicalCounter>(Duration::from_millis(5)), Box::new(|ctx| {
                if IRQ_RECURSION_DEPTH.get() > 1 {
                    ctx.no_reschedule();
                } else {
                    HYPER_SCHEDULER.switch(State::Ready, ctx.data);
                }
            }));
        });
    }

    pub fn bootstrap_hyper(&self) -> ! {
        let mut bootstrap_frame: HyperTrapFrame = Default::default();
        self.switch_to(&mut bootstrap_frame);

        let st = (&mut bootstrap_frame) as *mut HyperTrapFrame as u64;

        let old_sp = crate::smp::core_stack_top();
        // kprintln!("old_sp: {}", old_sp);

        unsafe {
            llvm_asm!("  mov x28, $0
                    mov x29, $1
                    mov sp, x28
                    bl hyper_context_restore
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
    pub fn start_hyper(&self) -> ! {
        // let el = unsafe { aarch64::current_el() };
        // kprintln!("Current EL: {}", el);

        // timer::tick_in(TICK);
        // interrupt::Controller::new().enable(pi::interrupt::Interrupt::Timer1);

        use aarch64::regs::*;

        let core = smp::core();

        // Since the Raspberry Pi 3 does not have an ARM GIC that would be able to
        // re-route a physical interrupt as a virtual one, we need to do something
        // hacky to enable routing some interrupts to the guest, and some to the
        // hypervisor. In this case, FIQs are not trapped by the hypervisor and so
        // affect the guest. It is the job of the hypervisor to ensure that only the
        // correct guest receives FIQs.

        let mut local_flags = 0;
        local_flags |= 1 << 7; // nCNTVIRQ FIQ
        local_flags |= 1 << 5; // nCNTPNSIRQ FIQ
        local_flags |= 1 << 2; // nCNTHPIRQ IRQ
        unsafe { ((0x4000_0040 + 4 * core) as *mut u32).write_volatile(local_flags) };
        aarch64::dsb();

        unsafe { CNTHP_TVAL_EL2.set(100000) };
        unsafe { CNTHP_CTL_EL2.set((CNTHP_CTL_EL2.get() & !CNTHP_CTL_EL2::IMASK) | CNTHP_CTL_EL2::ENABLE) };

        // Bootstrap the first process
        self.bootstrap_hyper();
    }

    pub fn get_process_snaps(&self, snaps: &mut Vec<SnapProcess>) {
        self.critical(|sched| {
            sched.iter_process_mut(|proc| {
                snaps.push(SnapProcess::from(&*proc));
            });
        });
    }
}

pub struct MySchedInfo<T: ProcessImpl> {
    idle_tasks: Vec<LightMutex<Option<Process<T>>>>,
}

impl<T: ProcessImpl> MySchedInfo<T> {
    pub fn new() -> Self {
        let idle_tasks = T::create_idle_processes(smp::MAX_CORES);
        assert_eq!(idle_tasks.len(), smp::MAX_CORES);

        let idle_tasks = idle_tasks.into_iter()
            .map(|x| LightMutex::new(Some(x)))
            .collect();

        Self { idle_tasks }
    }
}

impl<T: ProcessImpl> kscheduler::SchedInfo for MySchedInfo<T> {
    type Frame = T::Frame;
    type State = State<T>;
    type Process = Process<T>;

    fn current_core(&self) -> usize {
        smp::core()
    }

    fn get_idle_tasks(&self) -> &[LightMutex<Option<Self::Process>>] {
        &self.idle_tasks
    }

    fn running_state(&self) -> Self::State {
        let core_id = smp::core();
        let scheduled_at = crate::timing::clock_time_phys();

        State::Running(RunContext { core_id, scheduled_at })
    }

    fn dead_state(&self) -> Self::State {
        State::Dead
    }

    fn on_process_killed(&self, mut proc: Self::Process) {
        T::on_process_killed(&mut proc);
    }
}

type Scheduler<T> = kscheduler::wqs::WaitQueueScheduler<MySchedInfo<T>>;

#[allow(unused_assignments)]
pub extern "C" fn test_user_process() -> ! {
    loop {
        let ms = 10000;
        let error: u64;
        let elapsed_ms: u64;

        unsafe {
            llvm_asm!("mov x0, $2
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

