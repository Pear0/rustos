#![feature(const_fn)]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

use core::alloc::GlobalAlloc;
use core::marker::PhantomData;
use alloc::alloc::Layout;
use core::cell::UnsafeCell;
use core::ops::Deref;

pub trait ThreadLocal<T: Default> {
    unsafe fn get_mut(&self) -> &mut T;
}

pub trait Hooks {
    type TL: ThreadLocal<Option<&'static dyn GlobalAlloc>>;

}

struct AllocInner<H: Hooks> {
    default: &'static dyn GlobalAlloc,
    current: H::TL,
    __phantom: PhantomData<H>,
}

impl<H: Hooks> AllocInner<H> {

    pub fn get_alloc(&self) -> &'static dyn GlobalAlloc {
        let default = self.default;
        unsafe { self.current.get_mut().as_ref().map(|x| *x).unwrap_or(default) }
    }

    pub fn with_allocator<R, F: FnOnce() -> R>(&self, allocator: &'static dyn GlobalAlloc, func: F) -> R {
        unsafe {
            let old_alloc = core::mem::replace(self.current.get_mut(), Some(allocator));
            let result = func();
            *self.current.get_mut() = old_alloc;
            result
        }
    }

}

pub struct Allocator<H: Hooks> {
    inner: UnsafeCell<Option<AllocInner<H>>>,
}

unsafe impl<H: Hooks> Sync for Allocator<H> {}

impl<H: Hooks> Allocator<H> {
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(None),
        }
    }

    pub unsafe fn initialize(&self, default: &'static dyn GlobalAlloc, thread_local: H::TL) {

        let inner = AllocInner::<H> {
            default,
            current: thread_local,
            __phantom: PhantomData,
        };

        unsafe { *self.inner.get() = Some(inner) };
    }

    fn inner(&self) -> Option<&AllocInner<H>> {
        unsafe { ( *self.inner.get()).as_ref() }
    }

    pub fn with_allocator<R, F: FnOnce() -> R>(&self, allocator: &'static dyn GlobalAlloc, func: F) -> R {
        self.inner().expect("mpalloc::with_allocator called before initialize()").with_allocator(allocator, func)
    }

}

struct AllocFooter {
    my_allocator: &'static dyn GlobalAlloc,
}

impl AllocFooter {
    fn modify_layout(layout: Layout) -> Layout {
        let mut size = layout.size();
        let mut align = layout.align();

        // make sure footer is aligned
        size = align_up(size, core::mem::size_of::<AllocFooter>());

        // add size of footer
        size += core::mem::size_of::<AllocFooter>();


        if align < core::mem::align_of::<AllocFooter>() {
            align = core::mem::align_of::<AllocFooter>();
        }

        Layout::from_size_align(size, align).unwrap()
    }

    unsafe fn get_footer(ptr: *mut u8, layout: Layout) -> &'static mut AllocFooter {
        let addr = ptr as usize + align_up(layout.size(), core::mem::size_of::<AllocFooter>());
        &mut *(addr as *mut AllocFooter)
    }
}

unsafe impl<H: Hooks> GlobalAlloc for Allocator<H> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self.inner() {
            Some(inner) => {
                let chosen_alloc = inner.get_alloc();
                let addr = chosen_alloc.alloc(AllocFooter::modify_layout(layout));
                if !addr.is_null() {
                    let mut footer = AllocFooter::get_footer(addr, layout);
                    footer.my_allocator = chosen_alloc;
                }
                addr
            },
            None => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut footer = AllocFooter::get_footer(ptr, layout);
        let chosen_allocator = footer.my_allocator;
        chosen_allocator.dealloc(ptr, AllocFooter::modify_layout(layout));
    }
}

pub static NULL_ALLOC: NullAlloc = NullAlloc;

pub struct NullAlloc;

unsafe impl GlobalAlloc for NullAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    }
}


fn align_down(addr: usize, align: usize) -> usize {
    if align.count_ones() != 1 {
        panic!("invalid align: {}", align);
    }

    addr & !(align - 1)
}

fn align_up(addr: usize, align: usize) -> usize {
    if align.count_ones() != 1 {
        panic!("invalid align: {}", align);
    }

    if align_down(addr, align) < addr {
        align_down(addr, align) + align
    } else {
        align_down(addr, align)
    }
}


pub trait ThreadLocalAlloc: Default {

    unsafe fn alloc(&mut self, layout: Layout, del: &'static dyn GlobalAlloc) -> *mut u8;

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout, del: &'static dyn GlobalAlloc);

}

pub struct ThreadedAlloc<T: ThreadLocalAlloc, TL: ThreadLocal<T>> {
    local: TL,
    delegate: UnsafeCell<Option<&'static dyn GlobalAlloc>>,
    __phantom: PhantomData<T>,
}

unsafe impl<T: ThreadLocalAlloc, TL: ThreadLocal<T>> Sync for ThreadedAlloc<T, TL> {}

impl<T: ThreadLocalAlloc, TL: ThreadLocal<T>> ThreadedAlloc<T, TL> {

    pub const fn new(local: TL) -> Self {
        Self {
            local,
            delegate: UnsafeCell::new(None),
            __phantom: PhantomData,
        }
    }

}

impl<T: ThreadLocalAlloc, TL: ThreadLocal<T>> ThreadedAlloc<T, TL> {
    pub unsafe fn set_delegate(&self, del: &'static dyn GlobalAlloc) {
        (&mut *self.delegate.get()).replace(del);
    }

    fn get_delegate(&self) -> Option<&'static dyn GlobalAlloc> {
        unsafe { &*self.delegate.get() }.as_ref().cloned()
    }
}

unsafe impl<T: ThreadLocalAlloc, TL: ThreadLocal<T>> GlobalAlloc for ThreadedAlloc<T, TL> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let del = self.get_delegate().expect("no delegate");
        self.local.get_mut().alloc(layout, del)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let del = self.get_delegate().expect("no delegate");
        self.local.get_mut().dealloc(ptr, layout, del)
    }
}










