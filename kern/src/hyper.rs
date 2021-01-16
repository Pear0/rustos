use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;

use aarch64::CNTHP_CTL_EL2;
use pi::interrupt::{CoreInterrupt, Interrupt};
use pi::usb::Usb;

use crate::{hw, perf, shell, smp, timing, VMM};
use crate::arm::{HyperPhysicalCounter, TimerController};
use crate::BootVariant::Hypervisor;
use crate::cls::{CoreGlobal, CoreLocal, CoreMutex};
use crate::console::{console_ext_init, console_interrupt_handler};
use crate::debug::initialize_debug;
use crate::fs::handle::{Sink, SinkWrapper, Source, SourceWrapper, WaitingSourceWrapper};
use crate::iosync::Global;
use crate::net::physical::{Physical, PhysicalUsb};
use crate::process::{GlobalScheduler, HyperImpl, HyperProcess};
use crate::traps::{HyperTrapFrame, IRQ_RECURSION_DEPTH};
use crate::traps::irq::Irq;
use crate::virtualization::nic::VirtualSwitch;

pub static HYPER_IRQ: Irq<HyperImpl> = Irq::uninitialized();
pub static HYPER_SCHEDULER: GlobalScheduler<HyperImpl> = GlobalScheduler::uninitialized();

pub static NET_SWITCH: Global<VirtualSwitch> = Global::new(|| VirtualSwitch::new_hub());

pub static HYPER_TIMER: CoreGlobal<TimerController<HyperTrapFrame, HyperPhysicalCounter>> = CoreLocal::new_global(|| TimerController::new());

fn net_thread() -> ! {
    if hw::is_qemu() {
        error!("detected qemu, not booting network services.");
        kernel_api::syscall::exit();
    }

    kernel_api::syscall::sleep(Duration::from_millis(1000));

    crate::mbox::with_mbox(|mbox| {
        info!("Serial: {:x?}", mbox.serial_number());
        info!("MAC: {:x?}", mbox.mac_address());
        info!("Board Revision: {:x?}", mbox.board_revision());
        info!("Temp: {:?}", mbox.core_temperature());
    });

    crate::mbox::with_mbox(|mbox| mbox.set_power_state(0x00000003, true));
    pi::timer::spin_sleep(Duration::from_millis(100));

    let usb = unsafe { Usb::new().expect("failed to init usb") };

    debug!("created usb");

    let usb = PhysicalUsb(usb);

    while !usb.is_connected() {
        debug!("waiting for link");
        kernel_api::syscall::sleep(Duration::from_millis(500));
    }

    let usb = Arc::new(usb);

    NET_SWITCH.critical(move |switch| {
        switch.debug = true;

        // VirtualSwitch holds a weak reference.
        // For now we always want to have usb ethernet connected,
        // so just leak the Arc.
        switch.register(usb.clone());
        core::mem::forget(usb);
    });

    loop {
        if !(NET_SWITCH.critical(|s| s.process(Duration::from_micros(500)))) {
            kernel_api::syscall::sleep(Duration::from_millis(5));
        }
    }

    error!("net thread exit.");
    kernel_api::syscall::exit();
}

fn test_thread() -> ! {
    use aarch64::regs::*;

    loop {
        let core = smp::core();
        info!("I am the hyper thread! {} {}, {:#b}", unsafe { CNTHP_CTL_EL2.get() & CNTHP_CTL_EL2::ISTATUS }, unsafe { DAIF.get() }, unsafe { ((0x4000_0040 + 4 * core) as *mut u32).read_volatile() });

        // pi::timer::spin_sleep(Duration::from_secs(1));
        kernel_api::syscall::sleep(Duration::from_secs(1));
    }
}

fn my_thread() -> ! {
    let pid = kernel_api::syscall::getpid();

    shell::Shell::new("$ ",
                      WaitingSourceWrapper::new(Arc::new(Source::KernSerial)),
                      SinkWrapper::new(Arc::new(Sink::KernSerial))).shell_loop();

    kernel_api::syscall::exit();
}

pub static TIMER_EVENTS: AtomicU64 = AtomicU64::new(0);
pub static TIMER_EVENTS_EXC: AtomicU64 = AtomicU64::new(0);

static HYPER_TIMER_SKIP_COUNT: AtomicUsize = AtomicUsize::new(0);

fn configure_timer() {
    HYPER_IRQ.register_core(crate::smp::core(), CoreInterrupt::CNTHPIRQ, Box::new(|tf| {
        smp::no_interrupt(|| {
            HYPER_TIMER.critical(|timer| timer.process_timers(tf, |func| func()));
        });
    }));

    if smp::core() == 0 {
        perf::prepare();

        HYPER_TIMER.critical(|timer| {
            timer.add(10, timing::time_to_cycles::<HyperPhysicalCounter>(Duration::from_micros(10)), Box::new(|ctx| {
                if HYPER_TIMER_SKIP_COUNT.fetch_add(1, Ordering::Relaxed) < 10 {
                    return;
                }

                if !perf::record_event_hyper(ctx.data) {
                    ctx.remove_timer();
                }
                // TIMER_EVENTS.fetch_add(1, Ordering::Relaxed);
                // if IRQ_RECURSION_DEPTH.get() > 1 {
                //     TIMER_EVENTS_EXC.fetch_add(1, Ordering::Relaxed);
                // }
            }));

            timer.add(10, timing::time_to_cycles::<HyperPhysicalCounter>(Duration::from_secs(50)), Box::new(|ctx| {
                ctx.remove_timer();
                // info!("Timer events: {}, exc:{}", TIMER_EVENTS.load(Ordering::Relaxed), TIMER_EVENTS_EXC.load(Ordering::Relaxed));
                perf::dump_events();
            }));
        });
    }
}

