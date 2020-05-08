use core::fmt;

use pi::uart::MiniUart;
use shim::io;

use crate::mutex::{Mutex, MutexGuard};
use crate::smp;
use crate::collections::CapacityRingBuffer;
use aarch64::SCTLR_EL1;
use crate::fs::handle::{Sink, Source};

struct ConsoleImpl {
    send_buffer: CapacityRingBuffer<u8>,
    receive_buffer: CapacityRingBuffer<u8>,

    callback: Option<(u8, fn())>,

    redirect: Option<(Sink, Source)>,
}

impl ConsoleImpl {
    pub fn new() -> Self {
        Self {
            send_buffer: CapacityRingBuffer::new(1000),
            receive_buffer: CapacityRingBuffer::new(1000),
            callback: Some((0x02, || {
                info!("pressed Ctrl+B");
                let mut lock = CONSOLE.lock("callback");
                lock.ext.as_mut().unwrap().redirect = None;
            })),
            redirect: None,
        }
    }
}

/// A global singleton allowing read/write access to the console.
pub struct Console {
    inner: Option<MiniUart>,
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
            self.inner = Some(MiniUart::new());
        }
    }

    /// Returns a mutable borrow to the inner `MiniUart`, initializing it as
    /// needed.
    fn inner(&mut self) -> &mut MiniUart {
        self.initialize();
        self.inner.as_mut().unwrap()
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
                    return Ok(read)
                }
                buf[0] = byte;
                buf = &mut buf[1..];
                read += 1;
            }
        }

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

    pub fn interrupt_handle(&mut self) {
        if let None = &self.ext {
            return;
        }

        while self.inner().has_byte() {
            // TODO may drop bytes on ground.
            let byte = self.inner().read_byte();
            self.ext.as_mut().unwrap().receive_buffer.insert(byte);
        }

        self.send_and_update_interrupts();

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

    fn interrupt_handle(&mut self) {
        if let None = &self.ext {
            return;
        }

        'read: while self.inner().has_byte() {
            // TODO may drop bytes on ground.
            let byte = self.inner().read_byte();

            if let Some((expected, func)) = self.ext.as_ref().unwrap().callback {
                if byte == expected {
                    self.recursion(|| func());
                    continue 'read;
                }
            }

            self.ext.as_mut().unwrap().receive_buffer.insert(byte);
        }

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
pub static CONSOLE: Mutex<Console> = Mutex::new("CONSOLE", Console::new());

pub fn console_ext_init() {
    smp::no_interrupt(|| {
        let mut lock = CONSOLE.lock("console_ext_init");
        if lock.ext.is_none() {
            lock.ext = Some(ConsoleImpl::new());
        }
    });
}

pub fn console_interrupt_handler() {
    smp::no_interrupt(|| {
        let mut lock = CONSOLE.lock("console_interrupt_handler");
        lock.interrupt_handle();
    });
}

pub fn console_flush() {
    smp::no_interrupt(|| {
        let mut lock = CONSOLE.lock("console_flush");
        lock.flush();
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
            let mut console = CONSOLE.lock("_print");
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
