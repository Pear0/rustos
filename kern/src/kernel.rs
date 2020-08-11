use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::panic::Location;
use core::time::Duration;

use pi::gpio;
use pi::interrupt::{Interrupt, CoreInterrupt};
use shim::{io, ioerr};

use crate::{BootVariant, display_manager, hw, kernel_call, NET, shell, smp, timing, VMM};
use crate::console::{CONSOLE, console_ext_init, console_interrupt_handler};
use crate::fs::handle::{Sink, SinkWrapper, Source, SourceWrapper, WaitingSourceWrapper};
use crate::fs::service::PipeService;
use crate::mutex::Mutex;
use crate::net::ipv4;
use crate::process::{GlobalScheduler, Id, KernelImpl, KernelProcess, KernProcessCtx, Priority, Process};
use crate::process::fd::FileDescriptor;
use crate::traps::irq::Irq;
use crate::vm::VMManager;
use crate::cls::{CoreGlobal, CoreLocal};
use crate::arm::{TimerController, VirtualCounter, PhysicalCounter};
use crate::traps::KernelTrapFrame;
use crate::debug::initialize_debug;
use xhci::FlushType;
use usb_host::structs::USBDevice;
use usb_host::traits::USBHostController;
use usb_host::items::{ControlCommand, TypeTriple};
use usb_host::USBHost;

pub static KERNEL_IRQ: Irq<KernelImpl> = Irq::uninitialized();
pub static KERNEL_SCHEDULER: GlobalScheduler<KernelImpl> = GlobalScheduler::uninitialized();

pub static KERNEL_TIMER: CoreGlobal<TimerController<KernelTrapFrame, VirtualCounter>> = CoreLocal::new_global(|| TimerController::new());


fn network_thread() -> ! {

    // let serial = crate::mbox::with_mbox(|mbox| mbox.serial_number()).expect("could not get serial number");
    //
    // if serial == 0 {
    //     kprintln!("[net] skipping network thread init, qemu detected");
    //     kernel_api::syscall::exit();
    // }

    pi::timer::spin_sleep(Duration::from_millis(100));
    crate::mbox::with_mbox(|mbox| mbox.set_power_state(0x00000003, true));
    pi::timer::spin_sleep(Duration::from_millis(5));

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
            let mut proc = KernelProcess::kernel_process_old(String::from("net thread2"), my_net_thread2)
                .or(ioerr!(Other, "foo"))?;

            proc.detail.file_descriptors.push(FileDescriptor::read(Arc::new(source)));
            proc.detail.file_descriptors.push(FileDescriptor::write(Arc::new(sink)));

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

fn my_net_thread2() -> ! {
    let pid: Id = kernel_api::syscall::getpid();
    let (source, sink) = KERNEL_SCHEDULER.crit_process(pid, |f| {
        let f = f.unwrap();
        (f.detail.file_descriptors[0].read.as_ref().unwrap().clone(), f.detail.file_descriptors[1].write.as_ref().unwrap().clone())
    });

    shell::Shell::new("% ", SourceWrapper::new(source), SinkWrapper::new(sink)).shell_loop();

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
            KERNEL_TIMER.critical(|timer| timer.process_timers(tf));
        });
    }));

}

struct XHCIHal();

impl xhci::HAL for XHCIHal {
    fn current_time(&self) -> Duration {
        timing::clock_time::<PhysicalCounter>()
    }

    fn sleep(&self, dur: Duration) {
        timing::sleep_phys(dur);
    }

    fn memory_barrier(&self) {
        aarch64::dmb();
    }

    fn translate_addr(&self, addr: u64) -> u64 {
        addr
    }

    fn flush_cache(&self, addr: u64, len: u64, flush: FlushType) {
        match flush {
            FlushType::Clean => aarch64::clean_data_cache_region(addr, len),
            FlushType::Invalidate => aarch64::invalidate_data_cache_region(addr, len),
            FlushType::CleanAndInvalidate => aarch64::clean_and_invalidate_data_cache_region(addr, len),
        }
    }
}


