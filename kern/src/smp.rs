use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::time::Duration;

use aarch64::{MPIDR_EL1, SP};

use crate::init;
use crate::mutex::Mutex;

pub const MAX_CORES: usize = 4;

// utilities to handle secondary cores initialization.

const stack_size: usize = 1024 * 32;

type ParkingSpot = (AtomicU64, [u8; stack_size], AtomicBool);

const fn parking() -> ParkingSpot {
    (AtomicU64::new(0), [0; stack_size], AtomicBool::new(false))
}

static PARKING: [ParkingSpot; 4] = [parking(), parking(), parking(), parking()];

// must not use stack
#[inline(never)]
pub unsafe fn core_bootstrap() -> ! {
    let core_id = MPIDR_EL1.get_value(MPIDR_EL1::Aff0);

    let mut stack_top = ((PARKING[core_id as usize].1.as_ptr() as usize) + stack_size);
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
    init::switch_to_el1();
    //
    // let el = unsafe { aarch64::current_el() };
    // kprintln!("Big Current EL: {}", el);


    let core_id = MPIDR_EL1.get_value(MPIDR_EL1::Aff0);

    PARKING[core_id as usize].2.store(true, Ordering::SeqCst);

    pi::timer::spin_sleep(Duration::from_millis(10 * core_id));
    // kprintln!("bootstrap @ {} {}", core_id, MPIDR_EL1.get_value(MPIDR_EL1::Aff0));

    loop {
        unsafe { asm!("dsb sy" ::: "memory"); }
        let core_id = MPIDR_EL1.get_value(MPIDR_EL1::Aff0);

        let func = PARKING[core_id as usize].0.load(Ordering::SeqCst);
        if func != 0 {
            core::mem::transmute::<u64, fn()>(func)();
            unsafe { asm!("dsb sy" ::: "memory"); }
            PARKING[core_id as usize].0.store(0, Ordering::SeqCst);
            unsafe { asm!("dsb sy" ::: "memory"); }
            aarch64::sev();
        }

        // let mut did_work = false;
        // if let Some(mut lock) = PARKING[core_id as usize].0.try_lock() {
        //     if let Some(func) = lock.as_ref() {
        //         func();
        //         lock.take();
        //         did_work = true;
        //         unsafe { asm!("dsb sy" ::: "memory"); }
        //     }
        // }
        // if did_work {
        //     unsafe { asm!("dsb sy"); }
        //     aarch64::sev();
        // }
        aarch64::wfe();
    }
}

pub fn core_stack_top() -> u64 {
    let core_id = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) };

    let mut stack_top = ((PARKING[core_id as usize].1.as_ptr() as usize) + stack_size);
    stack_top -= stack_top % 16; // align to 16
    stack_top as u64
}

pub unsafe fn initialize(cores: usize) {
    let parking_base = 0xd8;

    for core_id in 1..cores {
        unsafe { ((parking_base + core_id * 8) as *mut u64).write_volatile(core_bootstrap as u64) };
    }
    aarch64::sev();
}

pub fn count_cores() -> usize {
    let mut count = 0;
    for core in PARKING.iter() {
        if core.2.load(Ordering::SeqCst) {
            count += 1;
        }
    }
    count
}

pub fn wait_for_cores(cores: usize) {
    while count_cores() < cores - 1 {
        aarch64::sev();
    }
}

pub fn run_no_return(func: fn()) {
    for (id, core) in PARKING.iter().enumerate() {
        if core.2.load(Ordering::SeqCst) {
            // let mut lock = core.0.lock();
            // lock.replace(func);

            core.0.store(func as u64, Ordering::SeqCst);
        }
    }

    unsafe { asm!("dsb sy" ::: "memory"); }
    aarch64::sev();
}

pub fn run_on_secondary_cores(func: fn()) {
    let mut enables = [false; 4];

    for (id, core) in PARKING.iter().enumerate() {
        if core.2.load(Ordering::SeqCst) {
            enables[id] = true;
            // kprintln!("enabled @ {}", id);
            // let mut lock = core.0.lock();
            // lock.replace(func);
            unsafe { asm!("dsb sy" ::: "memory"); }
            core.0.store(func as u64, Ordering::SeqCst);
            unsafe { asm!("dsb sy" ::: "memory"); }
        }
    }

    unsafe { asm!("dsb sy" ::: "memory"); }
    aarch64::sev();

    'wait: loop {
        pi::timer::spin_sleep(Duration::from_micros(10));

        for (id, core) in PARKING.iter().enumerate() {
            unsafe { asm!("dsb sy" ::: "memory"); }
            if enables[id] {
                if core.0.load(Ordering::SeqCst) != 0 {
                    unsafe { asm!("dsb sy" ::: "memory"); }
                    // kprintln!("waiting @ {}", id);
                    continue 'wait;
                }
            }
        }

        break;
    }
}

pub fn run_on_all_cores(func: fn()) {

    let mut enables = [false; 4];

    for (id, core) in PARKING.iter().enumerate() {
        if core.2.load(Ordering::SeqCst) {
            enables[id] = true;
            // kprintln!("enabled @ {}", id);
            // let mut lock = core.0.lock();
            // lock.replace(func);
            unsafe { asm!("dsb sy" ::: "memory"); }
            core.0.store(func as u64, Ordering::SeqCst);
            unsafe { asm!("dsb sy" ::: "memory"); }
        }
    }

    unsafe { asm!("dsb sy" ::: "memory"); }
    aarch64::sev();

    func();

    unsafe { asm!("dsb sy" ::: "memory"); }
    aarch64::sev();

    'wait: loop {
        pi::timer::spin_sleep(Duration::from_micros(10));

        for (id, core) in PARKING.iter().enumerate() {
            unsafe { asm!("dsb sy" ::: "memory"); }
            if enables[id] {
                if core.0.load(Ordering::SeqCst) != 0 {
                    unsafe { asm!("dsb sy" ::: "memory"); }
                    // kprintln!("waiting @ {}", id);
                    continue 'wait;
                }
            }
        }

        break;
    }
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











