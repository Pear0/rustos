use alloc::boxed::Box;
use alloc::fmt;
use core::alloc::{GlobalAlloc, Layout};
use core::fmt::Formatter;
use core::iter::Chain;
use core::ops::{Deref, DerefMut};
use core::ops::Sub;
use core::slice::Iter;

use aarch64::vmsa::*;
use aarch64::vmsa::EntryPerm::{KERN_RW, USER_RW};
use shim::const_assert_size;

use crate::allocator;
use crate::ALLOCATOR;
use crate::param::*;
use crate::vm::{PhysicalAddr, VirtualAddr};
use crate::traps::HyperTrapFrame;
use crate::process::HyperProcess;

#[repr(C)]
pub struct Page([u8; PAGE_SIZE]);
const_assert_size!(Page, PAGE_SIZE);

impl Page {
    pub const SIZE: usize = PAGE_SIZE;
    pub const ALIGN: usize = PAGE_SIZE;

    fn layout() -> Layout {
        unsafe { Layout::from_size_align_unchecked(Self::SIZE, Self::ALIGN) }
    }
}

const L2_PAGES: usize = 3;

#[repr(C)]
#[repr(align(65536))]
pub struct L2PageTable {
    pub entries: [RawL2Entry; 8192],
}
const_assert_size!(L2PageTable, PAGE_SIZE);

impl L2PageTable {
    /// Returns a new `L2PageTable`
    fn new() -> L2PageTable {
        L2PageTable {
            entries: [RawL2Entry::new(0); 8192],
        }
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
        PhysicalAddr::from(self as *const Self as usize)
    }
}

#[derive(Copy, Clone)]
pub struct L3Entry(RawL3Entry);

impl L3Entry {
    /// Returns a new `L3Entry`.
    fn new() -> L3Entry {
        L3Entry(RawL3Entry::new(0))
    }

    fn reset(&mut self) {
        self.0 = RawL3Entry::new(0);
    }

    /// Returns `true` if the L3Entry is valid and `false` otherwise.
    fn is_valid(&self) -> bool {
        self.0.get_value(RawL3Entry::VALID) != 0
    }

