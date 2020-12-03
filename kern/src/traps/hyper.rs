use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use pi::interrupt::{Controller, CoreInterrupt, Interrupt};

use crate::{debug, shell, smp, timing};
use crate::arm::{GenericCounterImpl, HyperPhysicalCounter, PhysicalCounter};
use crate::hyper::{HYPER_IRQ, HYPER_SCHEDULER, HYPER_TIMER};
use crate::param::PAGE_MASK;
use crate::process::State;
use crate::traps::{HyperTrapFrame, Info, IRQ_EL, IRQ_ESR, IRQ_INFO, IRQ_RECURSION_DEPTH, KernelTrapFrame, Kind, Source};
use crate::traps::coreinfo::{exc_enter, exc_exit, exc_record_time, ExceptionType};
use crate::traps::hypercall::{handle_hyper_syscall, handle_hypercall};
use crate::traps::Kind::Synchronous;
use crate::traps::syndrome::{AbortInfo, Fault, Syndrome};
use crate::traps::syscall::handle_syscall;
use crate::vm::VirtualAddr;

pub static TM_TOTAL_TIME: AtomicU64 = AtomicU64::new(0);
pub static TM_TOTAL_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
enum IrqVariant {
    Irq(Interrupt),
    CoreIrq(CoreInterrupt),
}

fn handle_irqs(tf: &mut HyperTrapFrame) {
    let ctl = Controller::new();
    // Invoke any handlers

    let mut pending: Option<IrqVariant> = None;

    let max_check_irq = 50;

    for k in 0..max_check_irq {
        let last = k == (max_check_irq - 1);
        let mut any_pending = false;

        // only read registers once per loop.
        let snap = ctl.snap();

        for int in Interrupt::iter() {
            if snap.is_pending(*int) {
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
            let start = timing::clock_time::<PhysicalCounter>();

            // total: 27us
            // lock: 3us
            // lock+update(): 14us
            HYPER_SCHEDULER.crit_process(tf.TPIDR_EL2, |p| {
                if let Some(p) = p {
                    // give process a chance to update any virtualized components.
                    *p.context = *tf;
                    p.update(info.kind == Kind::Irq);
                    *tf = *p.context;
                }
            });

            let diff = timing::clock_time::<PhysicalCounter>() - start;
            TM_TOTAL_TIME.fetch_add(diff.as_micros() as u64, Ordering::Relaxed);
            TM_TOTAL_COUNT.fetch_add(1, Ordering::Relaxed);

            // something useless that cannot be optimized out
            unsafe { aarch64::regs::ELR_EL2.get() };
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

pub static VERBOSE_CORE: AtomicBool = AtomicBool::new(false);

/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
#[no_mangle]
pub extern "C" fn hyper_handle_exception(info: Info, esr: u32, tf: &mut HyperTrapFrame) {
    // if VERBOSE_CORE.load(Ordering::Relaxed) {
    //     error!("exc!");
    // }

    let exc_start = unsafe { aarch64::CNTPCT_EL0.get() };
    let time_start = timing::clock_time::<HyperPhysicalCounter>();
    let mut exc_type = ExceptionType::Unknown;
    exc_enter();

    if IRQ_RECURSION_DEPTH.get() > 1 {
        kprintln!("Recursive IRQ: {:?}", info);
        shell::shell("#>");
    }

    // recursive irq for profiling
    let is_recursive = IRQ_RECURSION_DEPTH.get() == 1;
    let mut is_timer = info.kind == Kind::Irq && HyperPhysicalCounter::interrupted();

    if is_recursive {
        let mut disable_interrupts = true;

        if is_timer {
            IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() + 1);

            // interrupts are disabled here:
            disable_interrupts = HYPER_TIMER.critical(|timer| timer.process_timers(tf, |func| func()));

            IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() - 1);
        }
        if disable_interrupts {
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
            HYPER_TIMER.critical(|timer| timer.process_timers(tf, |func| func()));
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

    let time_end = timing::clock_time::<HyperPhysicalCounter>();
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


