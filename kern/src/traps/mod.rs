use alloc::vec::Vec;

use aarch64::MPIDR_EL1;
use pi::interrupt::{Controller, CoreInterrupt, Interrupt};

use crate::{debug, shell, smp};
use crate::cls::CoreLocal;
use crate::kernel::{KERNEL_IRQ, KERNEL_SCHEDULER};
use crate::process::State;
use crate::traps::Kind::Synchronous;

pub use self::frame::{Frame, HyperTrapFrame, KernelTrapFrame};
use self::syndrome::Fault;
use self::syndrome::Syndrome;
use self::syscall::handle_syscall;

mod frame;
mod hyper;
mod hypercall;
mod kernel;
// defines kernel_handle_exception
pub mod syndrome;
mod syscall;

pub mod irq;

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,

    None = 4,
}

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Source {
    CurrentSpEl0 = 0,
    CurrentSpElx = 1,
    LowerAArch64 = 2,
    LowerAArch32 = 3,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Info {
    source: Source,
    kind: Kind,
}

pub static IRQ_RECURSION_DEPTH: CoreLocal<i32> = CoreLocal::new_copy(0);
pub static IRQ_ESR: CoreLocal<u32> = CoreLocal::new_copy(0xFF_FF_FF_FF);
pub static IRQ_EL: CoreLocal<u64> = CoreLocal::new_copy(0xFF_FF_FF_FF);
pub static IRQ_INFO: CoreLocal<Info> = CoreLocal::new_copy(Info { kind: Kind::None, source: Source::CurrentSpEl0 });

pub fn irq_depth() -> i32 {
    IRQ_RECURSION_DEPTH.get()
}

pub fn irq_esr() -> Option<Syndrome> {
    let esr = IRQ_ESR.get();
    if esr != 0xFF_FF_FF_FF {
        Some(Syndrome::from(esr))
    } else {
        None
    }
}

pub fn irq_el() -> Option<u64> {
    let esr = IRQ_EL.get();
    if esr != 0xFF_FF_FF_FF {
        Some(esr)
    } else {
        None
    }
}

pub fn irq_info() -> Info {
    IRQ_INFO.get()
}





