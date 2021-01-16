use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::time::Duration;

use aarch64::{MPIDR_EL1, SP};

use crate::{init, BootVariant, smp, timing};
use crate::mutex::Mutex;
use crate::console::{CONSOLE, console_flush};
use crate::mbox::EveryTimer;
use crate::traps::IRQ_RECURSION_DEPTH;
use crate::kernel::{KERNEL_CORES, KERNEL_SCHEDULER};
use crate::kernel_call::syscall::exec_in_exc;
use crate::process::CoreAffinity;
use kscheduler::Process;

pub const MAX_CORES: usize = 4;

// utilities to handle secondary cores initialization.

const STACK_SIZE: usize = 32 * 1024;

struct ParkingSpot {
    addr: AtomicU64,
    enabled: AtomicBool,
    stack: [u8; STACK_SIZE],
}

const fn parking() -> ParkingSpot {
    ParkingSpot {
        addr: AtomicU64::new(0),
        enabled: AtomicBool::new(false),
        stack: [0; STACK_SIZE],
    }
}

static PARKING: [ParkingSpot; 4] = [parking(), parking(), parking(), parking()];

// must not use stack
#[inline(never)]
#[naked]
pub unsafe fn core_bootstrap() -> ! {
    let core_id = MPIDR_EL1.get_value(MPIDR_EL1::Aff0);

    let mut stack_top = ((PARKING[core_id as usize].stack.as_ptr() as usize) + STACK_SIZE);
    stack_top -= stack_top % 16; // align to 16
    SP.set(stack_top);

    core_bootstrap_stack();
}

pub fn core() -> usize {
    unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize }
}

#[inline(never)]
unsafe fn core_bootstrap_stack() -> ! {
    init::switch_to_el2();

    if BootVariant::kernel() {
        init::switch_to_el1();
        init::el1_init();
    } else {
        init::el2_init();
    }

    //
    // let el = unsafe { aarch64::current_el() };
    // kprintln!("Big Current EL: {}", el);


    let core_id = MPIDR_EL1.get_value(MPIDR_EL1::Aff0);

    PARKING[core_id as usize].enabled.store(true, Ordering::SeqCst);

    timing::sleep_phys(Duration::from_millis(10 * core_id));
    // kprintln!("bootstrap @ {} {}", core_id, MPIDR_EL1.get_value(MPIDR_EL1::Aff0));

    loop {
        aarch64::dsb();
        let core_id = MPIDR_EL1.get_value(MPIDR_EL1::Aff0);

        let func = PARKING[core_id as usize].addr.load(Ordering::SeqCst);
        if func != 0 {
            core::mem::transmute::<u64, fn()>(func)();
            aarch64::dsb();
            PARKING[core_id as usize].addr.store(0, Ordering::SeqCst);
            aarch64::dsb();
            aarch64::clean_data_cache_obj(& PARKING[core_id as usize].addr);
            aarch64::sev();
        }

        aarch64::wfe();
    }
}

pub fn core_stack_top() -> u64 {
    let core_id = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) };

    let mut stack_top = ((PARKING[core_id as usize].stack.as_ptr() as usize) + STACK_SIZE);
    stack_top -= stack_top % 16; // align to 16
    stack_top as u64
}

pub unsafe fn initialize(cores: usize) {
    let parking_base = 0xd8;

    for core_id in 1..cores {
        ((parking_base + core_id * 8) as *mut u64).write_volatile(core_bootstrap as u64);
    }
    aarch64::clean_data_cache_region(parking_base as u64, 4 * 8);
    aarch64::sev();
}

pub fn count_cores() -> usize {
    let mut count = 0;
    for core in PARKING.iter() {
        if core.enabled.load(Ordering::SeqCst) {
            count += 1;
        }
    }
    count
}

pub fn wait_for_cores(cores: usize) {
    let mut every = EveryTimer::new(Duration::from_secs(1));
    while count_cores() < cores - 1 {
        let count = count_cores();
        every.every(|| error!("waiting for cores, have: {}", count));
        aarch64::sev();
    }
}

