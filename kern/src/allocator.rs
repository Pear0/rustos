use core::alloc::{GlobalAlloc, Layout};
use core::fmt;

use pi::atags::Atags;

use crate::mutex::{mutex_new, Mutex};
use crate::smp;
use crate::mutex::m_lock;

mod linked_list;
pub mod util;

mod bin;
mod bump;

type AllocatorImpl = bin::Allocator;

#[cfg(test)]
mod tests;

/// `LocalAlloc` is an analogous trait to the standard library's `GlobalAlloc`,
/// but it takes `&mut self` in `alloc()` and `dealloc()`.
pub trait LocalAlloc {
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8;
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout);
}

/// Thread-safe (locking) wrapper around a particular memory allocator.
pub struct Allocator(Mutex<Option<AllocatorImpl>>);

impl Allocator {
    /// Returns an uninitialized `Allocator`.
    ///
    /// The allocator must be initialized by calling `initialize()` before the
    /// first memory allocation. Failure to do will result in panics.
    pub const fn uninitialized() -> Self {
        Allocator(mutex_new!(None))
    }

    /// Initializes the memory allocator.
    /// The caller should assure that the method is invoked only once during the
    /// kernel initialization.
    ///
    /// # Panics
    ///
    /// Panics if the system's memory map could not be retrieved.
    pub unsafe fn initialize(&self) {
        let (start, end) = memory_map().expect("failed to find memory map");
        unsafe { self.0.set_led(29); }
        *m_lock!(self.0) = Some(AllocatorImpl::new(start, end));
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        smp::no_interrupt(|| {
            m_lock!(self.0)
                .as_mut()
                .expect("allocator uninitialized")
                .alloc(layout)
        })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        smp::no_interrupt(|| {
            m_lock!(self.0)
                .as_mut()
                .expect("allocator uninitialized")
                .dealloc(ptr, layout);
        })
    }
}

extern "C" {
    static __text_end: u8;
}

/// Returns the (start address, end address) of the available memory on this
/// system if it can be determined. If it cannot, `None` is returned.
///
/// This function is expected to return `Some` under all normal cirumstances.
pub fn memory_map() -> Option<(usize, usize)> {
    // let page_size = 1 << 12;
    let binary_end = unsafe { (&__text_end as *const u8) as usize };

    let mut mem_end = 0u32;

    for atag in Atags::get() {
        if let Some(mem) = atag.mem() {
            mem_end = mem.start + mem.size;
        }
    }

    if (mem_end as usize) < binary_end {
        panic!("mem_end {} < binary_end {}", mem_end, binary_end);
    }

    Some((binary_end, mem_end as usize))
}

impl fmt::Debug for Allocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match m_lock!(self.0).as_mut() {
            Some(ref alloc) => write!(f, "{:?}", alloc)?,
            None => write!(f, "Not yet initialized")?,
        }
        Ok(())
    }
}
