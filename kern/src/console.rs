use alloc::boxed::Box;
use core::fmt;

use aarch64::SCTLR_EL1;
use pi::uart::MiniUart;
use shim::io;

use crate::{BootVariant, smp};
use crate::collections::CapacityRingBuffer;
use crate::fs::handle::{Sink, Source};
use crate::mutex::{Mutex, MutexGuard};

struct ConsoleImpl {
    send_buffer: CapacityRingBuffer<u8>,
    receive_buffer: CapacityRingBuffer<u8>,

    callback: Option<(u8, Box<dyn FnMut() -> bool + Send>)>,
}

impl ConsoleImpl {
    pub fn new() -> Self {
        Self {
            send_buffer: CapacityRingBuffer::new(1000),
            receive_buffer: CapacityRingBuffer::new(1000),
            callback: Some((0x02, Box::new(|| {
                error!("pressed Ctrl+B");
                true
            }))),
        }
    }
}

/// A global singleton allowing read/write access to the console.
pub struct Console {
    inner: Option<&'static dyn karch::EarlyPrintSerial>,
    ext: Option<ConsoleImpl>, // pieces that require an allocator
}

impl Console {
    /// Creates a new instance of `Console`.
    const fn new() -> Console {
        Console { inner: None, ext: None }
    }
}

impl MutexGuard<'_, Console> {
    /// Initializes the console if it's not already initialized.
    #[inline]
    fn initialize(&mut self) {
        if self.inner.is_none() {
            self.inner = Some(crate::hw::arch().early_print());
        }
    }

    /// Returns a mutable borrow to the inner `MiniUart`, initializing it as
    /// needed.
    fn inner(&mut self) -> &'static dyn karch::EarlyPrintSerial {
        self.initialize();
        self.inner.unwrap()
    }

    pub fn read_byte(&mut self) -> u8 {
        let mut byte: u8 = 0;
        loop {
            if let Ok(1) = self.read_nonblocking(core::slice::from_mut(&mut byte)) {
                return byte;
            }
        }
    }

    pub fn read_nonblocking(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let mut read = 0;
        if let Some(ext) = &mut self.ext {
            while let Some(byte) = ext.receive_buffer.remove() {
                if buf.len() == 0 {
                    return Ok(read);
                }
                buf[0] = byte;
                buf = &mut buf[1..];
                read += 1;
            }

            self.handle_receive();

            let ext = self.ext.as_mut().unwrap();

            while let Some(byte) = ext.receive_buffer.remove() {
                if buf.len() == 0 {
                    return Ok(read);
                }
                buf[0] = byte;
                buf = &mut buf[1..];
                read += 1;
            }

        } else {
            read = self.inner().read_nonblocking(buf).expect("MiniUart io::Error");
        }


        // if BootVariant::kernel_in_hypervisor() {
        //     read += self.inner().read_nonblocking(buf).expect("MiniUart io::Error");
        // }
        Ok(read)
    }

    pub fn has_byte(&mut self) -> bool {
        if let Some(ext) = &mut self.ext {
            if ext.receive_buffer.len() > 0 {
                return true;
            }
        }

        self.inner().has_byte()
    }

    fn send_and_update_interrupts(&mut self) {
        while self.inner().can_send() {
            if let Some(ext) = &mut self.ext {
                if let Some(byte) = ext.send_buffer.remove() {
                    self.inner().write_byte(byte);
                } else {
                    break; // buffer is empty.
                }
            }
        }

        if let Some(ext) = &mut self.ext {
            let enabled = ext.send_buffer.len() > 0;
            self.inner().set_send_interrupt_enabled(enabled);
        }
    }

    /// Writes the byte `byte` to the UART device.
    pub fn write_byte(&mut self, byte: u8) {

        // TODO inefficient waiting
        let mut inserted = false;
        while !inserted {
            if let Some(ext) = &mut self.ext {
                inserted = ext.send_buffer.insert(byte);
                self.send_and_update_interrupts();
            } else {
                self.inner().write_byte(byte);
                return; // MiniUart::write_byte() will wait for us.
            }
        }
    }

    pub fn flush(&mut self) {
        if let None = &self.ext {
            return;
        }
        while self.ext.as_ref().unwrap().send_buffer.len() > 0 {
            self.send_and_update_interrupts();
        }
    }

    fn set_callback(&mut self, callback: Option<(u8, Box<dyn FnMut() -> bool + Send>)>) {
        self.ext.as_mut().expect("impl not initialized").callback = callback;
    }

    fn handle_receive(&mut self) {
        if let None = &self.ext {
            return;
        }

        'read: while self.inner().has_byte() {
            // TODO may drop bytes on ground.
            let byte = self.inner().read_byte();

            if let Some((expected, func)) = &mut self.ext.as_mut().unwrap().callback {
                if byte == *expected {
                    let dont_remove = func();
                    if !dont_remove {
                        self.ext.as_mut().unwrap().callback.take();
                    }

                    continue 'read;
                }
            }

            self.ext.as_mut().unwrap().receive_buffer.insert(byte);
        }
    }

    fn interrupt_handle(&mut self) {
        if let None = &self.ext {
            return;
        }

        self.handle_receive();
        self.send_and_update_interrupts();
    }
}

impl io::Read for MutexGuard<'_, Console> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_nonblocking(buf)
    }
}

impl io::Write for MutexGuard<'_, Console> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for byte in buf.iter() {
            if *byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(*byte);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl fmt::Write for MutexGuard<'_, Console> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.as_bytes().iter() {
            if *byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(*byte);
        }
        Ok(())
    }
}

/// Global `Console` singleton.
pub static CONSOLE: Mutex<Console> = Mutex::new(Console::new());

pub fn console_ext_init() {
    smp::no_interrupt(|| {
        let mut lock = CONSOLE.lock();
        if lock.ext.is_none() {
            lock.ext = Some(ConsoleImpl::new());
        }
    });
}

pub fn console_interrupt_handler() {
    smp::no_interrupt(|| {
        let mut lock = CONSOLE.lock();
        lock.interrupt_handle();
    });
}

pub fn console_flush() {
    smp::no_interrupt(|| {
        let mut lock = CONSOLE.lock();
        lock.flush();
    });
}

pub fn console_set_callback(callback: Option<(u8, Box<dyn FnMut() -> bool + Send>)>) {
    smp::no_interrupt(|| {
        let mut lock = CONSOLE.lock();
        lock.set_callback(callback);
    });
}


/// Internal function called by the `kprint[ln]!` macros.
#[doc(hidden)]
#[no_mangle]
pub fn _print(args: fmt::Arguments) {
    #[cfg(not(test))]
        {
            use core::fmt::Write;
            smp::no_interrupt(|| {
                let mut console = CONSOLE.lock();
                // let mut console = unsafe { CONSOLE.unsafe_leak() };
                console.write_fmt(args).unwrap();
            });
        }

    #[cfg(test)]
        {
            print!("{}", args);
        }
}

/// Like `println!`, but for kernel-space.
#[macro_export]
macro_rules! kprintln {
    () => (kprint!("\n"));
    ($fmt:expr) => (kprint!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (kprint!(concat!($fmt, "\n"), $($arg)*));
}

/// Like `print!`, but for kernel-space.
#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => { $crate::console::_print(format_args!($($arg)*)) };
}