pub fn run_no_return(func: fn()) {
    for (id, core) in PARKING.iter().enumerate() {
        if core.enabled.load(Ordering::SeqCst) {
            core.addr.store(func as u64, Ordering::SeqCst);

            // Ensure other cores see our write regardless of our caching state.
            aarch64::clean_data_cache_obj(&core.addr);
        }
    }

    aarch64::dsb();
    aarch64::sev();
}

pub fn run_on_secondary_cores(func: fn()) {
    for (id, core) in PARKING.iter().enumerate() {
        if core.enabled.load(Ordering::SeqCst) {
            aarch64::dsb();
            core.addr.store(func as u64, Ordering::SeqCst);

            // Ensure other cores see our write regardless of our caching state.
            aarch64::clean_data_cache_obj(&core.addr);

            aarch64::sev();

            loop {
                timing::sleep_phys(Duration::from_micros(10));
                aarch64::dsb();
                if core.addr.load(Ordering::SeqCst) == 0 {
                    break;
                }
            }
        }
    }

}

pub fn run_on_all_cores(func: fn()) {
    func();
    aarch64::dsb();
    run_on_secondary_cores(func);
}

pub struct CriticalGuard {
    restore_daif: u64,
}

impl !Sync for CriticalGuard {}
impl !Send for CriticalGuard {}

impl Drop for CriticalGuard {
    fn drop(&mut self) {
        use aarch64::regs::*;
        unsafe {
            aarch64::dsb();
            DAIF.set(self.restore_daif);
        }
    }
}

pub fn interrupt_guard() -> CriticalGuard {
    use aarch64::regs::*;
    let full_mask = DAIF::D | DAIF::A | DAIF::I | DAIF::F;

    unsafe {
        let restore_daif = DAIF.get_masked(full_mask);
        DAIF.set(full_mask);
        aarch64::dsb();

        CriticalGuard { restore_daif }
    }
}

pub fn interrupt_guard_outside_exc() -> Option<CriticalGuard> {
    if IRQ_RECURSION_DEPTH.get() == 0 {
        Some(interrupt_guard())
    } else {
        None
    }
}

#[inline(always)]
pub fn no_interrupt<T, R>(func: T) -> R
    where T: (FnOnce() -> R) {

    let guard = interrupt_guard();
    let r = func();
    drop(guard);
    r
}

pub fn process_context() -> bool {
    IRQ_RECURSION_DEPTH.get() == 0
}

pub fn exception_context() -> bool {
    IRQ_RECURSION_DEPTH.get() == 1
}

/// Execute the callback on each core then restore original process affinity.
///
/// The callback will first be called on the current core after the affinity has
/// been locked. Then the callback will be called on the remaining cores in order.
///
/// This function must be called from a process context and executes `1 + 2 * num_cores` syscalls.
pub fn with_each_core<F>(mut func: F) where F: FnMut(usize) {
    assert!(process_context());
    let mut original_affinity: Option<CoreAffinity> = None;
    let mut original_core = 0usize;

    // lock this process to the current core...
    exec_in_exc(|exc| {
        KERNEL_SCHEDULER.crit_process(exc.pid, |proc| {
            let proc = proc.unwrap();
            original_affinity = Some(proc.affinity);
            original_core = smp::core();
            proc.affinity.set_only(original_core);
        });
    });

    func(original_core);

    for core_i in 0..*KERNEL_CORES {
        // skip the original core now
        if core_i == original_core {
            continue;
        }

        // move us to next core
        exec_in_exc(|exc| {
            KERNEL_SCHEDULER.crit_process(exc.pid, |proc| {
                let proc = proc.unwrap();
                proc.affinity.set_only(core_i);
            });
        });

        // trigger switch so that we are sent to the new core affinity.
        kernel_api::syscall::sched_yield();

        func(core_i);
    }

    // Restore original affinity and send process back to the core it was originally scheduled on.
    exec_in_exc(|exc| {
        KERNEL_SCHEDULER.crit_process(exc.pid, |proc| {
            let mut proc = proc.unwrap();
            proc.affinity = original_affinity.unwrap();
            proc.set_send_to_core(Some(original_core));
        });
    });

    // trigger switch so that we are sent back to our original core
    kernel_api::syscall::sched_yield();

}


