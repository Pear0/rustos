use core::fmt;

use pi::uart::MiniUart;
use shim::io;

use crate::mutex::Mutex;

/// A global singleton allowing read/write access to the console.
pub struct Console {
    inner: Option<MiniUart>,
}

impl Console {
    /// Creates a new instance of `Console`.
    const fn new() -> Console {
        Console { inner: None }
    }

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

    /// Reads a byte from the UART device, blocking until a byte is available.
    pub fn read_byte(&mut self) -> u8 {
        self.inner().read_byte()
    }

    /// Writes the byte `byte` to the UART device.
    pub fn write_byte(&mut self, byte: u8) {
        self.inner().write_byte(byte)
    }
}

impl io::Read for Console {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner().read(buf)
    }
}

impl io::Write for Console {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for byte in buf.iter() {
            if *byte == b'\n' {
                self.inner().write_byte(b'\r');
            }
            self.inner().write_byte(*byte);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.inner().write_str(s)
    }
}

/// Global `Console` singleton.
pub static CONSOLE: Mutex<Console> = Mutex::new(Console::new());

/// Internal function called by the `kprint[ln]!` macros.
#[doc(hidden)]
#[no_mangle]
pub fn _print(args: fmt::Arguments) {
    #[cfg(not(test))]
    {
        use core::fmt::Write;
        let mut console = CONSOLE.lock();
        console.write_fmt(args).unwrap();
    }

    #[cfg(test)]
    {
        print!("{}", args);
    }
}

/// Like `println!`, but for kernel-space.
pub macro kprintln {
    () => (kprint!("\n")),
    ($fmt:expr) => (kprint!(concat!($fmt, "\n"))),
    ($fmt:expr, $($arg:tt)*) => (kprint!(concat!($fmt, "\n"), $($arg)*))
}

/// Like `print!`, but for kernel-space.
pub macro kprint($($arg:tt)*) {
    _print(format_args!($($arg)*))
}