pub fn kernel_main() -> ! {
    debug!("init irq");
    KERNEL_IRQ.initialize();

    // dont do uart fanciness
    // console_ext_init();
    KERNEL_IRQ.register(Interrupt::Aux, Box::new(|_| {
        console_interrupt_handler();
    }));

    // initialize local timers for all cores
    // unsafe { (0x4000_0008 as *mut u32).write_volatile(0x8000_0000) };
    // unsafe { (0x4000_0040 as *mut u32).write_volatile(0b1010) };
    // unsafe { (0x4000_0044 as *mut u32).write_volatile(0b1010) };
    // unsafe { (0x4000_0048 as *mut u32).write_volatile(0b1010) };
    // unsafe { (0x4000_004C as *mut u32).write_volatile(0b1010) };

    debug!("initing smp");

    let attrs: Vec<_> = aarch64::attr::iter_enabled().collect();
    debug!("cpu attrs: {:?}", attrs);

    let enable_many_cores = !BootVariant::kernel_in_hypervisor() && false;

    if true {
        let cores = if enable_many_cores { 4 } else { 1 };
        unsafe { smp::initialize(cores); }
        smp::wait_for_cores(cores);
    }

    info!("VMM init");
    // error!("init VMM data structures");
    VMM.init_only();

    // error!("enabling VMM on all cores!");
    smp::run_on_all_cores(|| {
        VMM.setup_kernel();
    });

    info!("init irqs");
    khadas::irq::init_stuff();
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

    // debug!("read debug info");
    // initialize_debug();

    // spam on clock gating
    unsafe {
        for addr in [0xff63_c148u64, 0xff63c_0c8u64, 0xff63_c140u64, 0xff63_c0c0u64].iter() {
            let addr = (*addr) as *mut u32;
            info!("Addr: {:#x} => {:#032b}", addr as u64, addr.read_volatile());
            addr.write_volatile(0xffff_ffff);
            info!("Addr: {:#x} => {:#032b}", addr as u64, addr.read_volatile());
        }
    }
    // #define DWC3_REG_OFFSET				0xC100

    // DTB items: usb_pwr, usb3_pcie_phy

    debug!("start some processes");

    // pi::interrupt::Controller::new().enable(pi::interrupt::Interrupt::Aux);

    // Stable

    // timing::benchmark("kernel pi::timer", |num| {
    //     for _ in 0..num {
    //         pi::timer::current_time();
    //     }
    // });
    //
    // timing::benchmark("kernel hvc", |num| {
    //     for _ in 0..num {
    //         hvc!(0);
    //     }
    // });

    {
        let loc = Location::caller();
        info!("Hello: {:?}", loc);
    }

    let b = khadas::uart::get_status_and_control();
    info!("UART regs: ({:#b}, {:#b})", b.0, b.1);

    // UART regs: (0b10000100000011111100000000, 0b1011000000000000)


    {
        let mut proc = KernelProcess::kernel_process("shell".to_owned(), my_thread).unwrap();
        proc.set_stdio(Arc::new(Source::KernSerial), Arc::new(Sink::KernSerial));
        KERNEL_SCHEDULER.add(proc);
    }

    {
        let mut proc = KernelProcess::kernel_process_old("pipe".to_owned(), PipeService::task_func).unwrap();
        proc.priority = Priority::Highest;
        KERNEL_SCHEDULER.add(proc);
    }

    {
        let mut proc = KernelProcess::kernel_process("pipe".to_owned(), |ctx| {
            use usb_host::traits::*;

            let addr = 0xff500000u64;


            let x = Box::leak(Box::new(XHCIHal()));
            xhci::init_dwc3(addr);

            let mut xx = xhci::Xhci::new(addr, x);

            let my_xhci = Arc::new(xhci::XhciWrapper(spin::Mutex::new(xx)));

            info!("created things");

            let mut host = USBHost::new();
            let dev = host.attach_root_hub(my_xhci);

            host.enumerate_devices(dev);

            info!("Done xhci code");

            // smp::no_interrupt(|| {
            //     match xx.do_stuff() {
            //         Ok(()) => info!("did stuff successfully"),
            //         Err(e) => error!("Error failed to do stuff: {:?}", e),
            //     }
            // });

        }).unwrap();

        KERNEL_SCHEDULER.add(proc);
    }

    // if !hw::is_qemu() {
    //     let mut proc = KernelProcess::kernel_process_old("net thread".to_owned(), network_thread).unwrap();
    //     proc.affinity.set_only(0);
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
    //     let proc = KernelProcess::kernel_process("hello TAs".to_owned(), |_| {
    //
    //         kernel_api::syscall::sleep(Duration::from_millis(1000));
    //
    //         for _ in 0..3 {
    //
    //             kprintln!("Hello TAs, there is a `help` command and `proc2` to list processes. Don't run `net tcp`, networking is disabled right now. I did not implement berkeley sockets btw");
    //
    //             kernel_api::syscall::sleep(Duration::from_millis(5000));
    //         }
    //
    //     }).unwrap();
    //     KERNEL_SCHEDULER.add(proc);
    // }

    // {
    //     let mut proc = Process::load("/fib.bin").expect("failed to load");
    //     SCHEDULER.add(proc);
    // }
    //
    // smp::run_on_secondary_cores(|| {
    //     kprintln!("Baz");
    // });

    smp::run_no_return(|| {
        let core = smp::core();
        pi::timer::spin_sleep(Duration::from_millis(4 * core as u64));
        debug!("Core {} starting scheduler", core);
        KERNEL_SCHEDULER.start();

        error!("RIP RIP");
    });

    timing::sleep_phys(Duration::from_millis(50));
    debug!("Core 0 starting scheduler");

    KERNEL_SCHEDULER.start();
}


