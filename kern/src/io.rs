use shim::io;
use shim::io::Error;

use crate::console::{CONSOLE, kprintln};
use crate::mutex::{Mutex, MutexGuard};
use core::ops::DerefMut;
use core::ops::Deref;

pub trait SyncRead : Sync + Send {

    fn read(&self, buf: &mut [u8]) -> io::Result<usize>;

}

pub trait SyncWrite : Sync + Send {

    fn write(&self, buf: &[u8]) -> io::Result<usize>;

}

#[derive(Clone)]
pub struct ConsoleSync();

impl ConsoleSync {
    pub fn new() -> Self {
        ConsoleSync()
    }
}

impl SyncRead for ConsoleSync {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut console = CONSOLE.lock();
        console.read_nonblocking(buf)
    }
}

impl SyncWrite for ConsoleSync {
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        use shim::io::Write;
        let mut console = CONSOLE.lock();
        console.write(buf)
    }

}

enum GlobalState<T: Clone> {
    Init(fn() -> T),
    Val(T)
}

pub struct Global<T: Clone>(Mutex<GlobalState<T>>);

impl<T: Clone> Global<T> {
    pub const fn new(f: fn() -> T) -> Self {
        Global(Mutex::new(GlobalState::Init(f)))
    }

    pub fn get(&self) -> T {
        let mut lock = self.0.lock();

        match lock.deref() {
            GlobalState::Init(f) => {
                let t = f();
                *lock = GlobalState::Val(t.clone());
                t
            }
            GlobalState::Val(t) => {
                t.clone()
            }
        }
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

