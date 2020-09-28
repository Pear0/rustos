use core::alloc::Layout;
use crate::allocator::LocalAlloc;
use pi::atags::Mem;

pub trait TaggingAlloc {

    unsafe fn alloc_tag(&mut self, layout: Layout, tag: MemTag) -> *mut u8;
    unsafe fn dealloc_tag(&mut self, ptr: *mut u8, layout: Layout, tag: MemTag);

}


#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(Debug, Clone)]
pub enum MemTag {
    Global = 0,
    CLib,
    NoCacheMini,
    NoCacheDirect,
    __last,
}

impl MemTag {
    pub const fn len() -> u8 {
        MemTag::__last as u8
    }

    pub fn from(num: u8) -> MemTag {
        assert!(num < MemTag::len());
        unsafe { core::mem::transmute(num) }
    }
}



