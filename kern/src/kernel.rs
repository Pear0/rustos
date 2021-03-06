use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::panic::Location;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use core::time::Duration;

use enumset::EnumSet;

use dsx::sync::sema::SingleSetSemaphore;
use karch::capability::ExecCapability;
use pi::gpio;
use pi::interrupt::{CoreInterrupt, Interrupt};
use shim::{io, ioerr};
use usb_host::consts::USBSpeed;
use usb_host::items::{ControlCommand, TypeTriple};
use usb_host::structs::USBDevice;
use usb_host::traits::USBHostController;
use usb_host::USBHost;
use xhci::FlushType;

use crate::{BootVariant, display_manager, EXEC_CONTEXT, FILESYSTEM2, hw, kernel_call, NET, perf, shell, smp, tasks, timing, VMM};
use crate::arm::{PhysicalCounter, TimerController, VirtualCounter};
use crate::cls::{CoreGlobal, CoreLazy, CoreLocal};
use crate::console::{CONSOLE, console_ext_init, console_interrupt_handler};
use crate::debug::initialize_debug;
use crate::fs::handle::{Sink, SinkWrapper, Source, SourceWrapper, WaitingSourceWrapper};
use crate::fs::service::PipeService;
use crate::hw::ArchVariant;
use crate::mini_allocators::NOCACHE_PAGE_ALLOC;
use crate::mutex::Mutex;
use crate::net::ipv4;
use crate::process::{GlobalScheduler, Id, KernelImpl, KernelProcess, KernProcessCtx, Priority, Process};
use crate::process::fd::FileDescriptor;
use crate::smp::core;
use crate::traps::irq::Irq;
use crate::traps::KernelTrapFrame;
use crate::usb::usb_thread;
use crate::vm::VMManager;

pub static KERNEL_IRQ: Irq<KernelImpl> = Irq::uninitialized();
pub static KERNEL_SCHEDULER: GlobalScheduler<KernelImpl> = GlobalScheduler::uninitialized();

pub static KERNEL_TIMER: CoreLazy<TimerController<KernelTrapFrame, VirtualCounter>> = CoreLocal::new_lazy(|| TimerController::new());
pub static KERNEL_CORES: SingleSetSemaphore<usize> = SingleSetSemaphore::new();

