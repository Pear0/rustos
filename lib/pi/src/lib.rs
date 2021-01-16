#![feature(core_intrinsics)]
#![feature(const_fn)]
#![feature(asm)]
#![feature(decl_macro)]
#![feature(never_type)]
#![no_std]

#[macro_use]
extern crate aarch64;

use karch::EarlyPrintSerial;

use crate::atags::ATAG_BASE;
use crate::common::IO_BASE;
use crate::uart::MiniUart;

pub mod atags;
pub mod common;
pub mod dma;
pub mod gpio;
pub mod interrupt;
pub mod mbox;
pub mod pm;
pub mod timer;
pub mod types;
pub mod uart;
pub mod usb;

pub struct PiArch {
    printer: MiniUart,
}

impl PiArch {
    pub fn new() -> Option<PiArch> {
        let ptr = (ATAG_BASE + 4) as *const u32;
        if unsafe { ptr.read() } != atags::raw::Atag::CORE {
            return None;
        }

        Some(PiArch {
            printer: MiniUart::new(),
        })
    }
}

impl karch::Arch for PiArch {
    fn early_print(&self) -> &dyn EarlyPrintSerial {
        &self.printer
    }

    fn iter_memory_regions(&self, func: &mut dyn FnMut(u64, u64)) -> Result<(), &'static str> {
        func(0, IO_BASE as u64);
        Ok(())
    }
}


