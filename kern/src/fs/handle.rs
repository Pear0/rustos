use crate::net::buffer;
use crate::io::{SyncWrite, SyncRead};
use shim::io;
use crate::smp;
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

pub enum Sink {
    KernSerial,
    Buffer(buffer::BufferHandle),
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

