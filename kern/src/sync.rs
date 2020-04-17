use alloc::sync::Arc;
use crate::mutex::Mutex;
use core::marker::PhantomData;
use core::ops::Deref;
use crate::fs::handle::Sink;

#[derive(Debug)]
pub struct Completion<T>(Mutex<Option<T>>);

impl<T> Completion<T> {
    pub fn new() -> Self {
        Self(mutex_new!(None))
    }

    pub fn complete(&self, value: T) -> bool {
        let mut l = m_lock!(self.0);
        if l.is_none() {
            l.replace(value);
            true
        } else {
            false
        }
    }

    pub fn get(&self) -> Option<&T> {
        // we effectively leak out of the mutex here but this is
        // safe since only one write is ever allowed and we only
        // leak after the write has occurred, turning the value
        // immutable.
        let s = m_lock!(self.0).as_ref()? as *const T;
        unsafe { Some(&*s) }
    }
}


pub trait Waitable : Sync + Send {

    fn done_waiting(&self) -> bool;

    fn name(&self) -> &'static str {
        "[unknown waitable]"
    }

}


