#![cfg_attr(not(test), no_std)]

use shim::io;
use core::fmt;

pub struct EarlyWriter<'a>(&'a dyn EarlyPrintSerial);

impl fmt::Write for EarlyWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            if b == b'\n' {
                self.0.write_byte(b'\r');
            }
            self.0.write_byte(b);
        }
        Ok(())
    }
}

pub trait Arch {
    fn early_print(&self) -> &dyn EarlyPrintSerial;

    fn iter_memory_regions(&self, func: &mut dyn FnMut(u64, u64)) -> Result<(), &'static str>;

    // Provided

    fn early_writer(&self) -> EarlyWriter {
        EarlyWriter(self.early_print())
    }
}

pub trait EarlyPrintSerial : Sync {
    // Required

    fn has_byte(&self) -> bool;
    fn read_byte(&self) -> u8;

    fn can_send(&self) -> bool;
    fn write_byte(&self, b: u8);

    fn set_send_interrupt_enabled(&self, enabled: bool) {
    }

    // Provided

    fn read_nonblocking(&self, buf: &mut [u8]) -> io::Result<usize> {
        for i in 0..buf.len() {
            if !self.has_byte() {
                return Ok(i);
            }
            buf[i] = self.read_byte();
        }
        Ok(buf.len())
    }

    fn write_str(&self, s: &str) {
        for b in s.bytes() {
            self.write_byte(b);
        }
    }
}