    /// Extracts `ADDR` field of the L3Entry and returns as a `PhysicalAddr`
    /// if valid. Otherwise, return `None`.
    fn get_page_addr(&self) -> Option<PhysicalAddr> {
        match self.is_valid() {
            false => None,
            true => Some(PhysicalAddr::from((self.0.get_value(RawL3Entry::ADDR) << 16) as usize)),
        }
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct L3PageTable {
    pub entries: [L3Entry; 8192],
}
const_assert_size!(L3PageTable, PAGE_SIZE);

impl L3PageTable {
    /// Returns a new `L3PageTable`.
    fn new() -> L3PageTable {
        L3PageTable {
            entries: [L3Entry::new(); 8192],
        }
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
        PhysicalAddr::from(self as *const Self as usize)
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct PageTable {
    pub l2: L2PageTable,
    pub l3: [Box<L3PageTable>; L2_PAGES],
}

impl PageTable {
    /// Returns a new `Box` containing `PageTable`.
    /// Entries in L2PageTable should be initialized properly before return.
    fn new(perm: u64) -> Box<PageTable> {
        let mut table = Box::new(PageTable {
            l2: L2PageTable::new(),
            l3: [Box::new(L3PageTable::new()), Box::new(L3PageTable::new()), Box::new(L3PageTable::new())],
        });

        for (i, l3) in table.l3.iter().enumerate() {
            table.l2.entries[i].set_value(l3.as_ptr().as_u64() >> PAGE_ALIGN, RawL2Entry::ADDR);
            table.l2.entries[i].set_value(perm, RawL2Entry::AP);
            table.l2.entries[i].set_value(EntryValid::Valid, RawL2Entry::VALID);
            table.l2.entries[i].set_value(EntryType::Table, RawL2Entry::TYPE);
            table.l2.entries[i].set_value(EntryAttr::Mem, RawL2Entry::ATTR);
            table.l2.entries[i].set_value(EntrySh::ISh, RawL2Entry::SH);
            table.l2.entries[i].set_value(1, RawL2Entry::AF);
            table.l2.entries[i].set_value(1, RawL3Entry::NS);
        }

        table
    }

    /// Returns the (L2index, L3index) extracted from the given virtual address.
    /// Since we are only supporting 1GB virtual memory in this system, L2index
    /// should be smaller than 2.
    ///
    /// # Panics
    ///
    /// Panics if the virtual address is not properly aligned to page size.
    /// Panics if extracted L2index exceeds the number of L3PageTable.
    fn locate(va: VirtualAddr) -> (usize, usize) {
        let mut addr = va.as_u64();
        if addr as usize % PAGE_SIZE != 0 {
            panic!("Address: {:x} is not aligned", addr);
        }

        addr = addr >> 16;

        let l3 = addr & (0b1_1111_1111_1111); // 13 bits

        addr = addr >> 13;

        let l2 = addr & (0b1_1111_1111_1111); // 13 bits

        if l2 >= L2_PAGES as u64 {
            panic!("Address: {:x} -> L2 invalid: {:x}", va.as_u64(), l2);
        }

        (l2 as usize, l3 as usize)
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is valid.
    /// Otherwise, `false` is returned.
    pub fn is_valid(&self, va: VirtualAddr) -> bool {
        let (l2, l3) = PageTable::locate(va);
        self.l3[l2].entries[l3].is_valid()
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is invalid.
    /// Otherwise, `true` is returned.
    pub fn is_invalid(&self, va: VirtualAddr) -> bool {
        !self.is_valid(va)
    }

    /// Set the given RawL3Entry `entry` to the L3Entry indicated by the given virtual
    /// address.
    pub fn set_entry(&mut self, va: VirtualAddr, entry: RawL3Entry) -> &mut Self {
        let (l2, l3) = PageTable::locate(va);
        self.l3[l2].entries[l3].0 = entry;
        self
    }

    pub fn get_entry(&self, va: VirtualAddr) -> &L3Entry {
        let (l2, l3) = PageTable::locate(va);
        &self.l3[l2].entries[l3]
    }

    pub fn get_entry_mut(&mut self, va: VirtualAddr) -> &mut L3Entry {
        let (l2, l3) = PageTable::locate(va);
        &mut self.l3[l2].entries[l3]
    }

    /// Returns a base address of the pagetable. The returned `PhysicalAddr` value
    /// will point the start address of the L2PageTable.
    pub fn get_baddr(&self) -> PhysicalAddr {
        self.l2.as_ptr()
    }
}

impl<'a> IntoIterator for &'a PageTable {
    type Item = &'a L3Entry;
    type IntoIter = Chain<Chain<Iter<'a, L3Entry>, Iter<'a, L3Entry>>, Iter<'a, L3Entry>>;

    fn into_iter(self) -> Self::IntoIter {
        self.l3[0].entries.iter().chain(self.l3[1].entries.iter()).chain(self.l3[2].entries.iter())
    }
}

pub struct KernPageTable(Box<PageTable>);

impl KernPageTable {
    /// Returns a new `KernPageTable`. `KernPageTable` should have a `Pagetable`
    /// created with `KERN_RW` permission.
    ///
    /// Set L3entry of ARM physical address starting at 0x00000000 for RAM and
    /// physical address range from `IO_BASE` to `IO_BASE_END` for peripherals.
    /// Each L3 entry should have correct value for lower attributes[10:0] as well
    /// as address[47:16]. Refer to the definition of `RawL3Entry` in `vmsa.rs` for
    /// more details.
    pub fn new() -> KernPageTable {
        let mut table = PageTable::new(KERN_RW);

        let (_, end) = allocator::memory_map().expect("failed to memory map");
        let end = allocator::util::align_down(end, PAGE_SIZE);

        // Correct page type is chosen by address region. We just have to init.
        for addr in (0..end).step_by(PAGE_SIZE) {
            // not act
            table.set_entry(VirtualAddr::from(addr), KernPageTable::create_l3_entry(addr, EntryAttr::Mem));
        }

        for addr in (IO_BASE..IO_BASE_END).step_by(PAGE_SIZE) {
            // attr not actually used here
            table.set_entry(VirtualAddr::from(addr), KernPageTable::create_l3_entry(addr, EntryAttr::Dev));
        }

        KernPageTable(table)
    }

    pub fn dump(&self) {
        kprintln!("PageTable:");
        for (i, entry) in self.0.l2.entries.iter().enumerate() {
            if entry.get() != 0 {

                let addr = entry.get_value(RawL2Entry::ADDR) << 16;

                kprintln!("index = {}, addr = {:x}, value = {:x}", i, addr, entry.get());

                if addr != 0 {
                    let l3: &L3PageTable = unsafe { &*(addr as *const L3PageTable) };

                    for (i, e) in l3.entries.iter().enumerate() {
                        kprintln!("  index = {}, value = {:x}", i, e.0.get());
                    }
                }

            }
        }
    }

    pub fn create_l3_entry(addr: usize, attr: u64) -> RawL3Entry {
        assert_eq!(addr % PAGE_SIZE, 0);

        let mut entry = RawL3Entry::new(0);
        entry.set_value(EntryValid::Valid, RawL3Entry::VALID);
        entry.set_value(EntryType::Table, RawL3Entry::TYPE);
        entry.set_value(EntryPerm::KERN_RW, RawL3Entry::AP);
        entry.set_value(1, RawL3Entry::AF);
        entry.set_value(1, RawL3Entry::NS);

        if addr >= IO_BASE && addr < IO_BASE_END {
            entry.set_value(EntrySh::OSh, RawL3Entry::SH);
            entry.set_value(EntryAttr::Dev, RawL3Entry::ATTR);
        } else {
            entry.set_value(EntrySh::ISh, RawL3Entry::SH);
            // FIXME caching disabled so that MBox works properly.
            entry.set_value(attr, RawL3Entry::ATTR);
        }

        entry.set_value((addr >> PAGE_ALIGN) as u64, RawL3Entry::ADDR);

        entry
    }

    pub fn get_baddr(&self) -> PhysicalAddr {
        self.0.get_baddr()
    }

}

pub enum PagePerm {
    RW,
    RO,
    RWX,
}

pub trait GuestPageTable: Sized {
    fn new() -> Self;

    fn get_baddr(&self) -> PhysicalAddr;

    unsafe fn get_page_ref(&self, va: VirtualAddr) -> Option<&mut [u8]>;

    fn is_valid(&self, va: VirtualAddr) -> bool;

    fn alloc(&mut self, va: VirtualAddr, _perm: PagePerm) -> &mut [u8];

}

pub struct UserPageTable(Box<PageTable>);

impl UserPageTable {

    pub fn iter_mapped_pages<'a>(&'a self) -> impl Iterator<Item=(VirtualAddr, PhysicalAddr)> + 'a {
        self.0.l3.iter()
            .flat_map(|l3| l3.entries.iter())
            .enumerate()
            .map(|(i, entry)| (VirtualAddr::from(USER_IMG_BASE + i * PAGE_SIZE), entry.get_page_addr()))
            .filter_map(|(i, e)| {
                match e {
                    Some(s) => Some((i, s)),
                    None => None,
                }
            })
    }

    fn as_va_sub(va: VirtualAddr) -> VirtualAddr {
        if va.as_usize() < USER_IMG_BASE {
            panic!("Tried to create user page below USER_IMG_BASE: {:x}", va.as_usize());
        }

        va.sub(VirtualAddr::from(USER_IMG_BASE))
    }

    pub fn dealloc(&mut self, va: VirtualAddr) -> bool {
        let entry = self.0.get_entry_mut(Self::as_va_sub(va));

        if entry.is_valid() {
            let addr = entry.0.get_value(RawL3Entry::ADDR) << 16;
            unsafe { ALLOCATOR.dealloc(addr as *mut u8, Page::layout()) }
            entry.reset();
            true
        } else {
            false
        }
    }

    pub unsafe fn get_page_ref(&self, va: VirtualAddr) -> Option<&mut [u8]> {
        assert_eq!(va.as_usize() % PAGE_SIZE, 0);

        let entry = self.0.get_entry(Self::as_va_sub(va));
        if !entry.is_valid() {
            return None;
        }

        let addr = entry.0.get_value(RawL3Entry::ADDR) << 16;
        Some(unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, PAGE_SIZE) })
    }

    pub fn get_baddr(&self) -> PhysicalAddr {
        self.0.get_baddr()
    }

}

impl GuestPageTable for UserPageTable {
    /// Returns a new `UserPageTable` containing a `PageTable` created with
    /// `USER_RW` permission.
    fn new() -> UserPageTable {
        UserPageTable(PageTable::new(USER_RW))
    }


    fn get_baddr(&self) -> PhysicalAddr {
        self.0.get_baddr()
    }

    unsafe fn get_page_ref(&self, va: VirtualAddr) -> Option<&mut [u8]> {
        assert_eq!(va.as_usize() % PAGE_SIZE, 0);

        let entry = self.0.get_entry(Self::as_va_sub(va));
        if !entry.is_valid() {
            return None;
        }

        let addr = entry.0.get_value(RawL3Entry::ADDR) << 16;
        Some(unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, PAGE_SIZE) })
    }

