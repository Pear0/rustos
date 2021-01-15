use alloc::boxed::Box;
use alloc::sync::Arc;
use core::time::Duration;

use kernel_api::OsResult;

use crate::kernel_call::*;
use crate::sync::Waitable;

/// pretty much requires coerce_unsized feature to be usable.
pub fn wait_waitable(arc: Arc<dyn Waitable>) {
    let mut ecode: u64;
    let mut elapsed_ms: u64;

    let arc: [u64; 2] = unsafe { core::mem::transmute(arc) };

    unsafe {
        llvm_asm!("mov x0, $2
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

pub fn yield_for_timers() {
    unsafe {
        llvm_asm!("svc $0" :: "i"(NR_YIELD_FOR_TIMERS) :: "volatile");
    }
}

pub struct ExcContext {
    pub pid: u64,
}

pub struct ExecInExcPayload<'a> {
    func: Option<Box<dyn FnOnce(&mut ExcContext) + 'a>>,
}

impl<'a> ExecInExcPayload<'a> {
    pub fn execute(&mut self, exc: &mut ExcContext) {
        if let Some(func) = self.func.take() {
            func(exc);
        }
    }
}

pub fn exec_in_exc<R, F>(func: F) -> R where F: FnOnce(&mut ExcContext) -> R {
    let mut result: Option<R> = None;

    let mut payload = ExecInExcPayload::<'_> {
        func: Some(Box::new(|exc| {
            result = Some(func(exc));
        }))
    };

    let ptr = (&mut payload) as *mut _ as u64;

    unsafe {
        llvm_asm!("mov x0, $0
                   svc $1"
             :
             : "r"(ptr), "i"(NR_EXEC_IN_EXC)
             : "x0"
             : "volatile");
    }

    drop(payload);

    result.expect("payload was executed in exception context")
}
