


use kernel_api::OsResult;
use core::time::Duration;
use alloc::sync::Arc;
use crate::sync::Waitable;
use crate::kernel_call::NR_WAIT_WAITABLE;


/// pretty much requires coerce_unsized feature to be usable.
pub fn wait_waitable(arc: Arc<dyn Waitable>) {
    let mut ecode: u64;
    let mut elapsed_ms: u64;

    let arc: [u64; 2] = unsafe { core::mem::transmute(arc) };

    unsafe {
        asm!("mov x0, $2
              mov x1, $3
              svc $4
              mov $0, x0
              mov $1, x7"
             : "=r"(elapsed_ms), "=r"(ecode)
             : "r"(arc[0]), "r"(arc[1]), "i"(NR_WAIT_WAITABLE)
             : "x0", "x7"
             : "volatile");
    }

}

