use core::cell::UnsafeCell;
use core::ops::Deref;
use core::ops::DerefMut;
use core::sync::atomic::AtomicBool;
use core::time::Duration;

use dsx::sync::mutex::LockableMutex;

use pi::uart::MiniUart;
use shim::io;
use shim::io::Error;

use crate::console::CONSOLE;
use crate::mutex::Mutex;
use crate::smp;
use crate::smp::no_interrupt;
use crate::sync::Waitable;

pub trait SyncRead: Sync + Send {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize>;
}

pub trait SyncWrite: Sync + Send {
    fn write(&self, buf: &[u8]) -> io::Result<usize>;
}

pub struct ConsoleSync();

impl ConsoleSync {
    pub fn new() -> Self {
        ConsoleSync()
    }
}

impl SyncRead for ConsoleSync {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        smp::no_interrupt(|| {
            let mut lock = m_lock!(CONSOLE);
            lock.read_nonblocking(buf)
        })
    }
}

impl SyncWrite for ConsoleSync {
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        smp::no_interrupt(|| {
            use shim::io::Write;
            let mut lock = m_lock!(CONSOLE);
            lock.write(buf)
        })
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
    Val(T),
}

pub struct Global<T>(Mutex<GlobalState<T>>);

impl<T> Global<T> {
    #[track_caller]
    pub const fn new(f: fn() -> T) -> Self {
        Global(Mutex::new(GlobalState::Init(f)))
    }

    #[track_caller]
    pub fn critical<R, F: FnOnce(&mut T) -> R>(&self, func: F) -> R {
        let mut lock = self.0.lock();

        if let GlobalState::Init(f) = lock.deref() {
            *lock = GlobalState::Val(f());
        }

        if let GlobalState::Val(val) = lock.deref_mut() {
            func(val)
        } else {
            unreachable!();
        }
    }

    pub fn try_critical<R, F: FnOnce(&mut T) -> R>(&self, func: F) -> Option<R> {
        let mut lock = self.0.try_lock()?;

        if let GlobalState::Init(f) = lock.deref() {
            *lock = GlobalState::Val(f());
        }

        if let GlobalState::Val(val) = lock.deref_mut() {
            Some(func(val))
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


#[repr(align(64))]
pub struct Lazy<T> {
    var: UnsafeCell<GlobalState<T>>,
}

impl<T> Lazy<T> {
    #[track_caller]
    pub const fn new(f: fn() -> T) -> Self {
        Lazy {
            var: UnsafeCell::new(GlobalState::Init(f)),
        }
    }

    fn do_get(&self) -> &GlobalState<T> {
        unsafe { &*self.var.get() }
    }

    fn do_get_mut(&self) -> &mut GlobalState<T> {
        unsafe { &mut *self.var.get() }
    }

    #[inline(never)]
    fn do_init(&self) {
        no_interrupt(|| {
            if let GlobalState::Init(f) = self.do_get() {
                *self.do_get_mut() = GlobalState::Val(f());
            }
        });
    }

    pub fn get(&self) -> &T {
        use core::intrinsics::unlikely;

        if unsafe { unlikely(matches!(self.do_get(), GlobalState::Init(_))) } {
            self.do_init();
        }

        if let GlobalState::Val(val) = self.do_get() {
            val
        } else {
            unreachable!();
        }
    }
}

impl<T> Deref for Lazy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
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