fn network_thread(ctx: KernProcessCtx) {

    // let serial = crate::mbox::with_mbox(|mbox| mbox.serial_number()).expect("could not get serial number");
    //
    // if serial == 0 {
    //     kprintln!("[net] skipping network thread init, qemu detected");
    //     kernel_api::syscall::exit();
    // }

    info!("starting net thread...");

    if !hw::not_pi() {
        timing::sleep_phys(Duration::from_millis(100));
        crate::mbox::with_mbox(|mbox| mbox.set_power_state(0x00000003, true));
    }
    timing::sleep_phys(Duration::from_millis(5));

    unsafe {
        NET.initialize();
    }

    NET.critical(|net| {
        let my_ip = ipv4::Address::from(&[10, 45, 52, 130]);

        net.tcp.add_listening_port((my_ip, 300), Box::new(|sink, source| {
            let mut proc = Process::kernel_process(String::from("net/echo"), |ctx| {
                use crate::iosync::{SyncRead, SyncWrite};

                let (source, sink) = ctx.get_stdio_or_panic();

                loop {
                    let mut buf = [0u8; 1024];

                    match source.read(&mut buf) {
                        Ok(0) => kernel_call::syscall::wait_waitable(source.clone()),
                        Ok(n) => {
                            let mut left = &mut buf[..];

                            while left.len() > 0 {
                                match sink.write(left) {
                                    Ok(0) => kernel_call::syscall::wait_waitable(sink.clone()),
                                    Ok(n) => {
                                        left = &mut left[n..];
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }).or(ioerr!(Other, "foo"))?;

            proc.set_stdio(Arc::new(source), Arc::new(sink));

            KERNEL_SCHEDULER.add(proc);

            Ok(())
        }));


        net.tcp.add_listening_port((my_ip, 100), Box::new(|sink, source| {
            let mut proc = KernelProcess::kernel_process(String::from("net shell"), my_net_shell)
                .or(ioerr!(Other, "foo"))?;

            proc.set_stdio(Arc::new(source), Arc::new(sink));

            KERNEL_SCHEDULER.add(proc);

            Ok(())
        }));
    });

    loop {
        if !NET.critical(|n| n.dispatch()) {
            kernel_api::syscall::sleep(Duration::from_micros(1000)).ok();
        }
    }
}

fn my_net_shell(ctx: KernProcessCtx) {
    let (source, sink) = ctx.get_stdio_or_panic();

    shell::Shell::new("% ", WaitingSourceWrapper::new(source), SinkWrapper::new(sink)).shell_loop();

    kernel_api::syscall::exit();
}

fn my_thread(ctx: KernProcessCtx) {
    let (source, sink) = ctx.get_stdio_or_panic();

    // WaitingSourceWrapper::new(source)
    shell::Shell::new("$ ", WaitingSourceWrapper::new(source), SinkWrapper::new(sink)).shell_loop();
}

fn led_blink() -> ! {
    let mut g = gpio::Gpio::new(29).into_output();
    loop {
        g.set();
        kernel_api::syscall::sleep(Duration::from_millis(250)).ok();
        // timer::spin_sleep(Duration::from_millis(250));
        g.clear();
        kernel_api::syscall::sleep(Duration::from_millis(250)).ok();
        // timer::spin_sleep(Duration::from_millis(250));
    }
}

fn configure_timer() {
    // TODO only used on Pi
    KERNEL_IRQ.register_core(crate::smp::core(), CoreInterrupt::CNTVIRQ, Box::new(|tf| {
        smp::no_interrupt(|| {
            KERNEL_TIMER.process_timers(tf, |func| func());
        });
    }));

    if smp::core() == 0 {
        // perf::prepare();

        let mut sample_delay = Duration::from_micros(10);
        // TODO is qemu
        if true {
            sample_delay = Duration::from_micros(500);
        }

        KERNEL_TIMER.add(10, timing::time_to_cycles::<VirtualCounter>(sample_delay), Box::new(move |ctx| {
            if perf::record_event_kernel(ctx.data) {
                ctx.set_period(timing::time_to_cycles::<VirtualCounter>(sample_delay));
            } else {
                ctx.set_period(timing::time_to_cycles::<VirtualCounter>(Duration::from_millis(100)));
            }
            // TIMER_EVENTS.fetch_add(1, Ordering::Relaxed);
            // if IRQ_RECURSION_DEPTH.get() > 1 {
            //     TIMER_EVENTS_EXC.fetch_add(1, Ordering::Relaxed);
            // }
        }));

        // KERNEL_TIMER.add(timing::time_to_cycles::<VirtualCounter>(Duration::from_secs(50)), Box::new(|ctx| {
        //     ctx.remove_timer();
        //     // info!("Timer events: {}, exc:{}", TIMER_EVENTS.load(Ordering::Relaxed), TIMER_EVENTS_EXC.load(Ordering::Relaxed));
        //     perf::dump_events();
        // }));
    }
}


pub fn kernel_main() -> ! {
    info!("init irq");
    KERNEL_IRQ.initialize();

    info!("init irq2");

    // dont do uart fanciness
    // console_ext_init();
    KERNEL_IRQ.register(Interrupt::Aux, Box::new(|_| {
        console_interrupt_handler();
    }));

    // initialize local timers for all cores
    if matches!(hw::arch_variant(), ArchVariant::Pi(_)) {
        unsafe { (0x4000_0008 as *mut u32).write_volatile(0x8000_0000) };
        unsafe { (0x4000_0040 as *mut u32).write_volatile(0b1010) };
        unsafe { (0x4000_0044 as *mut u32).write_volatile(0b1010) };
        unsafe { (0x4000_0048 as *mut u32).write_volatile(0b1010) };
        unsafe { (0x4000_004C as *mut u32).write_volatile(0b1010) };
    }

    info!("init irq3");

    debug!("initing smp");

    let attrs: Vec<_> = aarch64::attr::iter_enabled().collect();
    info!("cpu attrs: {:?}", attrs);

    let enable_many_cores = !BootVariant::kernel_in_hypervisor() && false;
    let cores = if enable_many_cores { 4 } else { 1 };
    unsafe { SingleSetSemaphore::<usize>::set_racy(&KERNEL_CORES, cores) };

    if true {
        unsafe { smp::initialize(cores); }
        smp::wait_for_cores(cores);
    }

    smp::run_on_secondary_cores(|| {
        EXEC_CONTEXT.add_capabilities(EnumSet::only(ExecCapability::Allocation));
    });

    info!("VMM init");
    // error!("init VMM data structures");
    VMM.init_only();

    info!("setup kernel");

    // error!("enabling VMM on all cores!");
    smp::run_on_all_cores(|| {
        VMM.setup_kernel();
    });

    {
        // sanity checks
        // assert!(crossbeam_utils::atomic::AtomicCell::<EnumSet<ExecCapability>>::is_lock_free());
    }

    info!("registering mutex hooks");
    crate::mutex::register_hooks();

    info!("init irqs");
    if matches!(hw::arch_variant(), ArchVariant::Khadas(_)) {
        khadas::irq::init_stuff();
    }
    info!("done irqs");

    if BootVariant::kernel_in_hypervisor() {
        info!("Making hvc call");
        let start = pi::timer::current_time();
        hvc!(0);
        let end = pi::timer::current_time();
        info!("hvc took {:?}", end - start);
    }

    // error!("init Scheduler");
    configure_timer();
    unsafe {
        KERNEL_SCHEDULER.initialize_kernel();
    };

    use aarch64::regs::*;
    smp::run_on_secondary_cores(|| {
        unsafe {
            KERNEL_SCHEDULER.initialize_kernel();
        };
    });

    // info!("read debug info");
    // initialize_debug();

    info!("perf::prepare();");
    perf::prepare();

    if matches!(hw::arch_variant(), ArchVariant::Khadas(_)) {
        // spam on clock gating
        unsafe {
            for addr in [0xff63_c148u64, 0xff63c_0c8u64, 0xff63_c140u64, 0xff63_c0c0u64].iter() {
                let addr = (*addr) as *mut u32;
                info!("Addr: {:#x} => {:#032b}", addr as u64, addr.read_volatile());
                addr.write_volatile(0xffff_ffff);
                info!("Addr: {:#x} => {:#032b}", addr as u64, addr.read_volatile());
            }
        }
    }
    // #define DWC3_REG_OFFSET				0xC100

    // DTB items: usb_pwr, usb3_pcie_phy

    if matches!(hw::arch_variant(), ArchVariant::Pi(_)) {
        // TODO this causes problems on qemu...
        // pi::interrupt::Controller::new().enable(pi::interrupt::Interrupt::Aux);
    }

    if matches!(hw::arch_variant(), ArchVariant::Khadas(_)) {
        let b = khadas::uart::get_status_and_control();
        info!("UART regs: ({:#b}, {:#b})", b.0, b.1);
    }

    info!("filesystem init");
    unsafe { FILESYSTEM2.initialize() };

    info!("start some processes");
    {
        let mut proc = KernelProcess::kernel_process("shell".to_owned(), my_thread).unwrap();
        proc.set_stdio(Arc::new(Source::KernSerial), Arc::new(Sink::KernSerial));
        KERNEL_SCHEDULER.add(proc);
    }

    {
        let proc = KernelProcess::kernel_process_old("pipe".to_owned(), PipeService::task_func).unwrap();
        // proc.priority = Priority::Highest;
        KERNEL_SCHEDULER.add(proc);
    }

    {
        let proc = KernelProcess::kernel_process("usb".to_owned(), usb_thread).unwrap();
        KERNEL_SCHEDULER.add(proc);
    }

    // if true || !hw::is_qemu() || matches!(hw::arch_variant(), ArchVariant::Khadas(_)) {
    //     let mut proc = KernelProcess::kernel_process("net thread".to_owned(), network_thread).unwrap();
    //     proc.affinity.set_only(0);
    //     KERNEL_SCHEDULER.add(proc);
    // }

    // {
    //     let proc = KernelProcess::kernel_process("perf streamer".to_owned(), perf::perf_stream_proc).unwrap();
    //     KERNEL_SCHEDULER.add(proc);
    // }

    // {
    //     let proc = KernelProcess::kernel_process("balancer".to_owned(), tasks::core_balancing_thread).unwrap();
    //     KERNEL_SCHEDULER.add(proc);
    // }

    // {
    //     let proc = KernelProcess::kernel_process("net test".to_owned(), tasks::testing_send_thread).unwrap();
    //     KERNEL_SCHEDULER.add(proc);
    // }

    {
        let proc = KernelProcess::kernel_process("heartbeat".to_owned(), |_| {
            use core::fmt::Write;
            let mut w = hw::arch().early_writer();

            loop {
                write!(w, ".");
                kernel_api::syscall::sleep(Duration::from_secs(1));
            }
        }).unwrap();
        KERNEL_SCHEDULER.add(proc);
    }

    // for _ in 0..200 {
    //     let proc = KernelProcess::kernel_process("sleeper".to_owned(), |_ctx| {
    //
    //         loop {
    //             timing::sleep_phys(Duration::from_micros(500));
    //             kernel_api::syscall::sleep(Duration::from_secs(1));
    //         }
    //
    //     }).unwrap();
    //     KERNEL_SCHEDULER.add(proc);
    // }

    // {
    //     let proc = KernelProcess::kernel_process_old("led".to_owned(), led_blink).unwrap();
    //     KERNEL_SCHEDULER.add(proc);
    // }

    // {
    //     let proc = KernelProcess::kernel_process("display".to_owned(), display_manager::display_process).unwrap();
    //     KERNEL_SCHEDULER.add(proc);
    // }

    // {
    //     let mut proc = Process::load("/fib.bin").expect("failed to load");
    //     SCHEDULER.add(proc);
    // }

    info!("Starting other cores");
    smp::run_no_return(|| {
        let core = smp::core();
        timing::sleep_phys(Duration::from_millis(4 * core as u64));
        debug!("Core {} starting scheduler", core);
        KERNEL_SCHEDULER.start();

        error!("RIP RIP");
    });

    timing::sleep_phys(Duration::from_millis(50));
    info!("Core 0 starting scheduler");

    KERNEL_SCHEDULER.start();
}