    fn is_valid(&self, va: VirtualAddr) -> bool {
        self.0.is_valid(Self::as_va_sub(va))
    }

    /// Allocates a page and set an L3 entry translates given virtual address to the
    /// physical address of the allocated page. Returns the allocated page.
    ///
    /// # Panics
    /// Panics if the virtual address is lower than `USER_IMG_BASE`.
    /// Panics if the virtual address has already been allocated.
    /// Panics if allocator fails to allocate a page.
    ///
    /// TODO. use Result<T> and make it failurable
    /// TODO. use perm properly
    fn alloc(&mut self, va: VirtualAddr, _perm: PagePerm) -> &mut [u8] {
        let va_sub = Self::as_va_sub(va);

        if self.is_valid(va) {
            self.dealloc(va);
            error!("allocating over an already allocated page: {:x}", va.as_usize());
        }

        let mut entry = RawL3Entry::new(0);
        entry.set_value(EntryValid::Valid, RawL3Entry::VALID);
        entry.set_value(EntryType::Table, RawL3Entry::TYPE);
        entry.set_value(EntryPerm::USER_RW, RawL3Entry::AP);

        entry.set_value(EntrySh::ISh, RawL3Entry::SH);
        entry.set_value(EntryAttr::Mem, RawL3Entry::ATTR);

        entry.set_value(1, RawL3Entry::AF);
        entry.set_value(1, RawL3Entry::NS);

        let alloc = unsafe { ALLOCATOR.alloc(Page::layout()) };

        entry.set_value((alloc as u64) >> 16, RawL3Entry::ADDR);

        self.0.set_entry(va_sub, entry);

        unsafe { core::slice::from_raw_parts_mut(alloc, PAGE_SIZE) }
    }
}

