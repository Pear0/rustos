use core::alloc::Layout;

use crate::allocator::linked_list::LinkedList;
use crate::allocator::{LocalAlloc, AllocStats};
use crate::allocator::tags::{TaggingAlloc, MemTag};

use super::util::align_up;
use shim::io;
use pi::atags::Atag::Mem;
use arrayvec::ArrayVec;
use crate::allocator::util::align_down;

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
    start: usize,
    end: usize,
    used: usize,
    tag_used: [usize; MemTag::len() as usize],
    wilderness: usize,
    wilderness_end: usize,
    bins: [LinkedList; 30],
    reserved_regions: ArrayVec<[(usize, usize); 32]>,
}

fn has_alignment(ptr: usize, align: usize) -> bool {
    ptr % align == 0
}

fn region_overlap(a: (u64, u64), b: (u64, u64)) -> bool {
    // (start, size) -> region: [start, start+size)
    // ref: https://stackoverflow.com/a/325964
    a.0 < b.0 + b.1 && b.0 < a.0 + a.1
}

impl Allocator {
    /// Creates a new bin allocator that will allocate memory from the region
    /// starting at address `start` and ending at address `end`.
    pub fn new(start: usize, end: usize) -> Allocator {
        // println!("BinAlloc::new(0x{:x}, 0x{:x})", start, end);
        Allocator {
            start: align_up(start, 8),
            end,
            used: 0,
            tag_used: [0; MemTag::len() as usize],
            wilderness: align_up(start, 8),
            wilderness_end: end,
            bins: [LinkedList::new(); 30],
            reserved_regions: ArrayVec::new(),
        }
    }

    pub fn wilderness(&self) -> (usize, usize) {
        (self.wilderness, self.wilderness_end)
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
    fn fill_allocations_until(&mut self, end: usize) -> bool {

        'fill_loop: while self.wilderness != end {
            assert!(end - self.wilderness > 7);

            for i in (0..self.bins.len()).rev() {
                let bin_size = self.bin_size(i);
                if self.wilderness + bin_size < end && has_alignment(self.wilderness, bin_size) {
                    // will not recurse because this is a perfect fit.
                    self.allocate_bin_entry(i);
                    continue 'fill_loop;
                }
            }

            return false;
        }

        true
    }

    fn allocate_bin_entry(&mut self, bin: usize) -> bool {
        // notes:
        // - "end" / right side boundaries are all exclusive bounds.
        //

        loop {
            let alloc_start = align_up(self.wilderness, self.bin_size(bin));
            let bin_size = self.bin_size(bin);

            if alloc_start + bin_size > self.wilderness_end {
                // There is no way we can allocate.
                return false;
            }

            if let Some(res_region) = self.reserved_regions.first().cloned() as Option<(usize, usize)> {
                if alloc_start + bin_size > res_region.0 {
                    // allocation passes into a reserved region.

                    self.reserved_regions.remove(0);

                    self.fill_allocations_until(res_region.0);
                    self.wilderness = res_region.0 + res_region.1;

                    // restart allocation at the top of the next region.
                    continue;
                }
            }

            self.fill_allocations_until(alloc_start);

            self.wilderness = alloc_start + bin_size;

            unsafe { self.bins[bin].push(alloc_start as *mut usize) };

            return true;
        }
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

    fn do_alloc(&mut self, layout: Layout, tag: MemTag) -> Option<*mut u8> {
        let bin = self.layout_to_bin(layout);

        // println!("[alloc] layout(size:0x{:x}, align:0x{:x}) -> bin:{}, bin_size:0x{:x}", layout.size(), layout.align(), bin, self.bin_size(bin));

        if let Some(p) = self.bins[bin].pop() {
            // println!("[alloc] served with fastbin:{}", bin);
            self.used += self.bin_size(bin);
            self.tag_used[tag as u8 as usize] += self.bin_size(bin);
            return Some(p as *mut u8);
        }

        if !self.scavenge_bin(bin) {
            // println!("[alloc] failed to scavenge");
            return None;
        }

        self.used += self.bin_size(bin);
        self.tag_used[tag as u8 as usize] += self.bin_size(bin);
        Some(self.bins[bin].pop().unwrap() as *mut u8)
    }

    fn do_dealloc(&mut self, ptr: *mut u8, layout: Layout, tag: MemTag) {
        let bin = self.layout_to_bin(layout);
        self.used -= self.bin_size(bin);
        self.tag_used[tag as u8 as usize] -= self.bin_size(bin);

        // cast is safe because we only ever give out 8 byte aligned pointers
        // anyway.
        unsafe { self.bins[bin].push(ptr as *mut usize) };
    }

    pub fn register_reserved_region(&mut self, region: (usize, usize)) -> bool {
        let aligned_start =  align_down(region.0, 8);
        let aligned_size = align_up(region.0 + region.1, 8) - aligned_start;

        self.reserved_regions.push((aligned_start, aligned_size));

        // does not allocate.
        self.reserved_regions.sort_unstable();

        // return false if the allocator is unable / has already violated the reserved region.
        aligned_start >= self.wilderness
    }

}

impl TaggingAlloc for Allocator {
    unsafe fn alloc_tag(&mut self, layout: Layout, tag: MemTag) -> *mut u8 {
        self.do_alloc(layout, tag).unwrap_or(0 as *mut u8)
    }

    unsafe fn dealloc_tag(&mut self, ptr: *mut u8, layout: Layout, tag: MemTag) {
        self.do_dealloc(ptr, layout, tag);
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
        let x = self.do_alloc(layout, MemTag::Global).unwrap_or(0 as *mut u8);
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
        self.do_dealloc(ptr, layout, MemTag::Global);
    }
}

impl AllocStats for Allocator {
    fn total_allocation(&self) -> (usize, usize) {
        (self.used, self.end - self.start)
    }

    fn dump(&self, w: &mut io::Write) -> io::Result<()> {
        writeln!(w, "Allocator")?;

        let (allocated, total) = self.total_allocation();

        writeln!(w, "allocated: {}", allocated)?;
        writeln!(w, "total: {}", total)?;
        writeln!(w, "percent: {}%", 100.0 * (allocated as f64) / (total as f64))?;

        writeln!(w, "Tags:")?;
        for i in 0..MemTag::len() {
            let tag = MemTag::from(i);
            writeln!(w, "  {:?}: {}%", tag, 100.0 * (self.tag_used[i as usize] as f64) / (total as f64))?;
        }

        Ok(())
    }
}


