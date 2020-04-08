use alloc::vec::Vec;

use aarch64::MPIDR_EL1;
use pi::interrupt::{Controller, CoreInterrupt, Interrupt};

use crate::{debug, IRQ, SCHEDULER, shell, smp};
use crate::cls::CoreLocal;
use crate::traps::Kind::Synchronous;

pub use self::frame::TrapFrame;
use self::syndrome::Fault;
use self::syndrome::Syndrome;
use self::syscall::handle_syscall;

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

fn handle_irqs(tf: &mut TrapFrame) {
    let ctl = Controller::new();
    // Invoke any handlers

    for _ in 0..20 {
        let mut any_pending = false;
        for int in Interrupt::iter() {
            if ctl.is_pending(*int) {
                any_pending = true;
                IRQ.invoke(*int, tf);
            }
        }

        let core = smp::core();
        for _ in 0..CoreInterrupt::MAX {
            if let Some(int) = CoreInterrupt::read(core) {
                any_pending = true;
                IRQ.invoke_core(core, int, tf);
            } else {
                break;
            }
        }

        if !any_pending {
            return;
        }
    }

    kprintln!("irq stuck pending!");

    debug_shell(tf);
}

static IRQ_RECURSION_DEPTH: CoreLocal<i32> = CoreLocal::new_copy(0);
static IRQ_ESR: CoreLocal<u32> = CoreLocal::new_copy(0xFF_FF_FF_FF);
static IRQ_EL: CoreLocal<u64> = CoreLocal::new_copy(0xFF_FF_FF_FF);
static IRQ_INFO: CoreLocal<Info> = CoreLocal::new_copy(Info { kind: Kind::None, source: Source::CurrentSpEl0 });

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


/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
#[no_mangle]
pub extern "C" fn handle_exception(info: Info, esr: u32, tf: &mut TrapFrame) {
    //
    // let core_id = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize };
    // kprintln!("IRQ: {}", core_id);

    if IRQ_RECURSION_DEPTH.get() != 0 {
        kprintln!("Recursive IRQ: {:?}", info);
        shell::shell("#>");
    }

    IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() + 1);
    IRQ_ESR.set(esr);
    IRQ_EL.set(tf.elr);
    IRQ_INFO.set(info);

    match info.kind {
        Kind::Irq => {
            handle_irqs(tf);
        }
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

    IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() - 1);
}

fn debug_shell(tf: &mut TrapFrame) {
    let mut snaps = Vec::new();
    SCHEDULER.critical(|s| s.get_process_snaps(&mut snaps));

    let snap = snaps.into_iter().find(|s| s.tpidr == tf.tpidr);

    match &snap {
        Some(snap) => {
            kprintln!("Debug Shell on: {:?}", snap);
        }
        None => {
            kprintln!("Debug Shell, unknown process");
        }
    }

    use shim::io::Write;
    let mut sh = shell::serial_shell("#>");

    sh.command()
        .name("elev")
        .func(|sh, cmd| {
            writeln!(sh.writer, "elevated prompt");
        })
        .build();

    sh.command()
        .name("regs")
        .func(|sh, cmd| {
            if cmd.args.len() == 2 && cmd.args[1] == "full" {
                tf.dump(&mut sh.writer, true);
            } else {
                tf.dump(&mut sh.writer, false);
            }
        })
        .build();

    sh.command()
        .name("bt")
        .func(|sh, cmd| {
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
                    if debug::address_maybe_code(*num) {
                        writeln!(sh.writer, "0x{:08x}", num);
                    }
                }
            } else {
                writeln!(sh.writer, "cannot dump stack for unknown process");
            }
        })
        .build();


    sh.shell_loop();
}


