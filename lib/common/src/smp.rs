use aarch64::{MPIDR_EL1};

pub fn core() -> usize {
    unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize }
}

