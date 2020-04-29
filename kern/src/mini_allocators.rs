use core::alloc::Layout;

use mini_alloc::{LocalAlloc, SyncAlloc, MiniBox};
use mini_alloc::{BumpAllocator, BumpChunkProvider};

use crate::param::{PAGE_SIZE, PAGE_MASK};
use crate::{ALLOCATOR, VMM};
use crate::allocator::tags::MemTag;

fn page_layout() -> Layout {
    unsafe { Layout::from_size_align_unchecked(PAGE_SIZE, PAGE_SIZE) }
}

pub struct NoCachingChunkProvider();

impl BumpChunkProvider for NoCachingChunkProvider {
    fn make_chunk(min: Layout) -> Option<(usize, usize)> {
        assert!(min.size() <= PAGE_SIZE);
        assert!(min.align() <= PAGE_SIZE);

        let ptr = unsafe { ALLOCATOR.alloc_tag(page_layout(), MemTag::NoCacheMini) };

        unsafe { VMM.mark_page_non_cached(ptr as usize); }

        Some((ptr as usize, PAGE_SIZE))
    }
}

pub static NOCACHE_ALLOC: SyncAlloc<BumpAllocator<NoCachingChunkProvider>> = SyncAlloc::new(BumpAllocator::new);


