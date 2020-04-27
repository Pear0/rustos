
mod internal {
    defbit32!(ESR_DataAbort, [
        ISV [24-24],
        SAS [23-22],
        SSE [21-21],
        SRT [20-16],
        SF [15-15],
        AR [14-14],

        WnR [6-6],
    ]);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AccessSize {
    Byte,
    HalfWord, // 2 bytes
    Word, // 4 bytes
    DoubleWord, // 8 bytes
}

impl From<u32> for AccessSize {
    fn from(num: u32) -> Self {
        match num {
            0b00 => AccessSize::Byte,
            0b01 => AccessSize::HalfWord,
            0b10 => AccessSize::Word,
            0b11 => AccessSize::DoubleWord,
            _ => panic!("invalid num"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataAccess {
    pub access_size: AccessSize,
    pub sign_extend: bool,
    pub register_idx: usize,
    pub is_64: bool, // true = 64 bit, false = 32 bit
    pub acquire_release: bool, // has acquire release semantics
    pub write: bool,
}

impl DataAccess {
    pub fn parse_esr(esr: u32) -> Option<DataAccess> {
        use internal::*;
        let esr = ESR_DataAbort::new(esr);

        if esr.get_value(ESR_DataAbort::ISV) == 0 {
            return None;
        }

        Some(Self {
            access_size: AccessSize::from(esr.get_value(ESR_DataAbort::SAS)),
            sign_extend: esr.get_value(ESR_DataAbort::SSE) != 0,
            register_idx: esr.get_value(ESR_DataAbort::SRT) as usize,
            is_64: esr.get_value(ESR_DataAbort::SF) != 0,
            acquire_release: esr.get_value(ESR_DataAbort::AR) != 0,
            write: esr.get_value(ESR_DataAbort::WnR) != 0,
        })
    }
}



