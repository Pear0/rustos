use alloc::collections::VecDeque;
use alloc::sync::Arc;
use shim::ioerr;

use crate::mutex::Mutex;
use crate::net::NetErrorKind::BufferFull;
use core::ops::DerefMut;
use crate::net::{NetResult, NetErrorKind};
use crate::iosync::{SyncWrite, SyncRead};
use shim::io;
use shim::io::Error;
use crate::sync;
use core::time::Duration;

struct Buffer {
    deque: VecDeque<u8>,
    max_size: usize,
}

#[derive(Clone)]
pub struct BufferHandle(Arc<Mutex<Buffer>>);

impl BufferHandle {
    pub fn new() -> Self {
        BufferHandle(Arc::new(mutex_new!(Buffer {
            deque: VecDeque::new(),
            max_size: 4096,
        })))
    }

    fn critical<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut Buffer) -> R,
    {
        let mut guard = m_lock!(self.0);
        f(guard.deref_mut())
    }

    pub fn len(&self) -> usize {
        self.critical(|b| b.deque.len())
    }

    pub fn write_full(&self, buf: &[u8]) -> NetResult<()> {
        self.critical(|b| {
            if buf.len() > (b.max_size - b.deque.len()) {
                Err(NetErrorKind::BufferFull)
            } else {
                b.deque.extend(buf.into_iter());
                Ok(())
            }
        })
    }

    pub fn write(&self, buf: &[u8]) -> NetResult<usize> {
        self.critical(|b| {
            let amt = core::cmp::min(buf.len(), b.max_size - b.deque.len());
            for i in 0..amt {
                b.deque.push_back(buf[i]);
            }
            Ok(amt)
        })
    }

    pub fn read(&self, buf: &mut [u8]) -> NetResult<usize> {
        self.critical(|b| {
            let amt = core::cmp::min(buf.len(), b.deque.len());
            for i in 0..amt {
                buf[i] = b.deque.pop_front().expect("i have the lock ???");
            }
            Ok(amt)
        })
    }

}

impl SyncRead for BufferHandle {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        BufferHandle::read(self, buf).or(ioerr!(Other, "buffer read fail"))
    }
}

impl SyncWrite for BufferHandle {
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        BufferHandle::write(self, buf).or(ioerr!(WouldBlock, "buffer full"))
    }
}

#[derive(Clone)]
pub struct ReadWaitable(pub BufferHandle);

impl sync::Waitable for ReadWaitable {

    fn done_waiting(&self) -> bool {
        if let Some(b) = (self.0).0.lock_timeout("ReadWaitable::done_waiting", Duration::from_micros(1)) {
            return !b.deque.is_empty();
        }
        return false;
    }

    fn name(&self) -> &'static str {
        "[buffer::ReadWaitable]"
    }
}

#[derive(Clone)]
pub struct WriteWaitable(pub BufferHandle);

impl sync::Waitable for WriteWaitable {

    fn done_waiting(&self) -> bool {
        if let Some(b) = (self.0).0.lock_timeout("WriteWaitable::done_waiting", Duration::from_micros(1)) {
            return b.deque.len() < b.max_size;
        }
        return false;
    }

    fn name(&self) -> &'static str {
        "[buffer::WriteWaitable]"
    }
}


