use core::alloc::Layout;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use crate::allocator::Alloc;

pub struct MiniBox<T: ?Sized> {
    data: NonNull<T>,
    alloc: &'static dyn Alloc,
}

impl<T> MiniBox<T> {
    #[inline(always)]
    pub fn new(alloc: &'static dyn Alloc, x: T) -> MiniBox<T> {
        let lay = Layout::new::<T>();
        let ptr = unsafe { alloc.alloc(lay) };
        let mut data = NonNull::new(ptr as *mut T).expect("no memory");
        unsafe { *data.as_mut() = x };
        MiniBox {
            data,
            alloc,
        }
    }

    pub unsafe fn new_zeroed(alloc: &'static dyn Alloc) -> MiniBox<T> {
        let lay = Layout::new::<T>();
        let ptr = unsafe { alloc.alloc(lay) };
        if !ptr.is_null() {
            core::ptr::write_bytes(ptr, 0, core::mem::size_of::<T>());

            for i in 0..core::mem::size_of::<T>() {
                if *ptr.offset(i as isize) != 0 {
                    panic!("zero memory didnt work");
                }
            }

        }
        let mut data = NonNull::new(ptr as *mut T).expect("no memory");
        MiniBox {
            data,
            alloc,
        }
    }

    pub const fn as_ptr(&self) -> *mut T {
        self.data.as_ptr()
    }

}

impl<T: ?Sized> Deref for MiniBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for MiniBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.data.as_mut() }
    }
}

impl<T: fmt::Display + ?Sized> fmt::Display for MiniBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug + ?Sized> fmt::Debug for MiniBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized> Drop for MiniBox<T> {
    fn drop(&mut self) {
        let lay = unsafe { Layout::for_value(self.data.as_ref()) };
        let ptr = self.data.as_ptr() as *mut u8;
        unsafe { self.alloc.dealloc(ptr, lay) }
    }
}