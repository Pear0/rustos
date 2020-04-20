use core::alloc::Layout;
use core::cmp::max;
use core::marker::PhantomData;

use crate::allocator::LocalAlloc;
use crate::allocator::util::align_up;

pub trait BumpChunkProvider {
    // -> Option<(ptr, len)>
    fn make_chunk(min: Layout) -> Option<(usize, usize)>;
}

/// A "bump" allocator: allocates memory by bumping a pointer; never frees.
#[derive(Debug)]
pub struct BumpAllocator<P: BumpChunkProvider> {
    current: usize,
    end: usize,
    _phantom: PhantomData<P>,
}

impl<P: BumpChunkProvider> BumpAllocator<P> {
    /// Creates a new bump allocator that will allocate memory from the region
    /// starting at address `start` and ending at address `end`.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self { current: 0, end: 0, _phantom: PhantomData::default() }
    }

    unsafe fn try_alloc(&mut self, layout: Layout) -> *mut u8 {
        let min_align = 8;

        let aligned_current = align_up(self.current, max(layout.align(), min_align));

        let end = aligned_current.saturating_add(layout.size());
        if end - aligned_current == layout.size() && end <= self.end {
            self.current = end;
            return aligned_current as *mut u8;
        }

        core::ptr::null_mut()
    }
}

impl<P: BumpChunkProvider> LocalAlloc for BumpAllocator<P> {

    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let ptr = self.try_alloc(layout);
        if !ptr.is_null() {
            return ptr;
        }

        match P::make_chunk(layout) {
            Some((ptr, size)) => {
                self.current = ptr;
                self.end = ptr + size;
            }
            None => return core::ptr::null_mut(),
        }

        self.try_alloc(layout)
    }

    unsafe fn dealloc(&mut self, _ptr: *mut u8, _layout: Layout) {
        // LEAKED
    }
}
