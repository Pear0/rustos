use crate::net::buffer;
use crate::iosync::{SyncWrite, SyncRead};
use shim::io;
use crate::{smp, sync};
use crate::console::CONSOLE;

pub enum Source {
    KernSerial,
    Buffer(buffer::BufferHandle),
}

impl SyncRead for Source {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Source::KernSerial => {
                smp::no_interrupt(|| {
                    use shim::io::Read;
                    let mut console = CONSOLE.lock("handle::Sink::read()");
                    console.read(buf)
                })
            },
            Source::Buffer(b) => {
                b.read(buf).map_err(|e| e.into_io_err())
            },
        }
    }
}

impl sync::Waitable for Source {
    fn done_waiting(&self) -> bool {
        match self {
            Source::KernSerial => true, // TODO
            Source::Buffer(b) => {
                use sync::Waitable;
                buffer::ReadWaitable(b.clone()).done_waiting()
            },
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Source::KernSerial => "Source::KernSerial",
            Source::Buffer(_) => "Source::Buffer",
        }
    }
}


pub enum Sink {
    KernSerial,
    Buffer(buffer::BufferHandle),
}

impl Sink {

    pub fn estimate_free_capacity(&self) -> Option<usize> {
        match self {
            Sink::KernSerial => None,
            Sink::Buffer(b) => Some(b.free_capacity()),
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
            },
            Sink::Buffer(b) => {
                b.write(buf).map_err(|e| e.into_io_err())
            },
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
            },
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Sink::KernSerial => "Sink::KernSerial",
            Sink::Buffer(_) => "Sink::Buffer",
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



