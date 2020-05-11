use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::time::Duration;

use aarch64::{MPIDR_EL1, SP};

use crate::{init, BootVariant};
use crate::mutex::Mutex;
use crate::console::{CONSOLE, console_flush};
use crate::mbox::EveryTimer;

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

    pi::timer::spin_sleep(Duration::from_millis(10 * core_id));
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
        unsafe { ((parking_base + core_id * 8) as *mut u64).write_volatile(core_bootstrap as u64) };
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
                pi::timer::spin_sleep(Duration::from_micros(10));
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

pub fn no_interrupt<T, R>(func: T) -> R
    where T: (FnOnce() -> R) {
    use aarch64::regs::*;

    let int = DAIF::D | DAIF::A | DAIF::I | DAIF::F;

    unsafe {
        let orig = DAIF.get_masked(int);
        DAIF.set(int);
        aarch64::dsb();
        let r = func();
        aarch64::dsb();
        DAIF.set(orig);
        r
    }
}











