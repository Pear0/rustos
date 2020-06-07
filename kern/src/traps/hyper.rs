use alloc::vec::Vec;

use pi::interrupt::{Controller, CoreInterrupt, Interrupt};

use crate::{debug, shell, smp, timing};
use crate::process::State;
use crate::traps::{Info, IRQ_EL, IRQ_ESR, IRQ_INFO, IRQ_RECURSION_DEPTH, KernelTrapFrame, Kind, HyperTrapFrame, Source};
use crate::traps::Kind::Synchronous;
use crate::traps::syndrome::{Syndrome, Fault, AbortInfo};
use crate::traps::syscall::handle_syscall;
use crate::hyper::{HYPER_IRQ, HYPER_SCHEDULER, HYPER_TIMER};
use crate::vm::VirtualAddr;
use crate::traps::hypercall::{handle_hyper_syscall, handle_hypercall};
use crate::traps::coreinfo::{exc_enter, exc_exit, ExceptionType, exc_record_time};
use crate::param::PAGE_MASK;
use crate::arm::{HyperPhysicalCounter, GenericCounterImpl};

#[derive(Debug)]
enum IrqVariant {
    Irq(Interrupt),
    CoreIrq(CoreInterrupt),
}

fn handle_irqs(tf: &mut HyperTrapFrame) {
    let ctl = Controller::new();
    // Invoke any handlers

    let mut pending: Option<IrqVariant> = None;

    for k in 0..20 {
        let last = k == 19;
        let mut any_pending = false;
        for int in Interrupt::iter() {
            if ctl.is_pending(*int) {
                any_pending = true;
                if last {
                    kprintln!("{} irq stuck pending! -> {:?}", k, IrqVariant::Irq(*int));
                }

                HYPER_IRQ.invoke(*int, tf);
            }
        }

        let core = smp::core();
        for _ in 0..CoreInterrupt::MAX {
            if let Some(int) = CoreInterrupt::read(core) {
                any_pending = true;
                if last {
                    kprintln!("{}@core={} irq stuck pending! -> {:?}", k, core, IrqVariant::CoreIrq(int));
                }

                HYPER_IRQ.invoke_core(core, int, tf);
            } else {
                break;
            }
        }

        if !any_pending {
            return;
        }

    }

    debug_shell(tf);
}

fn do_hyper_handle_exception(info: Info, esr: u32, tf: &mut HyperTrapFrame) -> ExceptionType {
    let mut exc_type = ExceptionType::Unknown;

    match info.kind {
        Kind::Irq | Kind::Fiq => {

            exc_type = ExceptionType::Irq;
            handle_irqs(tf);
        }
        Kind::Synchronous => {
            match Syndrome::from(esr) {
                Syndrome::Brk(b) => {
                    kprintln!("{:?} {:?}", info, Syndrome::Brk(b));
                    kprintln!("brk #{}", b);

                    kprintln!("ELR: 0x{:x}", tf.ELR_EL2);

                    debug_shell(tf);
                }
                Syndrome::Svc(svc) => {
                    handle_hyper_syscall(svc, tf);
                }
                Syndrome::Hvc(b) => {
                    handle_hypercall(b, tf);
                }
                s => {
                    use aarch64::regs::*;

                    let mut is_access_flag: bool = false;
                    if let Syndrome::DataAbort(AbortInfo { kind: Fault::AccessFlag, .. }) = s {
                        is_access_flag = true;
                    }
                    if let Syndrome::InstructionAbort(AbortInfo { kind: Fault::AccessFlag, .. }) = s {
                        is_access_flag = true;
                    }

                    if is_access_flag {
                        HYPER_SCHEDULER.crit_process(tf.TPIDR_EL2, |p| {
                            if let Some(p) = p {
                                let addr = aarch64::far_ipa();
                                exc_type = ExceptionType::DataAccess(addr & !(0x1000 - 1));
                                p.on_access_fault(esr, VirtualAddr::from(addr), tf);
                            }
                        });
                    } else if let Syndrome::DataAbort(_) = s {
                        kprintln!("FAR_EL1 = 0x{:x}, FAR_EL2 = 0x{:x}, HPFAR_EL2 = 0x{:x}", unsafe { FAR_EL1.get() }, unsafe { FAR_EL2.get() }, unsafe { HPFAR_EL2.get() });
                    } else if let Syndrome::InstructionAbort(_) = s {
                        use aarch64::regs::*;
                        kprintln!("IRQ: {:?} {:?} (raw=0x{:x}) @ {:#x}", info, Kind::Irq, esr, tf.ELR_EL2);
                        kprintln!("FAR_EL1 = 0x{:x}, FAR_EL2 = 0x{:x}, HPFAR_EL2 = 0x{:x}", unsafe { FAR_EL1.get() }, unsafe { FAR_EL2.get() }, unsafe { HPFAR_EL2.get() });
                        kprintln!("EL1: {:?} (raw=0x{:x})", Syndrome::from(unsafe { ESR_EL1.get() } as u32), unsafe { ESR_EL1.get() });
                        kprintln!("SP: {:#x}, ELR_EL1: {:#x}", unsafe { SP_EL1.get() }, unsafe { ELR_EL1.get() });

                        info!("frame: {:#x?}", tf);
                    }

                    if !is_access_flag {
                        kprintln!("{:?} {:?} (raw=0x{:x}) @ {:#x}", info, s, esr, tf.ELR_EL2);
                        loop {}
                    }
                }
            }
        }
        _ => {
            kprintln!("{:?}", info);
            shell::shell("#>");
        }
    }

    if info.kind == Kind::Irq || (info.kind == Kind::Synchronous && info.source == Source::LowerAArch64) {
        use aarch64::regs::*;
        if tf.HCR_EL2 & HCR_EL2::VM != 0 {
            HYPER_SCHEDULER.crit_process(tf.TPIDR_EL2, |p| {
                if let Some(p) = p {
                    // give process a chance to update any virtualized components.
                    *p.context = *tf;
                    p.update(info.kind == Kind::Irq);
                    *tf = *p.context;
                }
            });
        }
    }

    // continue execution
    if info.kind == Synchronous {
        let syndrome = Syndrome::from(esr);
        if let Syndrome::Brk(_) = syndrome {
            tf.ELR_EL2 += 4;
        }
    }

    exc_type
}

