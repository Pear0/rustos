use core::fmt;
use core::fmt::Error;
use core::time::Duration;

use shim::const_assert_size;
use shim::io;
use volatile::{ReadVolatile, Reserved, Volatile};
use volatile::prelude::*;

use crate::{interrupt, timer};
use crate::common::IO_BASE;
use crate::gpio::{Function, Gpio};
use crate::interrupt::Interrupt;
use crate::uart::LsrStatus::{DataReady, TxAvailable};

/// The base address for the `MU` registers.
const MU_REG_BASE: usize = IO_BASE + 0x215040;

/// The `AUXENB` register from page 9 of the BCM2837 documentation.
const AUX_ENABLES: *mut Volatile<u8> = (IO_BASE + 0x215004) as *mut Volatile<u8>;

/// Enum representing bit fields of the `AUX_MU_LSR_REG` register.
#[repr(u8)]
enum LsrStatus {
    DataReady = 1,
    TxAvailable = 1 << 5,
}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    IO_REG: Volatile<u8>,
    __r0: [Reserved<u8>; 3],
    IER_REG: Volatile<u8>,
    __r1: [Reserved<u8>; 3],
    IIR_REG: Volatile<u8>,
    __r2: [Reserved<u8>; 3],
    LCR_REG: Volatile<u8>,
    __r3: [Reserved<u8>; 3],
    MCR_REG: Volatile<u8>,
    __r4: [Reserved<u8>; 3],
    LSR_REG: Volatile<u8>,
    __r5: [Reserved<u8>; 3],
    MSR_REG: Volatile<u8>,
    __r6: [Reserved<u8>; 3],
    SCRATCH: Volatile<u8>,
    __r7: [Reserved<u8>; 3],
    CNTL_REG: Volatile<u8>,
    __r8: [Reserved<u8>; 3],
    STAT_REG: Volatile<u32>,
    BAUD_REG: Volatile<u16>,
    __r9: [Reserved<u8>; 2],
}

const_assert_size!(Registers, 44);

/// The Raspberry Pi's "mini UART".
pub struct MiniUart {
    registers: &'static mut Registers,
    timeout: Option<Duration>,
}

unsafe impl Sync for MiniUart {}

impl MiniUart {
    /// Initializes the mini UART by enabling it as an auxiliary peripheral,
    /// setting the data size to 8 bits, setting the BAUD rate to ~115200 (baud
    /// divider of 270), setting GPIO pins 14 and 15 to alternative function 5
    /// (TXD1/RDXD1), and finally enabling the UART transmitter and receiver.
    ///
    /// By default, reads will never time out. To set a read timeout, use
    /// `set_read_timeout()`.
    pub fn new_opt_init(do_init: bool) -> MiniUart {
        let registers = unsafe {
            // Enable the mini UART as an auxiliary device.
            &mut *(MU_REG_BASE as *mut Registers)
        };

        if do_init {
            unsafe { (*AUX_ENABLES).or_mask(1); }

            // interrupt::Controller::new().enable(Interrupt::Uart);

            // ref: https://github.com/bztsrc/raspi3-tutorial/blob/master/03_uart1/uart.c

            registers.CNTL_REG.write(0);
            registers.LCR_REG.write(0b11); // 8 bit mode
//        registers.MCR_REG.write(0);
            registers.IER_REG.write(0b1111_1111); // receive interrupts / no transmit

//        registers.IIR_REG.write(0xc6); // disable interrupts

            registers.BAUD_REG.write(270); // 151200 baud

            Gpio::new(14).into_alt(Function::Alt5);
            Gpio::new(15).into_alt(Function::Alt5);

            registers.CNTL_REG.write(0b11); //enable receiver / transmitter
        }

        MiniUart {
            registers,
            timeout: None,
        }
    }

    pub fn new() -> Self {
        Self::new_opt_init(true)
    }

    fn unsafe_mut(&self) -> &mut Self {
        unsafe { &mut *(self as *const MiniUart as *mut MiniUart)  }
    }

