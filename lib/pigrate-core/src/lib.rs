#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[macro_use]
extern crate serde;

pub mod bundle;

use fat32::util::SliceExt;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct FakeTrapFrame {
    pub elr: u64,
    pub spsr: u64,
    pub sp: u64,
    pub tpidr: u64,
    pub ttbr0: u64,
    pub ttbr1: u64,
    pub simd: [u128; 32],
    pub regs: [u64; 31],
}

impl FakeTrapFrame {

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_ref(self).cast() }
    }

    pub fn decode_from_bytes(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if core::mem::size_of::<Self>() != bytes.len() {
            return Err(());
        }
        unsafe { core::slice::from_mut(self).cast_mut::<u8>() }.copy_from_slice(bytes);
        Ok(())
    }

}

