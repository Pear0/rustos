use core::mem::zeroed;
use core::ptr::write_volatile;
use core::sync::atomic::{AtomicU64, Ordering};

use aarch64::*;

use crate::kmain;
use crate::param::*;

mod oom;
mod panic;

global_asm!(include_str!("init/vectors.s"));

//
// big assumptions (better to be checked):
//   _start1/2(), _kinit1/2(), switch_to_el1/2() should NOT use stack!
//   e.g., #[no_stack] would be useful ..
//
// so, no debug build support!
//

pub static SAFE_ALLOC_START: AtomicU64 = AtomicU64::new(0);
pub static EL2_KERNEL_INIT: AtomicU64 = AtomicU64::new(0);
pub static EL2_KERNEL_INIT_LEN: AtomicU64 = AtomicU64::new(0);

/// FIXME lmao i needed this
#[link_section = ".text.init"]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    if MPIDR_EL1.get_value(MPIDR_EL1::Aff0) == 0 {
        SP.set(KERN_STACK_BASE);
        kinit()
    }

    loop {
        aarch64::wfe();
    }
    // unreachable!()
}

unsafe fn zeros_bss() {
    extern "C" {
        static mut __bss_beg: u64;
        static mut __bss_end: u64;
    }

    let mut iter: *mut u64 = &mut __bss_beg;
    let end: *mut u64 = &mut __bss_end;

    while iter < end {
        write_volatile(iter, zeroed());
        iter = iter.add(1);
    }
}

#[no_mangle]
#[inline(never)]
pub unsafe fn switch_to_el2() {
    if current_el() == 3 {
        // set up Secure Configuration Register (D13.2.10)
        SCR_EL3.set(SCR_EL3::NS | SCR_EL3::SMD | SCR_EL3::HCE | SCR_EL3::RW | SCR_EL3::RES1);

        // set up Saved Program Status Register (C5.2.19)
        SPSR_EL3
            .set((SPSR_EL3::M & 0b1001) | SPSR_EL3::F | SPSR_EL3::I | SPSR_EL3::A | SPSR_EL3::D);

        // eret to itself, expecting current_el() == 2 this time.
        ELR_EL3.set(switch_to_el2 as u64);
        asm::eret();
    }
}

#[no_mangle]
#[inline(never)]
pub unsafe fn switch_to_el1() {
    extern "C" {
        static mut kernel_vectors: u64;
    }

    if current_el() == 2 {
        // set the stack-pointer for EL1
        SP_EL1.set(SP.get() as u64);

        // enable CNTP for EL1/EL0 (ref: D7.5.2, D7.5.13)
        // NOTE: This doesn't actually enable the counter stream.
        CNTHCTL_EL2.set(CNTHCTL_EL2.get() | CNTHCTL_EL2::EL1PCEN | CNTHCTL_EL2::EL1PCTEN);
        CNTVOFF_EL2.set(0);

        // enable AArch64 in EL1 (A53: 4.3.36)
        HCR_EL2.set(HCR_EL2::RW | HCR_EL2::RES1);

        // enable floating point and SVE (SIMD) (A53: 4.3.38, 4.3.34)
        CPTR_EL2.set(0);
        CPACR_EL1.set(CPACR_EL1.get() | (0b11 << 20));

        // Set SCTLR to known state (A53: 4.3.30)
        SCTLR_EL1.set(SCTLR_EL1::RES1);

        // set up exception handlers
        VBAR_EL1.set((&kernel_vectors) as *const u64 as u64);

        // change execution level to EL1 (ref: C5.2.19)
        SPSR_EL2.set(
            (SPSR_EL2::M & 0b0101) // EL1h
                | SPSR_EL2::F
                | SPSR_EL2::I
                | SPSR_EL2::D
                | SPSR_EL2::A,
        );

        ELR_EL2.set(switch_to_el1 as u64);
        // ELR_EL1.set(switch_to_el1 as u64);
        aarch64::eret();
    }
}

#[no_mangle]
#[inline(never)]
unsafe fn el1_init() {
    if current_el() != 1 {
        return;
    }

    #[allow(non_upper_case_globals)]
    extern "C" {
        static mut __text_end: u64;
        static mut kernel_vectors: u64;
    }

    VBAR_EL1.set((&kernel_vectors) as *const u64 as u64);

    DAIF.set(DAIF::D | DAIF::A | DAIF::I | DAIF::F);

    SAFE_ALLOC_START.store((&__text_end) as *const u64 as u64, Ordering::Relaxed);
    aarch64::dmb();
}

#[no_mangle]
#[inline(never)]
unsafe fn el2_init() {
    if current_el() != 2 {
        return;
    }

    #[allow(non_upper_case_globals)]
    extern "C" {
        static mut __text_beg: u64;
        static mut __text_end: u64;
        static mut hyper_vectors: u64;
    }

    // set up exception handlers
    VBAR_EL2.set((&hyper_vectors) as *const u64 as u64);

    // Make a copy of all the code and data very early before and variables
    // in the data section are touched.

    let text_beg = (&__text_beg) as *const u64 as u64;
    let text_end = (&__text_end) as *const u64 as u64;
    let text_len = text_end - text_beg;

    core::ptr::copy_nonoverlapping(text_beg as *const u8, text_end as *mut u8, text_len as usize);

    aarch64::dsb();

    EL2_KERNEL_INIT.store(text_end, Ordering::Relaxed);
    EL2_KERNEL_INIT_LEN.store(text_len, Ordering::Relaxed);
    SAFE_ALLOC_START.store(text_end + text_len, Ordering::Relaxed);

    aarch64::dmb();
}

#[no_mangle]
unsafe fn kinit() -> ! {
    zeros_bss();
    switch_to_el2();

    // for now, always boot hypervisor
    if current_el() == 2 {
        el2_init();
        kmain(true);
    } else {
        switch_to_el1();
        el1_init();
        kmain(false);
    }
}
