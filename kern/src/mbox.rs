use crate::mutex::Mutex;

use crate::{ALLOCATOR, VMM};
use alloc::alloc::{GlobalAlloc, Layout};
use crate::param::{PAGE_SIZE, PAGE_ALIGN};
use core::borrow::{Borrow, BorrowMut};
use pi::mbox::MBox;

const MBOX_PAGE: Mutex<Option<&mut MBox>> = Mutex::new(None);

pub fn with_mbox<F, R>(f: F) -> R
    where
        F: FnOnce(&mut MBox) -> R,
{
    let mbox_page = &MBOX_PAGE;
    let mut guard = mbox_page.lock();

    if guard.is_none() {
        let mem = unsafe { ALLOCATOR.alloc( Layout::from_size_align_unchecked(PAGE_SIZE, PAGE_SIZE)) };

        unsafe { VMM.mark_page_non_cached(mem as usize); }

        guard.replace(unsafe { &mut *(mem as *mut MBox) });

    }
    f(guard.as_mut().unwrap())
}




