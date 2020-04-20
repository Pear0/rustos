use core::alloc::Layout;
use crate::mutex::Mutex;

mod bump;
mod util;

pub use bump::*;

pub trait Alloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8;
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout);
}

pub trait LocalAlloc {
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8;
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout);
}

enum Val<T: LocalAlloc> {
    Init(fn() -> T),
    Val(T),
}

pub struct SyncAlloc<T: LocalAlloc>(Mutex<Val<T>>);

impl<T: LocalAlloc> SyncAlloc<T> {
    pub const fn new(f: fn() -> T) -> Self {
        SyncAlloc(Mutex::new(Val::Init(f)))
    }

    fn with<R, F: FnOnce(&mut T) -> R>(&self, func: F) -> R {
        use core::ops::DerefMut;
        let mut lock = self.0.lock();
        if let Val::Init(f) = *lock {
            *lock = Val::Val(f());
        }

        if let Val::Val(t) = lock.deref_mut() {
            func(t)
        } else {
            unreachable!();
        }
    }

}

impl<T: LocalAlloc> Alloc for SyncAlloc<T> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.with(|a| a.alloc(layout))
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.with(|a| a.dealloc(ptr, layout))
    }
}


