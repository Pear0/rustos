use aarch64::ESR_EL1;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Fault {
    AddressSize,
    Translation,
    AccessFlag,
    Permission,
    Alignment,
    TlbConflict,
    Other(u8),
}

impl From<u32> for Fault {
    fn from(val: u32) -> Fault {
        use self::Fault::*;


        match (val & 0b1111_00) >> 2 {
            0b0000 => AddressSize,
            0b0001 => Translation,
            0b0010 => AccessFlag,
            0b0011 => Permission,
            0b1000 => Alignment,
            0b1100 => TlbConflict,
            e => Other(e as u8)
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AbortInfo {
    pub kind: Fault,
    pub level: u8,

    pub far_not_valid: bool,
    pub s1ptw: bool, // Fault on the stage 2 translation of an access for a stage 1 translation table walk
}

impl From<u32> for AbortInfo {
    fn from(esr: u32) -> Self {
        Self {
            kind: Fault::from(esr),
            level: (esr & 0b11) as u8,
            far_not_valid: (esr >> 10) & 0b1 != 0,
            s1ptw: (esr >> 7) & 0b1 != 0,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Syndrome {
    Unknown,
    WfiWfe,
    SimdFp,
    IllegalExecutionState,
    Svc(u16),
    Hvc(u16),
    Smc(u16),
    MsrMrsSystem,
    InstructionAbort(AbortInfo),
    PCAlignmentFault,
    DataAbort(AbortInfo),
    SpAlignmentFault,
    TrappedFpu,
    SError,
    Breakpoint,
    Step,
    Watchpoint,
    Brk(u16),
    Other(u32),
}

/// Converts a raw syndrome value (ESR) into a `Syndrome` (ref: D1.10.4).
impl From<u32> for Syndrome {
    fn from(esr: u32) -> Syndrome {
        use self::Syndrome::*;

        match ESR_EL1::get_value(esr as u64, ESR_EL1::EC) {
            0b000_000 => Unknown,
            0b000_001 => WfiWfe,
            0b000_111 => SimdFp,
            0b001_110 => IllegalExecutionState,
            0b010101 => Svc(ESR_EL1::get_value(esr as u64, ESR_EL1::ISS_HSVC_IMM) as u16),
            0b010110 => Hvc(ESR_EL1::get_value(esr as u64, ESR_EL1::ISS_HSVC_IMM) as u16),
            0b010111 => Smc(ESR_EL1::get_value(esr as u64, ESR_EL1::ISS_HSVC_IMM) as u16),
            0b011000 => MsrMrsSystem,
            0b100000 => InstructionAbort(AbortInfo::from(esr)),
            0b100001 => InstructionAbort(AbortInfo::from(esr)),
            0b100010 => PCAlignmentFault,
            0b100100 => DataAbort(AbortInfo::from(esr)),
            0b100101 => DataAbort(AbortInfo::from(esr)),
            0b100110 => SpAlignmentFault,
            0b101100 => TrappedFpu,
            0b101111 => SError,
            0b110000 => Breakpoint,
            0b110001 => Breakpoint,
            0b110010 => Step,
            0b110011 => Step,
            0b110100 => Watchpoint,
            0b110101 => Watchpoint,

            0b111100 => Brk(ESR_EL1::get_value(esr as u64, ESR_EL1::ISS_BRK_CMMT) as u16),

            e => Other(e as u32)
        }
    }
}
