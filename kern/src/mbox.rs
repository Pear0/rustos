use core::borrow::{Borrow, BorrowMut};

use mini_alloc::MiniBox;
use pi::mbox::MBox;

use crate::mini_allocators::NOCACHE_ALLOC;

pub fn with_mbox<F, R>(f: F) -> R
    where
        F: FnOnce(&mut MBox) -> R,
{
    let mut mbox = MiniBox::new(&NOCACHE_ALLOC, unsafe { MBox::new() });
    f(&mut mbox)
}




