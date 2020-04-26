use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use core::time::Duration;
use alloc::borrow::ToOwned;

use pi::gpio;
use shim::{io, ioerr};

use crate::{display_manager, hw, kernel_call, NET, shell, smp, VMM};
use crate::fs::handle::{SinkWrapper, SourceWrapper};
use crate::net::ipv4;
use crate::process::{GlobalScheduler, Id, KernelImpl, Process};
use crate::process::fd::FileDescriptor;

pub static KERNEL_SCHEDULER: GlobalScheduler<KernelImpl> = GlobalScheduler::uninitialized();

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
            let mut proc = Process::kernel_process_old(String::from("net thread2"), my_net_thread2)
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

fn my_thread() -> ! {
    kprintln!("initializing other threads");
    // CORE_REGISTER.lock().replace(Vec::new());

    kprintln!("all threads initialized");


    shell::shell("$ ");

    kernel_api::syscall::exit();
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


pub fn kernel_main() -> ! {


    // initialize local timers for all cores
    unsafe { (0x4000_0008 as *mut u32).write_volatile(0x8000_0000) };
    unsafe { (0x4000_0040 as *mut u32).write_volatile(0b1010) };
    unsafe { (0x4000_0044 as *mut u32).write_volatile(0b1010) };
    unsafe { (0x4000_0048 as *mut u32).write_volatile(0b1010) };
    unsafe { (0x4000_004C as *mut u32).write_volatile(0b1010) };


    debug!("initing smp");

    if true {
        let cores = 4;
        unsafe { smp::initialize(cores); }
        smp::wait_for_cores(cores);
    }

    debug!("init VMM data structures");
    VMM.init_only();

    info!("enabling VMM on all cores!");
    smp::run_on_all_cores(|| {
        VMM.setup();
    });

    info!("init Scheduler");
    unsafe {
        KERNEL_SCHEDULER.initialize();
    };

    use aarch64::regs::*;
    smp::run_on_secondary_cores(|| {
        unsafe {
            KERNEL_SCHEDULER.initialize();
        };
    });

    debug!("start some processes");

    {
        let proc = Process::kernel_process_old("shell".to_owned(), my_thread).unwrap();
        KERNEL_SCHEDULER.add(proc);
    }

    if !hw::is_qemu() {
        let mut proc = Process::kernel_process_old("net thread".to_owned(), network_thread).unwrap();
        proc.affinity.set_only(0);
        KERNEL_SCHEDULER.add(proc);
    }

    {
        let proc = Process::kernel_process_old("led".to_owned(), led_blink).unwrap();
        KERNEL_SCHEDULER.add(proc);
    }

    {
        let proc = Process::kernel_process("display".to_owned(), display_manager::display_process).unwrap();
        KERNEL_SCHEDULER.add(proc);
    }

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

    pi::timer::spin_sleep(Duration::from_millis(50));
    kprintln!("Core 0 starting scheduler");

    KERNEL_SCHEDULER.start();
}


