#![feature(asm)]
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

pub use sp::SP;
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


