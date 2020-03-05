use pi::interrupt::{Controller, Interrupt};

use crate::{IRQ, shell};
use crate::console::kprintln;
use crate::traps::Kind::Synchronous;

pub use self::frame::TrapFrame;
use self::syndrome::Syndrome;
use self::syscall::handle_syscall;

mod frame;
mod syndrome;
mod syscall;

pub mod irq;

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,
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

/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
#[no_mangle]
pub extern "C" fn handle_exception(info: Info, esr: u32, tf: &mut TrapFrame) {
    let ctl = Controller::new();

    match info.kind {
        Kind::Irq => {
            for int in Interrupt::iter() {
                if ctl.is_pending(*int) {
                    IRQ.invoke(*int, tf);
                }
            }
        },
        Kind::Synchronous => {
            match Syndrome::from(esr) {
                Syndrome::Svc(svc) => {
                    // kprintln!("svc #{}", svc);
                    handle_syscall(svc, tf);
                }
                Syndrome::Brk(b) => {
                    kprintln!("{:?} {:?}", info, Syndrome::Brk(b));
                    kprintln!("brk #{}", b);
                    shell::shell("#>");
                }
                s => {
                    kprintln!("{:?} {:?} @ {:x}", info, s, tf.elr);
                }
            }
        }
        _ => {
            kprintln!("{:?}", info);
            shell::shell("#>");
        }
    }

    // continue execution
    if info.kind == Synchronous {
        let syndrome = Syndrome::from(esr);
        if let Syndrome::Brk(_) = syndrome {
            tf.elr += 4;
        }
    }

}
