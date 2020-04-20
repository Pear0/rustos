use crate::mutex::Mutex;

use crate::{ALLOCATOR, VMM};
use alloc::alloc::{GlobalAlloc, Layout};
use crate::param::{PAGE_SIZE, PAGE_ALIGN};
use core::borrow::{Borrow, BorrowMut};
use pi::mbox::MBox;
use mini_alloc::MiniBox;
use crate::mini_allocators::NOCACHE_ALLOC;

pub fn with_mbox<F, R>(f: F) -> R
    where
        F: FnOnce(&mut MBox) -> R,
{
    let mut mbox = MiniBox::new(&NOCACHE_ALLOC, unsafe { MBox::new() });
    f(&mut mbox)
}




