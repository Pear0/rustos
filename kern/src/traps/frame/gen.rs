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
    pub ELR_EL1: u64,
    pub FPCR: u64,
    pub FPSR: u64,
    pub SP_EL0: u64,
    pub SP_EL1: u64,
    pub SPSR_EL1: u64,
    pub SPSR_abt: u64,
    pub SPSR_fiq: u64,
    pub SPSR_irq: u64,
    pub SPSR_und: u64,
    pub ACTLR_EL1: u64,
    pub AFSR0_EL1: u64,
    pub AFSR1_EL1: u64,
    pub AMAIR_EL1: u64,
    pub CONTEXTIDR_EL1: u64,
    pub CPACR_EL1: u64,
    pub CPTR_EL2: u64,
    pub CSSELR_EL1: u64,
    pub ESR_EL1: u64,
    pub FAR_EL1: u64,
    pub MAIR_EL1: u64,
    pub PAR_EL1: u64,
    pub SCTLR_EL1: u64,
    pub TCR_EL1: u64,
    pub TPIDR_EL0: u64,
    pub TPIDR_EL1: u64,
    pub TPIDRRO_EL0: u64,
    pub TTBR0_EL1: u64,
    pub TTBR1_EL1: u64,
    pub VBAR_EL1: u64,
    pub CNTKCTL_EL1: u64,
    pub CNTP_CTL_EL0: u64,
    pub CNTP_CVAL_EL0: u64,
    pub CNTV_CTL_EL0: u64,
    pub CNTV_CVAL_EL0: u64,
    pub CNTVOFF_EL2: u64,
    pub ELR_EL2: u64,
    pub SPSR_EL2: u64,
    pub HCR_EL2: u64,
    pub VTTBR_EL2: u64,
    pub TPIDR_EL2: u64,
    __res0: u64,
    pub simd: [u128; 32],
    pub regs: [u64; 31],
    __res1: u64,
}

const_assert_size!(HyperTrapFrame, 1104);

