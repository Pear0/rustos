use pi::interrupt::{Controller, Interrupt};

use crate::{IRQ, shell, SCHEDULER};
use crate::console::kprintln;
use crate::traps::Kind::Synchronous;

pub use self::frame::TrapFrame;
use self::syndrome::Syndrome;
use self::syndrome::Fault;
use self::syscall::handle_syscall;
use alloc::vec::Vec;
use aarch64::MPIDR_EL1;

mod frame;
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

    let core_id = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize };
    kprintln!("IRQ: {}", core_id);

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

                    kprintln!("ELR: 0x{:x}", tf.elr);

                    debug_shell(tf);
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

extern "C" {
    static __code_beg: u8;
    static __code_end: u8;
}

fn address_maybe_code(num: u64) -> bool {
    unsafe { num >= (&__code_beg as *const u8 as u64) && num <= (&__code_end as *const u8 as u64) }
}

fn debug_shell(tf: &mut TrapFrame) {
    let mut snaps = Vec::new();
    SCHEDULER.critical(|s| s.get_process_snaps(&mut snaps));

    let snap = snaps.into_iter().find(|s| s.tpidr == tf.tpidr);

    match &snap {
        Some(snap) => {
            kprintln!("Debug Shell on: {:?}", snap);
        },
        None => {
            kprintln!("Debug Shell, unknown process");
        }
    }

    use shim::io::Write;
    let mut sh = shell::serial_shell("#>");

    sh.register_func("elev", |sh, cmd| {
        writeln!(sh.writer, "elevated prompt");
    });

    sh.register_func("regs", |sh, cmd| {
        if cmd.args.len() == 2 && cmd.args[1] == "full" {
            tf.dump(&mut sh.writer, true);
        } else {
            tf.dump(&mut sh.writer, false);
        }
    });

    sh.register_func("bt", |sh, cmd| {
        if let Some(snap) = &snap {
            writeln!(sh.writer, "sp_top: 0x{:08x}", snap.stack_top);
            writeln!(sh.writer, "sp: 0x{:08x}", tf.sp);
            if snap.stack_top % 8 != 0 {
                writeln!(sh.writer, "dude stack_top is not aligned. I'm out");
                return;
            }

            let mut sp = tf.sp;
            if sp % 8 != 0 {
                writeln!(sh.writer, "stack not aligned! aligning up...");
                sp = sp.wrapping_add(8).wrapping_sub(sp % 8);
                return;
            }

            // alignment already handled
            let slice = unsafe { core::slice::from_raw_parts(sp as *const u64, ((snap.stack_top - sp) / 8) as usize) };

            writeln!(sh.writer, "==== scanning {} addresses ====", slice.len());

            for num in slice.iter().rev() {
                if address_maybe_code(*num) {
                    writeln!(sh.writer, "0x{:08x}", num);
                }
            }

        } else {
            writeln!(sh.writer, "cannot dump stack for unknown process");
        }
    });

    sh.shell_loop();
}