/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
#[no_mangle]
pub extern "C" fn hyper_handle_exception(info: Info, esr: u32, tf: &mut HyperTrapFrame) {

    let exc_start = unsafe { aarch64::CNTPCT_EL0.get() };
    let time_start = timing::clock_time();
    let mut exc_type = ExceptionType::Unknown;
    exc_enter();

    if IRQ_RECURSION_DEPTH.get() > 1 {
        kprintln!("Recursive IRQ: {:?}", info);
        shell::shell("#>");
    }

    // recursive irq for profiling
    let is_recursive = IRQ_RECURSION_DEPTH.get() == 1;
    let is_timer = HyperPhysicalCounter::interrupted();

    if is_recursive {

        if is_timer {
            IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() + 1);

            // interrupts are disabled here:
            HYPER_TIMER.critical(|timer| timer.process_timers(tf));

            IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() - 1);
        } else {
            use aarch64::regs::*;
            // set guest interrupts masked, we don't know what is interrupting us but
            // it's not the timer.
            tf.SPSR_EL2 |= SPSR_EL2::D | SPSR_EL2::A | SPSR_EL2::I | SPSR_EL2::F;
        }

    } else {
        use aarch64::regs::*;

        IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() + 1);
        IRQ_ESR.set(esr);
        IRQ_EL.set(tf.ELR_EL2);
        IRQ_INFO.set(info);

        // try to handle timers sooner
        if is_timer {
            HYPER_TIMER.critical(|timer| timer.process_timers(tf));
        }

        {
            // enable general IRQ, so we can get interrupted by the timer.
            unsafe { DAIF.set(DAIF::D | DAIF::A | DAIF::F) };

            exc_type = do_hyper_handle_exception(info, esr, tf);

            // disable interrupts again, IRQ_RECURSION_DEPTH will be wrong and a recursive
            // interrupt won't realize it is recursive.
            unsafe { DAIF.set(DAIF::D | DAIF::A | DAIF::I | DAIF::F) };
        }

        IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() - 1);
    }

    let time_end = timing::clock_time();
    exc_record_time(exc_type, time_end - time_start);
    exc_exit();

    // update offset register to somewhat hide irq time from guest.
    let exc_end = unsafe { aarch64::CNTPCT_EL0.get() };
    tf.CNTVOFF_EL2 += exc_end - exc_start;

}


fn debug_shell(tf: &mut HyperTrapFrame) {
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

    sh.shell_loop();
}


