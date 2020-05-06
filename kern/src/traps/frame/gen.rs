// Auto-generated. Do not edit
#![allow(non_snake_case)]

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct KernelTrapFrame {
    pub ELR_EL1: u64,
    pub SPSR_EL1: u64,
    pub SP_EL0: u64,
    pub TPIDR_EL0: u64,
    pub TTBR0_EL1: u64,
    pub TTBR1_EL1: u64,
    pub simd: [u128; 32],
    pub regs: [u64; 31],
    __res1: u64,
}

const_assert_size!(KernelTrapFrame, 816);

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct HyperTrapFrame {
    pub ELR_EL2: u64,
    pub SPSR_EL2: u64,
    pub SP_EL0: u64,
    pub TPIDR_EL0: u64,
    pub SP_EL1: u64,
    pub TPIDR_EL2: u64,
    pub VTTBR_EL2: u64,
    pub HCR_EL2: u64,
    pub simd: [u128; 32],
    pub regs: [u64; 31],
    __res1: u64,
}

const_assert_size!(HyperTrapFrame, 832);

