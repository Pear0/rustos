use core::fmt;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct TrapFrame {
    pub elr: u64,
    pub spsr: u64,
    pub sp: u64,
    pub tpidr: u64,
    pub simd: [u128; 32],
    pub regs: [u64; 31],
}