    /// Set the read timeout to `t` duration.
    pub fn set_read_timeout(&mut self, t: Duration) {
        self.timeout = Some(t)
    }

    pub fn can_send(&self) -> bool {
        (self.registers.LSR_REG.read() & (TxAvailable as u8)) != 0
    }

    pub fn set_send_interrupt_enabled(&self, enabled: bool) {

        let mut val = self.unsafe_mut().registers.IER_REG.read();
        if enabled {
            val |= 0b10;
        } else {
            val &= !0b10;
        }
        self.unsafe_mut().registers.IER_REG.write(val);

    }

    /// Write the byte `byte`. This method blocks until there is space available
    /// in the output FIFO.
    pub fn write_byte(&self, byte: u8) {
        while !self.can_send() {}
        self.unsafe_mut().registers.IO_REG.write(byte);
    }

    /// Returns `true` if there is at least one byte ready to be read. If this
    /// method returns `true`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately. This method does not block.
    pub fn has_byte(&self) -> bool {
        (self.registers.LSR_REG.read() & (DataReady as u8)) != 0
    }

    /// Blocks until there is a byte ready to read. If a read timeout is set,
    /// this method blocks for at most that amount of time. Otherwise, this
    /// method blocks indefinitely until there is a byte to read.
    ///
    /// Returns `Ok(())` if a byte is ready to read. Returns `Err(())` if the
    /// timeout expired while waiting for a byte to be ready. If this method
    /// returns `Ok(())`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately.
    pub fn wait_for_byte(&self) -> Result<(), ()> {
        match self.timeout {
            Some(timeout) => {
                let end = timer::current_time() + timeout;

                while !self.has_byte() && timer::current_time() < end {}

                if timer::current_time() < end {
                    Ok(())
                } else {
                    Err(())
                }
            }
            None => {
                while !self.has_byte() {}
                Ok(())
            }
        }
    }

    /// Reads a byte. Blocks indefinitely until a byte is ready to be read.
    pub fn read_byte(&self) -> u8 {
        while !self.has_byte() {}
        self.unsafe_mut().registers.IO_REG.read()
    }

    pub fn read_nonblocking(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for i in 0..buf.len() {
            if !self.has_byte() {
                return Ok(i);
            }
            buf[i] = self.read_byte();
        }
        Ok(buf.len())
    }
}

impl karch::EarlyPrintSerial for MiniUart {
    fn has_byte(&self) -> bool {
        MiniUart::has_byte(self)
    }

    fn read_byte(&self) -> u8 {
        MiniUart::read_byte(self)
    }

    fn can_send(&self) -> bool {
        MiniUart::can_send(self)
    }

    fn write_byte(&self, b: u8) {
        MiniUart::write_byte(self, b)
    }

    fn set_send_interrupt_enabled(&self, enabled: bool) {
        MiniUart::set_send_interrupt_enabled(self, enabled)
    }
}

// FIXME: Implement `fmt::Write` for `MiniUart`. A b'\r' byte should be written
// before writing any b'\n' byte.

impl fmt::Write for MiniUart {
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        for byte in s.as_bytes().iter() {
            if *byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(*byte);
        }
        Ok(())
    }
}

pub mod uart_io {
    use shim::ioerr;
    use volatile::prelude::*;

    use super::io;
    use super::MiniUart;

    impl io::Write for MiniUart {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            for byte in buf.iter() {
                self.write_byte(*byte);
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl io::Read for MiniUart {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            match self.wait_for_byte() {
                Ok(_) => {
                    for i in 0..buf.len() {
                        if !self.has_byte() {
                            return Ok(i);
                        }
                        buf[i] = self.read_byte();
                    }
                    Ok(buf.len())
                }
                Err(_) => ioerr!(TimedOut, "read timed out"),
            }
        }
    }

    // The `io::Read::read()` implementation must respect the read timeout by
    // waiting at most that time for the _first byte_. It should not wait for
    // any additional bytes but _should_ read as many bytes as possible. If the
    // read times out, an error of kind `TimedOut` should be returned.
    //
    // The `io::Write::write()` method must write all of the requested bytes
    // before returning.
}
