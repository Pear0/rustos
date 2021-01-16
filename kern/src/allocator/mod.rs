use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::fmt;
use core::sync::atomic::Ordering;

use dsx::sync::mutex::LockableMutex;
use enumset::EnumSet;
use karch::capability::ExecCapability;

use pi::atags::Atags;
use shim::io;

use crate::{EXEC_CONTEXT, hw, smp};
use crate::allocator::tags::{MemTag, TaggingAlloc};
use crate::init::SAFE_ALLOC_START;
use crate::mutex::Mutex;
use crate::cls::CORE_COUNT;
use crate::smp::core;
use crate::traps::IRQ_RECURSION_DEPTH;

mod linked_list;
pub mod tags;
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

pub trait AllocStats {
    fn total_allocation(&self) -> (usize, usize);

    fn dump(&self, w: &mut dyn io::Write) -> io::Result<()>;
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
        *m_lock!(self.0) = Some(AllocatorImpl::new(start, end));
    }

    pub fn with_internal<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&AllocatorImpl) -> R,
    {
        smp::no_interrupt(|| {
            let lock = m_lock!(self.0);
            f(lock.as_ref().expect("allocator uninitialized"))
        })
    }

    pub fn with_internal_mut<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut AllocatorImpl) -> R,
    {
        smp::no_interrupt(|| {
            let mut lock = m_lock!(self.0);
            f(lock.as_mut().expect("allocator uninitialized"))
        })
    }

    pub unsafe fn alloc_tag(&self, layout: Layout, tag: MemTag) -> *mut u8 {
        let _guard = smp::interrupt_guard_outside_exc();

        let v = EXEC_CONTEXT.lock_capability(EnumSet::only(ExecCapability::Allocation), || {
            self.0.lock()
                .as_mut()
                .expect("allocator uninitialized")
                .alloc_tag(layout, tag)
        });

        // drop(_guard);

        v
    }

    pub unsafe fn dealloc_tag(&self, ptr: *mut u8, layout: Layout, tag: MemTag) {
        let _guard = smp::interrupt_guard_outside_exc();

        EXEC_CONTEXT.lock_capability(EnumSet::only(ExecCapability::Allocation), || {
            self.0.lock()
                .as_mut()
                .expect("allocator uninitialized")
                .dealloc_tag(ptr, layout, tag);
        });

        // drop(_guard)
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_tag(layout, MemTag::Global)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.dealloc_tag(ptr, layout, MemTag::Global)
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
    let binary_end = SAFE_ALLOC_START.load(Ordering::Relaxed) as usize;

    let mut mem_end = 0u32;

    hw::arch().iter_memory_regions(&mut |start, size| {
        let size = size as u32;
        if start == 0 && size > mem_end {
            mem_end = size;
        }
    });

    // for atag in Atags::get() {
    //     if let Some(mem) = atag.mem() {
    //         mem_end = mem.start + mem.size;
    //     }
    // }

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

#[derive(Default)]
pub struct MpThreadLocal<T: Default>(UnsafeCell<T>);

impl<T: Default> mpalloc::ThreadLocal<T> for MpThreadLocal<T> {
    unsafe fn get_mut(&self) -> &mut T {
        &mut *self.0.get()
    }
}

pub struct MpAllocHook;

impl mpalloc::Hooks for MpAllocHook {
    type TL = MpThreadLocal<Option<&'static dyn GlobalAlloc>>;
}

pub type MpAllocator = mpalloc::Allocator<MpAllocHook>;


pub struct FullThreadLocal<T: Default> {
    pub containers: [UnsafeCell<Option<T>>; CORE_COUNT * 2],
    pub init: fn() -> T,
}

impl <T: Default> FullThreadLocal<T> {
    pub const fn new(init: fn() -> T) -> Self {
        FullThreadLocal {
            containers: [
                UnsafeCell::new(None), UnsafeCell::new(None), UnsafeCell::new(None), UnsafeCell::new(None),
                UnsafeCell::new(None), UnsafeCell::new(None), UnsafeCell::new(None), UnsafeCell::new(None),
            ],
            init,
        }
    }

    pub const fn new_default() -> Self {
        Self::new(|| T::default())
    }
}

impl<T: Default> mpalloc::ThreadLocal<T> for FullThreadLocal<T> {
    unsafe fn get_mut(&self) -> &mut T {
        let mut core = core();
        if IRQ_RECURSION_DEPTH.get() > 0 {
            core += CORE_COUNT;
        }
        let container = &mut *self.containers[core].get();
        if container.is_none() {
            container.replace((self.init)());
        }
        container.as_mut().unwrap()
    }
}