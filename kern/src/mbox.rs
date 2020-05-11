use core::borrow::{Borrow, BorrowMut};
use core::time::Duration;

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


pub struct EveryTimer {
    every: Duration,
    last: Option<Duration>,
}

impl EveryTimer {
    pub fn new(every: Duration) -> Self {
        Self {
            every,
            last: None,
        }
    }

    pub fn every<F: FnOnce()>(&mut self, func: F) {
        let now = pi::timer::current_time();
        if let Some(last) = self.last {
            let diff = now - last;
            if diff >= self.every {
                self.last = Some(now);
                func();
            }
        } else {
            self.last = Some(now);
            func();
        }
    }
}



