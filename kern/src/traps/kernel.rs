use alloc::vec::Vec;
use core::time::Duration;

use pi::interrupt::{Controller, CoreInterrupt, Interrupt};

use crate::{debug, shell, smp, hw, timing};
use crate::kernel::{KERNEL_IRQ, KERNEL_SCHEDULER, KERNEL_TIMER};
use crate::param::{PAGE_MASK, PAGE_SIZE, USER_IMG_BASE};
use crate::process::State;
use crate::traps::{Info, IRQ_EL, IRQ_ESR, IRQ_INFO, IRQ_RECURSION_DEPTH, KernelTrapFrame, Kind, IRQ_FP, Source};
use crate::traps::Kind::Synchronous;
use crate::traps::syndrome::Syndrome;
use crate::traps::syscall::handle_syscall;
use crate::vm::VirtualAddr;
use crate::arm::{VirtualCounter, PhysicalCounter, GenericCounterImpl};
use crate::traps::coreinfo::{exc_enter, exc_record_time, exc_exit, ExceptionType};

#[derive(Debug)]
enum IrqVariant {
    Irq(Interrupt),
    CoreIrq(CoreInterrupt),
}


fn handle_irqs(tf: &mut KernelTrapFrame) {

    if hw::not_pi() {
        KERNEL_TIMER.process_timers(tf, |func| func());
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

fn do_kernel_handle_exception(info: Info, esr: u32, tf: &mut KernelTrapFrame) {
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
                Syndrome::DataAbort(_) | Syndrome::InstructionAbort(_) => {
                    let s = Syndrome::from(esr);
                    error!("MemAbort {:?} {:?} (FAR_EL1={:#x}) @ {:#x}", info, s, unsafe { aarch64::FAR_EL1.get() }, tf.ELR_EL1);
                }
                s => {
                    error!("F {:?} {:?} (raw={:#x}) @ {:#x}", info, s, esr, tf.ELR_EL1);

                    KERNEL_SCHEDULER.switch(State::Suspended, tf);
                }
            }
        }
        Kind::SError => {
            kprintln!("{:?} @ {:#x}", info, tf.ELR_EL1);
        }
        _ => {
            kprintln!("{:?} @ {:#x}", info, tf.ELR_EL1);
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
}


/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
#[no_mangle]
pub extern "C" fn kernel_handle_exception(info: Info, esr: u32, tf: &mut KernelTrapFrame) {

    let exc_start = unsafe { aarch64::CNTPCT_EL0.get() };
    let time_start = timing::clock_time::<VirtualCounter>();
    exc_enter();

    if IRQ_RECURSION_DEPTH.get() > 1 {
        kprintln!("Recursive IRQ: {:?}", info);
        if matches!(info.kind, Synchronous) {
            kprintln!("synchronous ESR: {:?}", Syndrome::from(esr));
            if matches!(Syndrome::from(esr), Syndrome::DataAbort(_)) {
                kprintln!("FAR_EL1 = {:#?}", unsafe { aarch64::FAR_EL1.get() });
            }
        }
        kprintln!("ELR: {:#x}", tf.ELR_EL1);
        shell::shell("#>");
    }

    const TIMER_YIELD: u16 = crate::kernel_call::NR_YIELD_FOR_TIMERS as u16;

    // recursive irq for profiling
    let is_recursive = IRQ_RECURSION_DEPTH.get() == 1;
    let (is_timer, is_timer_yield) = match info.kind {
        Kind::Irq => (VirtualCounter::interrupted(), false),
        Kind::Synchronous => (matches!(Syndrome::from(esr), Syndrome::Svc(TIMER_YIELD)), true),
        _ => (false, false),
    };

    if is_recursive {
        let mut disable_interrupts = true;

        if is_timer {
            IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() + 1);

            // interrupts are disabled here:
            disable_interrupts = KERNEL_TIMER.process_timers(tf, |func| func());

            IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() - 1);
        }
        if disable_interrupts {
            use aarch64::regs::*;
            // set guest interrupts masked, we don't know what is interrupting us but
            // it's not the timer.
            tf.SPSR_EL1 |= SPSR_EL2::D | SPSR_EL2::A | SPSR_EL2::I | SPSR_EL2::F;
        }
    } else {
        use aarch64::regs::*;

        IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() + 1);
        IRQ_ESR.set(esr);
        IRQ_EL.set(tf.ELR_EL1);
        IRQ_FP.set(tf.regs[29]);
        IRQ_INFO.set(info);

        // try to handle timers sooner
        if is_timer {
            KERNEL_TIMER.process_timers(tf, |func| {

                // enable general IRQ, so we can get interrupted by the timer.
                unsafe { DAIF.set(DAIF::D | DAIF::A | DAIF::F) };

                func();

                // disable interrupts again, IRQ_RECURSION_DEPTH will be wrong and a recursive
                // interrupt won't realize it is recursive.
                unsafe { DAIF.set(DAIF::D | DAIF::A | DAIF::I | DAIF::F) };

            });
        }

        {
            // enable general IRQ, so we can get interrupted by the timer.
            unsafe { DAIF.set(DAIF::D | DAIF::A | DAIF::F) };

            do_kernel_handle_exception(info, esr, tf);

            // disable interrupts again, IRQ_RECURSION_DEPTH will be wrong and a recursive
            // interrupt won't realize it is recursive.
            unsafe { DAIF.set(DAIF::D | DAIF::A | DAIF::I | DAIF::F) };
        }

        IRQ_EL.set(0xFF_FF_FF_FF);

        IRQ_RECURSION_DEPTH.set(IRQ_RECURSION_DEPTH.get() - 1);
    }

    let time_end = timing::clock_time::<VirtualCounter>();
    exc_record_time(ExceptionType::Unknown, time_end - time_start);
    exc_exit();
}

#[inline(never)]
fn debug_shell(tf: &mut KernelTrapFrame) {
    let mut snaps = Vec::new();
    KERNEL_SCHEDULER.get_process_snaps(&mut snaps);

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
