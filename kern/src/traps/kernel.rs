use alloc::vec::Vec;
use core::time::Duration;

use pi::interrupt::{Controller, CoreInterrupt, Interrupt};

use crate::{debug, shell, smp, hw, timing};
use crate::kernel::{KERNEL_IRQ, KERNEL_SCHEDULER, KERNEL_TIMER};
use crate::param::{PAGE_MASK, PAGE_SIZE, USER_IMG_BASE};
use crate::process::State;
use crate::traps::{Info, IRQ_EL, IRQ_ESR, IRQ_INFO, IRQ_RECURSION_DEPTH, KernelTrapFrame, Kind};
use crate::traps::Kind::Synchronous;
use crate::traps::syndrome::Syndrome;
use crate::traps::syscall::handle_syscall;
use crate::vm::VirtualAddr;
use crate::arm::VirtualCounter;

#[derive(Debug)]
enum IrqVariant {
    Irq(Interrupt),
    CoreIrq(CoreInterrupt),
}


fn handle_irqs(tf: &mut KernelTrapFrame) {

    if hw::not_pi() {
        KERNEL_TIMER.critical(|d| d.process_timers(tf));
        return;
    }

    let ctl = Controller::new();
    // Invoke any handlers

    const max_irq: usize = 50;

    let mut pending: Option<IrqVariant> = None;
    let mut diffs = [Duration::from_secs(0); max_irq];
    let mut start = timing::clock_time::<VirtualCounter>();

    for i in 0..max_irq {
        let mut any_pending = false;
        pending = None;
        for int in Interrupt::iter() {
            if ctl.is_pending(*int) {
                any_pending = true;
                pending = Some(IrqVariant::Irq(*int));
                KERNEL_IRQ.invoke(*int, tf);
            }
        }

        let core = smp::core();
        for _ in 0..CoreInterrupt::MAX {
            if let Some(int) = CoreInterrupt::read(core) {
                any_pending = true;
                pending = Some(IrqVariant::CoreIrq(int));
                KERNEL_IRQ.invoke_core(core, int, tf);
            } else {
                break;
            }
        }

        if !any_pending {
            return;
        }

        for _ in 0..5 {
            timing::clock_time::<VirtualCounter>();
        }

        let now = timing::clock_time::<VirtualCounter>();
        diffs[i] = now - start;
        start = now;
    }

    kprintln!("irq stuck pending! -> {:?} @ {:?} per loop", pending, diffs.as_ref());

    // {
    //     let mut diffs = [Duration::from_secs(0); 20];
    //     let mut start = pi::timer::current_time();
    //
    //     for i in 0..20 {
    //         for _ in 0..i {
    //             pi::timer::current_time();
    //         }
    //
    //         let now = pi::timer::current_time();
    //         diffs[i] = now - start;
    //         start = now;
    //     }
    //
    //     kprintln!("foo: {:?}", diffs);
    // }

    debug_shell(tf);
}


/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
#[no_mangle]
pub extern "C" fn kernel_handle_exception(info: Info, esr: u32, tf: &mut KernelTrapFrame) {
    //
    // let core_id = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize };
    // kprintln!("IRQ: {}", core_id);

    if IRQ_RECURSION_DEPTH.get() != 0 {
        kprintln!("Recursive IRQ: {:?}", info);
        shell::shell("#>");
    }

    IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() + 1);
    IRQ_ESR.set(esr);
    IRQ_EL.set(tf.ELR_EL1);
    IRQ_INFO.set(info);

    match info.kind {
        Kind::Irq | Kind::Fiq => {
            use aarch64::regs::*;
            // kprintln!("Got an irq");
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

                    kprintln!("ELR: 0x{:x}", tf.ELR_EL1);

                    debug_shell(tf);
                }
                s @ Syndrome::Other(13) => {
                    use core::fmt::Write;
                    writeln!(hw::arch().early_writer(), "bad atomic instruction :( {:?} {:?} (raw={:#x}) @ {:#x}", info, s, esr, tf.ELR_EL1);
                }
                s => {
                    error!("F {:?} {:?} (raw={:#x}) @ {:#x}", info, s, esr, tf.ELR_EL1);

                    // KERNEL_SCHEDULER.crit_process(tf.TPIDR_EL0, |proc| {
                    //     if let Some(proc) = proc {
                    //
                    //         let b = tf.ELR_EL1 > USER_IMG_BASE as u64;
                    //         error!("A @ {:#x} {:#x} {:?}", tf.ELR_EL1, USER_IMG_BASE, b);
                    //         if b {
                    //             error!("B");
                    //             if let Some(page) = proc.vmap.get_page_mut(VirtualAddr::from(tf.ELR_EL1 & PAGE_MASK as u64)) {
                    //                 let offset = (((tf.ELR_EL1 as usize % PAGE_SIZE) / 4) * 4);
                    //
                    //                 error!("C");
                    //                 error!("value at ELR_EL1: {:#x} {:#x} {:#x} {:#x}", page[offset], page[offset+1], page[offset+2], page[offset+3]);
                    //             }
                    //         } else {
                    //             let page = unsafe { core::slice::from_raw_parts(0 as *const u8, 0x10000000) };
                    //             let offset = tf.ELR_EL1 as usize;
                    //             error!("value at ELR_EL1: {:#x} {:#x} {:#x} {:#x}", page[offset], page[offset+1], page[offset+2], page[offset+3]);
                    //         }
                    //
                    //
                    //         proc.request_suspend = true;
                    //     }
                    // });

                    KERNEL_SCHEDULER.switch(State::Suspended, tf);
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
            tf.ELR_EL1 += 4;
        }
    }

    IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() - 1);
}

fn debug_shell(tf: &mut KernelTrapFrame) {
    let mut snaps = Vec::new();
    KERNEL_SCHEDULER.critical(|s| s.get_process_snaps(&mut snaps));

    let snap = snaps.into_iter().find(|s| s.tpidr == tf.TPIDR_EL0);

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
                writeln!(sh.writer, "sp: 0x{:08x}", tf.SP_EL0);
                if snap.stack_top % 8 != 0 {
                    writeln!(sh.writer, "dude stack_top is not aligned. I'm out");
                    return;
                }

                let mut sp = tf.SP_EL0;
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
