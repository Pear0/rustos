use core::alloc::Layout;
use core::fmt;
use core::ptr;
use core::result;

use crate::allocator::linked_list::LinkedList;
use crate::allocator::util::*;
use crate::allocator::LocalAlloc;

use super::util::align_up;

type Result<T> = result::Result<T, &'static str>;


/// A simple allocator that allocates based on size classes.
///   bin 0 (2^3 bytes)    : handles allocations in (0, 2^3]
///   bin 1 (2^4 bytes)    : handles allocations in (2^3, 2^4]
///   ...
///   bin 29 (2^22 bytes): handles allocations in (2^31, 2^32]
///   
///   map_to_bin(size) -> k
///

/// I hope you like fastbins.
#[derive(Debug)]
pub struct Allocator {
    wilderness: usize,
    wilderness_end: usize,
    bins: [LinkedList; 30],
}

fn has_alignment(ptr: usize, align: usize) -> bool {
    ptr % align == 0
}

impl Allocator {
    /// Creates a new bin allocator that will allocate memory from the region
    /// starting at address `start` and ending at address `end`.
    pub fn new(start: usize, end: usize) -> Allocator {
        // println!("BinAlloc::new(0x{:x}, 0x{:x})", start, end);
        Allocator {
            wilderness: align_up(start, 8),
            wilderness_end: end,
            bins: [LinkedList::new(); 30],
        }
    }

    fn dump(&self, msg: &'static str) {
        // println!("[dump:{}] BinAlloc(wilderness: 0x{:x}, wilderness_end: 0x{:x}) -> wilderness size: 0x{:x}",
        //          msg, self.wilderness, self.wilderness_end, self.wilderness_end - self.wilderness);
    }

    fn map_to_bin(&self, mut size: usize) -> usize {
        let mut bin = 0usize;
        size = (size - 1) / 8;

        while size != 0 {
            size /= 2;
            bin += 1;
        }

        bin
    }

    fn bin_size(&self, bin: usize) -> usize {
        1usize << (bin + 3)
    }

    fn split_bin(&mut self, bin: usize) -> bool {
        if bin == 0 {
            return false; // cannot split a minimum size bin
        }

        match self.bins[bin].pop() {
            None => false,
            Some(ptr) => {
                let sub_size = self.bin_size(bin - 1);

                unsafe {
                    self.bins[bin - 1].push(ptr);
                    self.bins[bin - 1].push(((ptr as usize) + sub_size) as *mut usize);
                }

                true
            }
        }
    }

    /// Instead of naively incurring external fragmentation, this function will
    /// place the allocate the largest possible bin entries as possible in the
    /// region. This way otherwise lost space gets turned into more small fastbins.
    fn fill_allocations(&mut self, mut start: usize, end: usize) -> bool {
        'fill_loop: while start != end {
            assert!(end - start > 7);

            for i in (0..self.bins.len()).rev() {
                if has_alignment(self.wilderness, self.bin_size(i)) {
                    // will not recurse because this is a perfect fit.
                    self.allocate_bin_entry(i);
                    start += self.bin_size(i);
                    continue 'fill_loop;
                }
            }

            return false;
        }

        true
    }

    fn allocate_bin_entry(&mut self, bin: usize) -> bool {
        let alloc_start = align_up(self.wilderness, self.bin_size(bin));

        // println!("[alloc wilderness] wilderness=0x{:x}, bin_size=0x{:x}, alloc_start=0x{:x}, wilderness_end=0x{:x}",
        //          self.wilderness, self.bin_size(bin), alloc_start, self.wilderness_end );

        if alloc_start + self.bin_size(bin) > self.wilderness_end {
            // println!("[alloc wilderness] failed, allocation would exceed wilderness");
            return false;
        }

        if !self.fill_allocations(self.wilderness, alloc_start) {
            // println!("[alloc wilderness] failed to fill in alignment bins");
            return false;
        }

        self.wilderness = alloc_start + self.bin_size(bin);

        unsafe { self.bins[bin].push(alloc_start as *mut usize) };

        true
    }

    fn layout_to_bin(&self, layout: Layout) -> usize {
        self.map_to_bin(core::cmp::max(layout.size(), layout.align()))
    }

    fn recursive_split_bin(&mut self, target_bin: usize) -> bool {
        if !self.bins[target_bin].is_empty() {
            return true;
        }

        // cannot split larger, we are largest bin
        if target_bin == self.bins.len() - 1 {
            return false;
        }

        // failed to split larger chunks
        if self.bins[target_bin+1].is_empty() && !self.recursive_split_bin(target_bin+1) {
            return false;
        }

        if !self.split_bin(target_bin+1) {
            return false;
        }

        !self.bins[target_bin].is_empty()
    }

    fn scavenge_bin(&mut self, bin: usize) -> bool {

        if self.recursive_split_bin(bin) {
            return true;
        }

        // println!("[alloc] bin splitting failed, allocate from wilderness");

        // nothing else worked. try allocate
        self.allocate_bin_entry(bin)
    }

    fn do_alloc(&mut self, layout: Layout) -> Option<*mut u8> {
        let bin = self.layout_to_bin(layout);

        // println!("[alloc] layout(size:0x{:x}, align:0x{:x}) -> bin:{}, bin_size:0x{:x}", layout.size(), layout.align(), bin, self.bin_size(bin));

        if let Some(p) = self.bins[bin].pop() {
            // println!("[alloc] served with fastbin:{}", bin);
            return Some(p as *mut u8);
        }

        if !self.scavenge_bin(bin) {
            // println!("[alloc] failed to scavenge");
            return None;
        }

        Some(self.bins[bin].pop().unwrap() as *mut u8)
    }

    fn do_dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let bin = self.layout_to_bin(layout);

        // cast is safe because we only ever give out 8 byte aligned pointers
        // anyway.
        unsafe { self.bins[bin].push(ptr as *mut usize) };
    }

}

impl LocalAlloc for Allocator {
    /// Allocates memory. Returns a pointer meeting the size and alignment
    /// properties of `layout.size()` and `layout.align()`.
    ///
    /// If this method returns an `Ok(addr)`, `addr` will be non-null address
    /// pointing to a block of storage suitable for holding an instance of
    /// `layout`. In particular, the block will be at least `layout.size()`
    /// bytes large and will be aligned to `layout.align()`. The returned block
    /// of storage may or may not have its contents initialized or zeroed.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure that `layout.size() > 0` and that
    /// `layout.align()` is a power of two. Parameters not meeting these
    /// conditions may result in undefined behavior.
    ///
    /// # Errors
    ///
    /// Returning null pointer (`core::ptr::null_mut`)
    /// indicates that either memory is exhausted
    /// or `layout` does not meet this allocator's
    /// size or alignment constraints.
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        // println!("alloc(size: 0x{:x}, align: 0x{:x})", layout.size(), layout.align());
        let x = self.do_alloc(layout).unwrap_or(0 as *mut u8);
        // self.dump("alloc");
        x
    }

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure the following:
    ///
    ///   * `ptr` must denote a block of memory currently allocated via this
    ///     allocator
    ///   * `layout` must properly represent the original layout used in the
    ///     allocation call that returned `ptr`
    ///
    /// Parameters not meeting these conditions may result in undefined
    /// behavior.
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        // println!("dealloc(ptr: 0x{:x}, size: 0x{:x}, align: 0x{:x})", ptr as usize, layout.size(), layout.align());
        self.do_dealloc(ptr, layout);
        // self.dump("alloc");
    }
}

// FIXME: Implement `Debug` for `Allocator`.
