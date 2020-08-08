#![feature(llvm_asm)]
#![feature(global_asm)]

#![cfg_attr(not(test), no_std)]

#[macro_use]
pub mod macros;

pub mod sp;
pub mod asm;
pub mod attr;
pub mod regs;
pub mod semi;
pub mod vmsa;

pub use sp::{SP, LR};
pub use regs::*;
pub use vmsa::*;
pub use asm::*;

/// Returns the current exception level.
///
/// # Safety
/// This function should only be called when EL is >= 1.
#[inline(always)]
pub unsafe fn current_el() -> u8 {
    ((CurrentEL.get() & 0b1100) >> 2) as u8
}

/// Returns the SPSel value.
#[inline(always)]
pub fn sp_sel() -> u8 {
    unsafe {
        SPSel.get_value(SPSel::SP) as u8
    }
}

/// Returns the core currently executing.
///
/// # Safety
///
/// This function should only be called when EL is >= 1.
pub fn affinity() -> usize {
    unsafe {
        MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize
    }
}


pub fn far_ipa() -> u64 {
    // assumes 64kb pages
    let offset = 4;
    unsafe {
        ((HPFAR_EL2.get_value(HPFAR_EL2::FIPA) & !0xF) << (8 + offset)) + (FAR_EL2.get() & 0xFFFF)
    }
}


pub fn clean_data_cache(addr: u64) {
    dsb();
    unsafe {
        llvm_asm!("dc cvac, $0
              dsb ish
             " :: "r"(addr) :: "volatile");
    }
}

pub fn clean_data_cache_region(mut addr: u64, length: u64) {
    dsb();
    unsafe {
        let mut end = addr + length;
        addr &= !(64 - 1);

        // round end up to the cache line.
        if end % 64 != 0 {
            end += 64;
        }
        addr &= !(64 - 1);

        for i in (addr..end).step_by(64) {
            llvm_asm!("dc cvac, $0" :: "r"(i) :: "volatile");
        }

        llvm_asm!("dsb ish" :::: "volatile");
    }
}

pub fn invalidate_data_cache_region(mut addr: u64, length: u64) {
    dsb();
    unsafe {
        let mut end = addr + length;
        addr &= !(64 - 1);

        // round end up to the cache line.
        if end % 64 != 0 {
            end += 64;
        }
        addr &= !(64 - 1);

        for i in (addr..end).step_by(64) {
            llvm_asm!("dc ivac, $0" :: "r"(i) :: "volatile");
        }

        llvm_asm!("dsb ish" :::: "volatile");
    }
}

pub fn clean_and_invalidate_data_cache_region(mut addr: u64, length: u64) {
    dsb();
    unsafe {
        let mut end = addr + length;
        addr &= !(64 - 1);

        // round end up to the cache line.
        if end % 64 != 0 {
            end += 64;
        }
        addr &= !(64 - 1);

        for i in (addr..end).step_by(64) {
            llvm_asm!("dc civac, $0" :: "r"(i) :: "volatile");
        }

        llvm_asm!("dsb ish" :::: "volatile");
    }
}

pub fn clean_data_cache_obj<T: ?Sized>(item: &T) {
    clean_data_cache_region(item as *const T as *const () as u64, core::mem::size_of_val(item) as u64);
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Implementor {
    ARM,
    Broadcom,
    Other(u8),
}

impl Implementor {
    pub fn hardware() -> Self {
        (unsafe { MIDR_EL1.get_value(MIDR_EL1::IMPL) } as u8).into()
    }
}

impl From<u8> for Implementor {
    fn from(a: u8) -> Self {
        use Implementor::*;
        match a {
            b'A' => ARM,
            b'B' => Broadcom,
            o => Other(o),
        }
    }
}