impl Deref for KernPageTable {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KernPageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for UserPageTable {
    fn drop(&mut self) {

        for l3 in self.0.l3.iter_mut() {
            for entry in l3.entries.iter_mut() {
                if entry.is_valid() {
                    let addr = entry.0.get_value(RawL3Entry::ADDR) << 16;
                    unsafe { ALLOCATOR.dealloc(addr as *mut u8, Page::layout()) }
                }
            }
        }

    }
}

impl fmt::Debug for UserPageTable {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserPageTable")
            .finish()
    }
}

pub struct VirtualizationPageTable(Box<PageTable>);

impl VirtualizationPageTable {

    pub fn iter_mapped_pages<'a>(&'a self) -> impl Iterator<Item=(VirtualAddr, PhysicalAddr)> + 'a {
        self.0.l3.iter()
            .flat_map(|l3| l3.entries.iter())
            .enumerate()
            .map(|(i, entry)| (VirtualAddr::from(i * PAGE_SIZE), entry.get_page_addr()))
            .filter_map(|(i, e)| {
                match e {
                    Some(s) => Some((i, s)),
                    None => None,
                }
            })
    }

    fn as_va_sub(va: VirtualAddr) -> VirtualAddr {
        va
    }

    fn template_page() -> RawL3Entry {
        let mut entry = RawL3Entry::new(0);
        entry.set_value(EntryValid::Valid, RawL3Entry::VALID);
        entry.set_value(EntryType::Table, RawL3Entry::TYPE);

        // 00 = none, 01 = read-only, 10 = write-only, 11 = R/W
        entry.set_value(0b11, RawS2L3Entry::S2AP);

        entry.set_value(EntrySh::ISh, RawL3Entry::SH);
        // entry.set_value(EntryAttr::Mem, RawL3Entry::ATTR);

        // D4.5.2 : 1111 leaves stage 1 rules unchanged
        // entry.set_value(0b0001, RawS2L3Entry::MEM_ATTR);
        entry.set_value(0b1111, RawS2L3Entry::MEM_ATTR);

        entry.set_value(0, RawL3Entry::AF);

        entry
    }

    pub fn map_direct(&mut self, va: VirtualAddr, pa: PhysicalAddr) -> &mut [u8] {
        let va_sub = Self::as_va_sub(va);

        if self.is_valid(va) {
            error!("allocating over an already allocated page: {:x}", va.as_usize());
        }

        assert_eq!(pa.as_usize() % PAGE_SIZE, 0);

        let mut entry = Self::template_page();

        let alloc = pa.as_usize() as *mut u8;

        entry.set_value((alloc as u64) >> 16, RawL3Entry::ADDR);

        self.0.set_entry(va_sub, entry);

        unsafe { core::slice::from_raw_parts_mut(alloc, PAGE_SIZE) }
    }

    pub fn mark_accessed(&mut self, va: VirtualAddr) {
        let va_sub = Self::as_va_sub(va);

        if !self.is_valid(va) {
            error!("cannot mark page accessed: {:x}", va.as_usize());
            return;
        }

        self.0.get_entry_mut(va_sub).0.set_bit(RawL3Entry::AF);

        aarch64::clean_data_cache((&mut self.0.get_entry_mut(va_sub).0) as *mut RawL3Entry as u64);

    }

    pub fn dealloc(&mut self, va: VirtualAddr) -> bool {
        let entry = self.0.get_entry_mut(Self::as_va_sub(va));

        if entry.is_valid() {
            let addr = entry.0.get_value(RawL3Entry::ADDR) << 16;
            unsafe { ALLOCATOR.dealloc(addr as *mut u8, Page::layout()) }
            entry.reset();
            true
        } else {
            false
        }
    }

}

