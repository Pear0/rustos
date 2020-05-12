use alloc::sync::Arc;

use shim::io;

use crate::{smp, sync};
use crate::console::CONSOLE;
use crate::iosync::{SyncRead, SyncWrite};
use crate::kernel_call::syscall;
use crate::net::buffer;
use crate::sync::Waitable;

#[derive(Clone)]
pub enum Source {
    KernSerial,
    Buffer(buffer::BufferHandle),
    Nil,
}

impl SyncRead for Source {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Source::KernSerial => {
                smp::no_interrupt(|| {
                    use shim::io::Read;
                    let mut console = CONSOLE.lock("handle::Source::read()");
                    console.read(buf)
                })
            }
            Source::Buffer(b) => {
                b.read(buf).map_err(|e| e.into_io_err())
            }
            Source::Nil => Ok(0),
        }
    }
}

impl sync::Waitable for Source {
    fn done_waiting(&self) -> bool {
        match self {
            Source::KernSerial => {
                smp::no_interrupt(|| {
                    let mut console = CONSOLE.lock("handle::Source::done_waiting()");
                    console.has_byte()
                })
            }
            Source::Buffer(b) => {
                use sync::Waitable;
                buffer::ReadWaitable(b.clone()).done_waiting()
            }
            Source::Nil => false,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Source::KernSerial => "Source::KernSerial",
            Source::Buffer(_) => "Source::Buffer",
            Source::Nil => "Source::Nil",
        }
    }
}


#[derive(Clone)]
pub enum Sink {
    KernSerial,
    Buffer(buffer::BufferHandle),
    Nil,
}

impl Sink {
    pub fn estimate_free_capacity(&self) -> Option<usize> {
        match self {
            Sink::KernSerial => None,
            Sink::Buffer(b) => Some(b.free_capacity()),
            Sink::Nil => None,
        }
    }
}

impl SyncWrite for Sink {
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Sink::KernSerial => {
                smp::no_interrupt(|| {
                    use shim::io::Write;
                    let mut console = CONSOLE.lock("handle::Sink::write()");
                    console.write(buf)
                })
            }
            Sink::Buffer(b) => {
                b.write(buf).map_err(|e| e.into_io_err())
            }
            Sink::Nil => Ok(buf.len()),
        }
    }
}

impl sync::Waitable for Sink {
    fn done_waiting(&self) -> bool {
        match self {
            Sink::KernSerial => true, // TODO
            Sink::Buffer(b) => {
                use sync::Waitable;
                buffer::WriteWaitable(b.clone()).done_waiting()
            }
            Sink::Nil => true,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Sink::KernSerial => "Sink::KernSerial",
            Sink::Buffer(_) => "Sink::Buffer",
            Sink::Nil => "Sink::Nil",
        }
    }
}


pub struct SourceWrapper<T: AsRef<Source>>(T);

impl<T: AsRef<Source>> SourceWrapper<T> {
    pub fn new(t: T) -> Self {
        Self(t)
    }
}

impl<T: AsRef<Source>> io::Read for SourceWrapper<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.as_ref().read(buf)
    }
}

pub struct WaitingSourceWrapper(Arc<Source>);

impl WaitingSourceWrapper {
    pub fn new(t: Arc<Source>) -> Self {
        Self(t)
    }
}

impl io::Read for WaitingSourceWrapper {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.0.done_waiting() {
            syscall::wait_waitable(self.0.clone());
        }
        self.0.as_ref().read(buf)
    }
}

pub struct SinkWrapper<T: AsRef<Sink>>(T);

impl<T: AsRef<Sink>> SinkWrapper<T> {
    pub fn new(t: T) -> Self {
        Self(t)
    }
}

impl<T: AsRef<Sink>> io::Write for SinkWrapper<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.as_ref().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}



