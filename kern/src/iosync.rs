use shim::io;
use shim::io::Error;

use crate::console::CONSOLE;
use crate::mutex::{Mutex, MutexGuard};
use core::ops::DerefMut;
use core::ops::Deref;
use core::cell::UnsafeCell;
use pi::uart::MiniUart;
use crate::sync::Waitable;

pub trait SyncRead : Sync + Send {

    fn read(&self, buf: &mut [u8]) -> io::Result<usize>;

}

pub trait SyncWrite : Sync + Send {

    fn write(&self, buf: &[u8]) -> io::Result<usize>;

}

pub struct ConsoleSync(UnsafeCell<MiniUart>);

unsafe impl Sync for ConsoleSync {}

impl ConsoleSync {
    pub fn new() -> Self {
        ConsoleSync(UnsafeCell::new(MiniUart::new_opt_init(false)))
    }

    fn inner(&self) -> &mut MiniUart {
        unsafe { &mut *self.0.get() }
    }

}

impl SyncRead for ConsoleSync {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut lock = m_lock!(CONSOLE);
        lock.read_nonblocking(buf)
    }
}

impl SyncWrite for ConsoleSync {
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        use shim::io::Write;
        let mut lock = m_lock!(CONSOLE);
        lock.write(buf)
    }

}

impl io::Read for ConsoleSync {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        SyncRead::read(self, buf)
    }
}

impl io::Write for ConsoleSync {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        SyncWrite::write(self, buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

enum GlobalState<T> {
    Init(fn() -> T),
    Val(T)
}

pub struct Global<T>(Mutex<GlobalState<T>>);

impl<T> Global<T> {
    pub const fn new(f: fn() -> T) -> Self {
        Global(mutex_new!(GlobalState::Init(f)))
    }

    pub fn critical<R, F: FnOnce(&mut T) -> R>(&self, func: F) -> R {
        let mut lock = m_lock!(self.0);

        if let GlobalState::Init(f) = lock.deref() {
            *lock = GlobalState::Val(f());
        }

        if let GlobalState::Val(val) = lock.deref_mut() {
            func(val)
        } else {
            unreachable!();
        }
    }
}

impl<T: Clone> Global<T> {
    pub fn clone(&self) -> T {
        self.critical(|v| v.clone())
    }
}

pub struct ReadWrapper<T: AsRef<dyn SyncRead>>(T);

impl<T: AsRef<dyn SyncRead>> ReadWrapper<T> {
    pub fn new(t: T) -> Self {
        Self(t)
    }
}

impl<T: AsRef<dyn SyncRead>> io::Read for ReadWrapper<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.as_ref().read(buf)
    }
}

pub struct WriteWrapper<T: AsRef<dyn SyncWrite>>(T);

impl<T: AsRef<dyn SyncWrite>> WriteWrapper<T> {
    pub fn new(t: T) -> Self {
        Self(t)
    }
}

impl<T: AsRef<dyn SyncWrite>> io::Write for WriteWrapper<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.as_ref().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct TeeingWriter<W: io::Write, T: io::Write> {
    writer: W,
    tee: T,
}

impl<W: io::Write, T: io::Write> TeeingWriter<W, T> {
    pub fn new(writer: W, tee: T) -> Self {
        Self { writer, tee }
    }
}

impl<W: io::Write, T: io::Write> io::Write for TeeingWriter<W, T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tee.write(buf).ok();
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tee.flush().ok();
        self.writer.flush()
    }
}
