use crate::{ID_AA64ISAR0_EL1, ID_AA64ISAR1_EL1};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Attribute {
    AES,
    SHA1,
    SHA2,
    CRC32, // ref. C3.4.8
    ATOMIC,
    RDM,
    DPB,
}

const ALL_ATTRIBUTES: [Attribute; 7] = {
    use Attribute::*;
    [AES, SHA1, SHA2, CRC32, ATOMIC, RDM, DPB]
};

impl Attribute {
    pub fn enabled(&self) -> bool {
        unsafe {
            match self {
                Attribute::AES => ID_AA64ISAR0_EL1.get_value(ID_AA64ISAR0_EL1::AES) != 0,
                Attribute::SHA1 => ID_AA64ISAR0_EL1.get_value(ID_AA64ISAR0_EL1::SHA1) != 0,
                Attribute::SHA2 => ID_AA64ISAR0_EL1.get_value(ID_AA64ISAR0_EL1::SHA2) != 0,
                Attribute::CRC32 => ID_AA64ISAR0_EL1.get_value(ID_AA64ISAR0_EL1::CRC32) != 0,
                Attribute::ATOMIC => ID_AA64ISAR0_EL1.get_value(ID_AA64ISAR0_EL1::ATOMIC) != 0,
                Attribute::RDM => ID_AA64ISAR0_EL1.get_value(ID_AA64ISAR0_EL1::RDM) != 0,
                Attribute::DPB => ID_AA64ISAR1_EL1.get_value(ID_AA64ISAR1_EL1::DPB) != 0,
            }
        }
    }
}

pub fn iter_enabled() -> impl Iterator<Item = Attribute> {
    ALL_ATTRIBUTES.iter().map(|x| *x).filter(|a| a.enabled())
}
