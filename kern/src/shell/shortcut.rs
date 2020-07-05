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
        let a = Arc::new(Self(AtomicBool::new(false)));
        error!("WaitFlag::new() @ {:#x}", a.as_ref() as *const WaitFlag as u64);
        a
    }

    pub fn is_flagged(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub fn flag(&self) {
        // error!("WaitFlag::flag() @ {:#x}", self as *const WaitFlag as u64);
        self.0.store(true, Ordering::SeqCst)
    }
}

impl Drop for WaitFlag {
    fn drop(&mut self) {
        // error!("WaitFlag({})::drop() @ {:#x}", self.0.load(Ordering::Relaxed), self as *mut WaitFlag as u64);
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
        // error!("Ctrl+C pressed");
        false
    }))));

    // error!("WaitFlag1 @ {:#x}", flag.as_ref() as *const WaitFlag as u64);
    // error!("WaitFlag2 @ {:#x}", unsafe { core::mem::transmute::<Arc<WaitFlag>, (usize)>(core::mem::transmute_copy(&flag)) });

    let arc: Arc<dyn Waitable> = flag;
    // error!("WaitFlag3: {:?} @ {:#x?}", arc.done_waiting(), unsafe { core::mem::transmute::<Arc<dyn Waitable>, (usize, usize)>(core::mem::transmute_copy(&arc)) });

    wait_waitable(arc);
    // error!("done waiting");
}