pub fn hyper_main() -> ! {
    info!("VMM init");
    VMM.init_only();
    info!("VMM setup");
    VMM.setup_hypervisor();

    info!("ID_AA64PFR0_EL1: {:#b}", unsafe { ID_AA64PFR0_EL1.get() });
    info!("ID_AA64MMFR1_EL1: {:#b}", unsafe { ID_AA64MMFR1_EL1.get() });

    info!("Making hvc call");
    hvc!(5);

    HYPER_IRQ.initialize();

    console_ext_init();
    HYPER_IRQ.register(Interrupt::Aux, Box::new(|_| {
        console_interrupt_handler();
    }));

    pi::interrupt::Controller::new().enable(pi::interrupt::Interrupt::Aux);

    error!("init cores");

    let cores = 4;
    unsafe { smp::initialize(cores); }

    error!("waiting cores");


    smp::wait_for_cores(cores);

    error!("cores vmm");

    smp::run_on_secondary_cores(|| {
        VMM.setup_hypervisor();
    });

    configure_timer();
    unsafe {
        HYPER_SCHEDULER.initialize_hyper();
    }

    error!("cores scheduler");


    smp::run_on_secondary_cores(|| {
        configure_timer();
        unsafe {
            HYPER_SCHEDULER.initialize_hyper();
        };
    });


    // for atag in pi::atags::Atags::get() {
    //     info!("{:?}", atag);
    // }

    // Ensure timers are set up. Scheduler will also do this on start.
    // unsafe { CNTHP_CTL_EL2.set((CNTHP_CTL_EL2.get() & !CNTHP_CTL_EL2::IMASK) | CNTHP_CTL_EL2::ENABLE) };

    initialize_debug();

    debug!("ARM Timer Freq: {}", unsafe { CNTFRQ_EL0.get() });

    // timing::benchmark("pi::timer", |num| {
    //     for _ in 0..num {
    //         pi::timer::current_time();
    //     }
    // });
    // timing::benchmark("CNTPCT_EL0", |num| {
    //     for _ in 0..num {
    //         unsafe { CNTPCT_EL0.get() };
    //     }
    // });
    //
    // timing::benchmark("mutex lock", |num| {
    //     let mut mu = mutex_new!(5);
    //
    //     for _ in 0..num {
    //         let mut lock = m_lock!(mu);
    //         *lock += 1;
    //     }
    // });

    error!("Add kernel process");

    {
        let proc = HyperProcess::hyper_process_old("shell".to_owned(), my_thread).unwrap();
        HYPER_SCHEDULER.add(proc);
    }

    {
        let mut p = HyperProcess::load_self().expect("failed to find bin");
        p.affinity.set_only(0);
        let id = HYPER_SCHEDULER.add(p);
        info!("kernel id: {:?}", id);
    }

    // {
    //     // let p = HyperProcess::load("/kernel.bin").expect("failed to find bin");
    //     let p = HyperProcess::load_self().expect("failed to find bin");
    //
    //     let id = HYPER_SCHEDULER.add(p);
    //     info!("kernel id: {:?}", id);
    // }
    //
    // {
    //     let p = HyperProcess::hyper_process_old(String::from("hyper proc"), test_thread).expect("failed to create hyper thread");
    //     let id = HYPER_SCHEDULER.add(p);
    //     info!("kernel id: {:?}", id);
    // }

    {
        let p = HyperProcess::hyper_process_old(String::from("hyper net"), net_thread).expect("failed to create hyper thread");
        let id = HYPER_SCHEDULER.add(p);
        info!("kernel id: {:?}", id);
    }

    info!("Starting kernel");

    use aarch64::{VTTBR_EL2, TTBR0_EL1, TTBR1_EL1};
    use aarch64::regs::*;

    unsafe {
        ((0x4000_000C) as *mut u32).write_volatile(0b1111);
        aarch64::dsb();

        // let vm = VirtualizationPageTable::new();
        //
        // VTTBR_EL2.set(vm.get_baddr().as_u64());

        llvm_asm!("dsb ish");
        aarch64::isb();

        // don't trap CNTP for EL1/EL0 (ref: D7.5.2, D7.5.13)
        CNTHCTL_EL2.set(CNTHCTL_EL2.get() | CNTHCTL_EL2::EL1PCEN | CNTHCTL_EL2::EL1PCTEN);
    }

    smp::run_no_return(|| {
        let core = smp::core();
        pi::timer::spin_sleep(Duration::from_millis(4 * core as u64));
        debug!("Core {} starting scheduler", core);

        unsafe { CNTHCTL_EL2.set(CNTHCTL_EL2.get() | CNTHCTL_EL2::EL1PCEN | CNTHCTL_EL2::EL1PCTEN) };
        HYPER_SCHEDULER.start_hyper();

        error!("RIP RIP");
    });

    pi::timer::spin_sleep(Duration::from_millis(50));
    debug!("Core 0 starting scheduler");

    HYPER_SCHEDULER.start_hyper();

    info!("looping in HVC");
    loop {}
}

