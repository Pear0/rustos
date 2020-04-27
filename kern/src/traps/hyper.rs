use alloc::vec::Vec;

use pi::interrupt::{Controller, CoreInterrupt, Interrupt};

use crate::{debug, shell, smp};
use crate::process::State;
use crate::traps::{Info, IRQ_EL, IRQ_ESR, IRQ_INFO, IRQ_RECURSION_DEPTH, KernelTrapFrame, Kind, HyperTrapFrame, Source};
use crate::traps::Kind::Synchronous;
use crate::traps::syndrome::{Syndrome, Fault, AbortInfo};
use crate::traps::syscall::handle_syscall;
use crate::hyper::{HYPER_IRQ, HYPER_SCHEDULER};
use crate::vm::VirtualAddr;

#[derive(Debug)]
enum IrqVariant {
    Irq(Interrupt),
    CoreIrq(CoreInterrupt),
}

fn handle_irqs(tf: &mut HyperTrapFrame) {
    let ctl = Controller::new();
    // Invoke any handlers

    let mut pending: Option<IrqVariant> = None;

    for _ in 0..20 {
        let mut any_pending = false;
        for int in Interrupt::iter() {
            if ctl.is_pending(*int) {
                any_pending = true;
                pending = Some(IrqVariant::Irq(*int));
                HYPER_IRQ.invoke(*int, tf);
            }
        }

        let core = smp::core();
        for _ in 0..CoreInterrupt::MAX {
            if let Some(int) = CoreInterrupt::read(core) {
                any_pending = true;
                pending = Some(IrqVariant::CoreIrq(int));
                HYPER_IRQ.invoke_core(core, int, tf);
            } else {
                break;
            }
        }

        if !any_pending {
            return;
        }

        // todo HACK
        if let Some(IrqVariant::Irq(Interrupt::Timer1)) = pending {
            return;
        }
    }

    kprintln!("irq stuck pending! -> {:?}", pending);

    debug_shell(tf);
}

/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
#[no_mangle]
pub extern "C" fn hyper_handle_exception(info: Info, esr: u32, tf: &mut HyperTrapFrame) {

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
            use aarch64::regs::*;
            tf.simd[0] += 1;

            if tf.simd[0] % 100 == 0 {
                kprintln!("IRQ: {:?} {:?} (raw=0x{:x}) @ {:#x}", info, Kind::Irq, esr, tf.elr);
                kprintln!("FAR_EL1 = 0x{:x}, FAR_EL2 = 0x{:x}, HPFAR_EL2 = 0x{:x}", unsafe { FAR_EL1.get() }, unsafe { FAR_EL2.get() }, unsafe { HPFAR_EL2.get() });
                kprintln!("EL1: {:?} (raw=0x{:x})", Syndrome::from(unsafe { ESR_EL1.get() } as u32), unsafe { ESR_EL1.get() });
                kprintln!("SP: {:#x}, ELR_EL1: {:#x}", unsafe { SP_EL1.get() }, unsafe { ELR_EL1.get() });
            }

            handle_irqs(tf);
        }
        Kind::Synchronous => {
            match Syndrome::from(esr) {
                Syndrome::Brk(b) => {
                    kprintln!("{:?} {:?}", info, Syndrome::Brk(b));
                    kprintln!("brk #{}", b);

                    kprintln!("ELR: 0x{:x}", tf.elr);

                    debug_shell(tf);
                }
                Syndrome::Hvc(b) => {
                    if b == 8 {
                        use aarch64::regs::*;
                        // kprintln!("returning from hvc({}), {:?}, {:#x?}", b, Syndrome::from(unsafe { ESR_EL1.get() } as u32), unsafe { ELR_EL1.get() });
                    } else if b == 5 {
                        kprintln!("returning from hvc({})", b);
                    } else {
                        kprintln!("{:?} {:?} (raw=0x{:x}) @ {:x}", info, Syndrome::Hvc(b), esr, tf.elr);

                        loop {}
                    }
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
                        HYPER_SCHEDULER.crit_process(tf.tpidr, |p| {
                            if let Some(p) = p {
                                // won't work once guest enables virtualization.

                                p.on_access_fault(esr, VirtualAddr::from(aarch64::far_ipa()), tf);

                                // p.vmap.table.mark_accessed(VirtualAddr::from(unsafe { FAR_EL2.get() } & PAGE_MASK as u64));
                            }
                        });
                    } else if let Syndrome::DataAbort(_) = s {
                        kprintln!("FAR_EL1 = 0x{:x}, FAR_EL2 = 0x{:x}, HPFAR_EL2 = 0x{:x}", unsafe { FAR_EL1.get() }, unsafe { FAR_EL2.get() }, unsafe { HPFAR_EL2.get() });
                    } else if let Syndrome::InstructionAbort(_) = s {
                        use aarch64::regs::*;
                        kprintln!("IRQ: {:?} {:?} (raw=0x{:x}) @ {:#x}", info, Kind::Irq, esr, tf.elr);
                        kprintln!("FAR_EL1 = 0x{:x}, FAR_EL2 = 0x{:x}, HPFAR_EL2 = 0x{:x}", unsafe { FAR_EL1.get() }, unsafe { FAR_EL2.get() }, unsafe { HPFAR_EL2.get() });
                        kprintln!("EL1: {:?} (raw=0x{:x})", Syndrome::from(unsafe { ESR_EL1.get() } as u32), unsafe { ESR_EL1.get() });
                        kprintln!("SP: {:#x}, ELR_EL1: {:#x}", unsafe { SP_EL1.get() }, unsafe { ELR_EL1.get() });

                        info!("frame: {:#x?}", tf);
                    }

                    if !is_access_flag {
                        kprintln!("{:?} {:?} (raw=0x{:x}) @ {:#x}", info, s, esr, tf.elr);


                        // SCHEDULER.crit_process(tf.tpidr, |p| {
                        //     if let Some(p) = p {
                        //         // won't work once guest enables virtualization.
                        //
                        //         let mut foo = m_lock!(CONSOLE);
                        //         p.dump(foo.deref_mut());
                        //     }
                        // });


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
        if tf.hcr & HCR_EL2::VM != 0 {
            HYPER_SCHEDULER.crit_process(tf.tpidr, |p| {
                if let Some(p) = p {
                    // give process a chance to update any virtualized components.
                    *p.context = *tf;
                    p.update();
                    *tf = *p.context;
                }
            });
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


