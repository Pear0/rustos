#![feature(llvm_asm)]
#![feature(global_asm)]
#![allow(unused_imports)]

#![cfg_attr(not(test), no_std)]

#[macro_use]
extern crate log;

use karch::EarlyPrintSerial;

pub mod gpio;
pub mod irq;
pub mod uart;

#[repr(C, align(4))]
struct MyAlignedBuffer([u8; 128]);

impl MyAlignedBuffer {
    pub fn new() -> Self {
        MyAlignedBuffer([0; 128])
    }
}
pub struct KhadasArch {
    device_tree_base: u64,
    early_print: KhadasEarlyPrint,
}

impl KhadasArch {
    pub fn new(device_tree_base: u64) -> Option<Self> {
        Some(Self {
            device_tree_base,
            early_print: KhadasEarlyPrint(),
        })
    }

    pub fn dtb_reader(&self) -> Result<dtb::Reader, &'static str> {
        let addr = self.device_tree_base;
        if addr == 0 {
            return Err("device_tree_base cannot be zero");
        }

        unsafe {
            match dtb::Reader::read_from_address(addr as usize) {
                Ok(reader) => Ok(reader),
                Err(_) => Err("failed to read device tree")
            }
        }
    }

}

impl karch::Arch for KhadasArch {

    fn early_print(&self) -> &dyn EarlyPrintSerial {
        &self.early_print
    }

    fn iter_memory_regions(&self, func: &mut dyn FnMut(u64, u64)) -> Result<(), &'static str> {
        let reader = self.dtb_reader()?;

        let (entry, _) = reader.struct_items()
            .path_struct_items("/memory/reg")
            .next().ok_or("no device tree entry /memory/reg")?;

        let mut scratch = MyAlignedBuffer::new();

        let entries = entry.value_u32_list(&mut scratch.0).map_err(|_| "failed to read u32 list from /memory/reg")?;

        for pair in entries.chunks(2) {
            if pair.len() == 2 && pair[1] != 0 {
                func(pair[0] as u64, pair[1] as u64);
            }
        }

        Ok(())
    }
}

pub struct KhadasEarlyPrint();

impl karch::EarlyPrintSerial for KhadasEarlyPrint {
    fn has_byte(&self) -> bool {
        uart::has_byte()
    }

    fn read_byte(&self) -> u8 {
        uart::read_byte()
    }

    fn can_send(&self) -> bool {
        true
    }

    fn write_byte(&self, b: u8) {
        uart::print_char(b);
    }
}



