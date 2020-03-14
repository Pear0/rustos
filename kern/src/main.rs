#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![feature(ptr_internals)]
#![feature(raw_vec_internals)]
#![feature(panic_info_message)]
#![feature(c_variadic)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

extern crate alloc;
#[macro_use]
extern crate modular_bitfield;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::time::Duration;

use aarch64::{CNTP_CTL_EL0, SP};
use allocator::Allocator;
use fs::FileSystem;
use pi::{gpio, timer};
use pi::interrupt::CoreInterrupt;
use process::GlobalScheduler;
use traps::irq::Irq;
use vm::VMManager;

use crate::io::{ConsoleSync, ReadWrapper, SyncRead, SyncWrite, WriteWrapper};
use crate::mutex::Mutex;
use crate::net::GlobalNetHandler;
use crate::net::tcp::{SHELL_READ, SHELL_WRITE};
use crate::process::{Process, Stack};
use crate::traps::syndrome::Syndrome;

#[macro_use]
pub mod console;
#[macro_use]
pub mod mutex;

#[cfg(not(test))]
mod init;

pub mod allocator;
pub mod cls;
mod compat;
pub mod debug;
pub mod fs;
pub mod io;
pub mod mbox;
pub mod net;
pub mod shell;
pub mod smp;
pub mod param;
pub mod process;
pub mod traps;
pub mod vm;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();
pub static SCHEDULER: GlobalScheduler = GlobalScheduler::uninitialized();
pub static VMM: VMManager = VMManager::uninitialized();
pub static IRQ: Irq = Irq::uninitialized();
pub static NET: GlobalNetHandler = GlobalNetHandler::uninitialized();

fn init_jtag() {
    use gpio::{Function, Gpio};

    for pin in 22..=27 {
        Gpio::new(pin).into_alt(Function::Alt4);
    }
}

fn network_thread() {

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

    loop {
        if !NET.critical(|n| n.dispatch()) {
            kernel_api::syscall::sleep(Duration::from_micros(1000)).ok();
        }
    }

    //
    // loop {
    //     kernel_api::syscall::sleep(Duration::from_micros(1000)).ok();
    // }
}

static CORE_REGISTER: Mutex<Option<Vec<u64>>> = mutex_new!(None);

#[no_mangle]
fn core_bootstrap() -> ! {
    unsafe { smp::core_bootstrap(); }
}

#[inline(never)]
fn core_bootstrap_2() -> ! {
    kprintln!("Hello!");

    loop {}
}

fn my_thread() {
    kprintln!("initializing other threads");
    // CORE_REGISTER.lock().replace(Vec::new());

    kprintln!("all threads initialized");


    shell::shell("$ ");
}

fn my_net_thread() {
    let write: Arc<dyn SyncWrite> = Arc::new(SHELL_WRITE.get());
    let read: Arc<dyn SyncRead> = Arc::new(SHELL_READ.get());

    shell::Shell::new("$ ", ReadWrapper::new(read), WriteWrapper::new(write)).shell_loop();
}

fn led_blink() {
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


fn kmain() -> ! {
    init_jtag();

    // This is so that the host computer can attach serial console/screen whatever.
    timer::spin_sleep(Duration::from_millis(500));

    kprintln!("early boot");

    // for atag in pi::atags::Atags::get() {
    //     kprintln!("{:?}", atag);
    // }

    unsafe {
        ALLOCATOR.initialize();
        FILESYSTEM.initialize();
    }

    IRQ.initialize();

    mutex_new!(5);


    // for core in 0..smp::MAX_CORES {
    //     IRQ.register_core(core, CoreInterrupt::CNTPNSIRQ, Box::new(|tf| {
    //
    //         let v = unsafe { CNTPCT_EL0.get() };
    //         unsafe { CNTP_CVAL_EL0.set(v + 10000) };
    //
    //         // kprintln!("foo");
    //
    //     }));
    // }

    unsafe { (0x4000_0008 as *mut u32).write_volatile(0x8000_0000) };

    unsafe { (0x4000_0040 as *mut u32).write_volatile(0b1010) };
    unsafe { (0x4000_0044 as *mut u32).write_volatile(0b1010) };
    unsafe { (0x4000_0048 as *mut u32).write_volatile(0b1010) };
    unsafe { (0x4000_004C as *mut u32).write_volatile(0b1010) };


    kprintln!("initing smp");

    if true {
        let cores = 4;
        unsafe { smp::initialize(cores); }
        smp::wait_for_cores(cores);
    }

    // aarch64::dsb();
    // aarch64::isb();
    // aarch64::dmb();

    // unsafe {
    //     asm!("dsb     ishst
    // tlbi    vmalle1
    // dsb     ish
    // isb":::"memory");
    // }

    // kprintln!("Cores: {}", smp::count_cores());

    // smp::run_on_secondary_cores(|| {
    //
    //     // let el = unsafe { aarch64::current_el() };
    //     // kprintln!("Current EL: {}", el);
    //
    //     kprintln!("Hello!");
    // });

    kprintln!("foo {:?}", Syndrome::from(0x96000050));

    kprintln!("foo");

    // VMM.initialize();

    VMM.init_only();

    smp::run_on_all_cores(|| {
        VMM.setup();
    });

    kprintln!("Initing Scheduler");

    use aarch64::regs::*;

    unsafe {
        SCHEDULER.initialize();
    };

    smp::run_on_secondary_cores(|| {
        unsafe {
            SCHEDULER.initialize();
        };
    });

    {
        kprintln!("Creating first thread");
        let proc = Process::kernel_process_old("shell".to_owned(), my_thread).unwrap();
        SCHEDULER.add(proc);
    }

    {
        let proc = Process::kernel_process_old("net shell".to_owned(), my_net_thread).unwrap();
        SCHEDULER.add(proc);
    }

    {
        let mut proc = Process::kernel_process_old("net thread".to_owned(), network_thread).unwrap();
        proc.affinity.set_only(0);
        SCHEDULER.add(proc);
    }

    {
        let proc = Process::kernel_process_old("led".to_owned(), led_blink).unwrap();
        SCHEDULER.add(proc);
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
        kprintln!("Luanching {}", core);
        SCHEDULER.start();

        kprintln!("RIP RIP");
    });

    pi::timer::spin_sleep(Duration::from_millis(50));
    kprintln!("starting");

    SCHEDULER.start();
}
