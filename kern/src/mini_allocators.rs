use core::alloc::Layout;

use mini_alloc::{LocalAlloc, SyncAlloc, MiniBox};
use mini_alloc::{BumpAllocator, BumpChunkProvider};

use crate::param::{PAGE_SIZE, PAGE_MASK};
use crate::{ALLOCATOR, VMM};
use crate::allocator::tags::MemTag;

pub static NOCACHE_ALLOC: SyncAlloc<BumpAllocator<NoCachingChunkProvider>> = SyncAlloc::new(BumpAllocator::new);
pub static NOCACHE_PAGE_ALLOC: SyncAlloc<NoCachingPageAllocator> = SyncAlloc::new(NoCachingPageAllocator::new);

fn page_layout() -> Layout {
    unsafe { Layout::from_size_align_unchecked(PAGE_SIZE, PAGE_SIZE) }
}

pub struct NoCachingChunkProvider;

impl BumpChunkProvider for NoCachingChunkProvider {
    fn make_chunk(min: Layout) -> Option<(usize, usize)> {
        assert!(min.size() <= PAGE_SIZE);
        assert!(min.align() <= PAGE_SIZE);

        let ptr = unsafe { ALLOCATOR.alloc_tag(page_layout(), MemTag::NoCacheMini) };
        if !ptr.is_null() {
            unsafe { VMM.mark_page_non_cached(ptr as usize); }
        }

        Some((ptr as usize, PAGE_SIZE))
    }
}

pub struct NoCachingPageAllocator();

impl NoCachingPageAllocator {
    pub fn new() -> Self {
        Self()
    }
}

impl LocalAlloc for NoCachingPageAllocator {
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let page_layout = layout.align_to(PAGE_SIZE).unwrap();

        let ptr = ALLOCATOR.alloc_tag(page_layout, MemTag::NoCacheDirect);
        if !ptr.is_null() {
            for offset in (0..page_layout.size()).step_by(PAGE_SIZE) {
                VMM.mark_page_non_cached((ptr as usize) + offset);
            }
        }

        ptr
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
    }
}


