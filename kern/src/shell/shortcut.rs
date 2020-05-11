use alloc::boxed::Box;
use alloc::sync::Arc;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

use crate::console::console_set_callback;
use crate::kernel_call::syscall::wait_waitable;
use crate::sync::Waitable;

pub struct WaitFlag(AtomicBool);

impl WaitFlag {
    pub fn new() -> Arc<Self> {
        Arc::new(Self(AtomicBool::new(false)))
    }

    pub fn is_flagged(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub fn flag(&self) {
        self.0.store(true, Ordering::SeqCst)
    }
}

impl Waitable for WaitFlag {
    fn done_waiting(&self) -> bool {
        self.is_flagged()
    }
}

pub fn sleep_until_key(key: u8) {
    let flag = WaitFlag::new();

    let flag_copy = flag.clone();
    console_set_callback(Some((key, Box::new(move || {
        flag_copy.flag();
        false
    }))));

    wait_waitable(flag);
}