impl GuestPageTable for VirtualizationPageTable {
    /// Returns a new `UserPageTable` containing a `PageTable` created with
    /// `USER_RW` permission.
    fn new() -> VirtualizationPageTable {
        VirtualizationPageTable(PageTable::new(USER_RW))
    }

    fn get_baddr(&self) -> PhysicalAddr {
        self.0.get_baddr()
    }

    unsafe fn get_page_ref(&self, va: VirtualAddr) -> Option<&mut [u8]> {
        assert_eq!(va.as_usize() % PAGE_SIZE, 0);

        let entry = self.0.get_entry(Self::as_va_sub(va));
        if !entry.is_valid() {
            return None;
        }

        let addr = entry.0.get_value(RawL3Entry::ADDR) << 16;
        Some(unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, PAGE_SIZE) })
    }

    fn is_valid(&self, va: VirtualAddr) -> bool {
        self.0.is_valid(Self::as_va_sub(va))
    }

    /// Allocates a page and set an L3 entry translates given virtual address to the
    /// physical address of the allocated page. Returns the allocated page.
    ///
    /// # Panics
    /// Panics if the virtual address is lower than `USER_IMG_BASE`.
    /// Panics if the virtual address has already been allocated.
    /// Panics if allocator fails to allocate a page.
    ///
    /// TODO. use Result<T> and make it failurable
    /// TODO. use perm properly
    fn alloc(&mut self, va: VirtualAddr, _perm: PagePerm) -> &mut [u8] {
        let va_sub = Self::as_va_sub(va);

        if self.is_valid(va) {
            self.dealloc(va);
            error!("allocating over an already allocated page: {:x}", va.as_usize());
        }

        let mut entry = Self::template_page();

        let alloc = unsafe { ALLOCATOR.alloc(Page::layout()) };

        entry.set_value((alloc as u64) >> 16, RawL3Entry::ADDR);

        self.0.set_entry(va_sub, entry);

        aarch64::clean_data_cache((&mut self.0.get_entry_mut(va_sub).0) as *mut RawL3Entry as u64);

        unsafe { core::slice::from_raw_parts_mut(alloc, PAGE_SIZE) }
    }

}


impl Drop for VirtualizationPageTable {
    fn drop(&mut self) {

        for l3 in self.0.l3.iter_mut() {
            for entry in l3.entries.iter_mut() {
                if entry.is_valid() {
                    let addr = entry.0.get_value(RawL3Entry::ADDR) << 16;
                    unsafe { ALLOCATOR.dealloc(addr as *mut u8, Page::layout()) }
                }
            }
        }

    }
}

impl fmt::Debug for VirtualizationPageTable {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserPageTable")
            .finish()
    }
}

