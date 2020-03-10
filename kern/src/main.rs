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

use core::time::Duration;

use allocator::Allocator;
use console::kprintln;
use fs::FileSystem;
use pi::{gpio, timer};
use process::GlobalScheduler;
use traps::irq::Irq;
use vm::VMManager;
use alloc::boxed::Box;
use crate::traps::syndrome::Syndrome;

use crate::net::GlobalNetHandler;
use crate::process::{Process, Stack};
use alloc::sync::Arc;
use crate::io::{SyncWrite, ConsoleSync, ReadWrapper, SyncRead, WriteWrapper};
use crate::net::tcp::{SHELL_READ, SHELL_WRITE};
use alloc::borrow::ToOwned;
use crate::mutex::Mutex;
use alloc::vec::Vec;
use aarch64::SP;

#[cfg(not(test))]
mod init;

pub mod allocator;
pub mod cls;
mod compat;
pub mod console;
pub mod fs;
pub mod io;
pub mod mbox;
pub mod mutex;
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

    let serial = crate::mbox::with_mbox(|mbox| mbox.serial_number()).expect("could not get serial number");

    if serial == 0 {
        kprintln!("[net] skipping network thread init, qemu detected");
        kernel_api::syscall::exit();
    }

    unsafe {
        NET.initialize();
    }

    loop {
        if !NET.critical(|n| n.dispatch()) {
            kernel_api::syscall::sleep(Duration::from_micros(1000)).ok();
        }
    }
}

static CORE_REGISTER: Mutex<Option<Vec<u64>>> = Mutex::new(None);

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
    CORE_REGISTER.lock().replace(Vec::new());

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

    kprintln!("initing smp");

    // unsafe { smp::initialize(2); }

    // aarch64::dsb();
    // aarch64::isb();
    // aarch64::dmb();

    // unsafe {
    //     asm!("dsb     ishst
    // tlbi    vmalle1
    // dsb     ish
    // isb":::"memory");
    // }

    // smp::wait_for_cores(2);

    // kprintln!("Cores: {}", smp::count_cores());

    // smp::run_on_secondary_cores(|| {
    //
    //     // let el = unsafe { aarch64::current_el() };
    //     // kprintln!("Current EL: {}", el);
    //
    //     kprintln!("Hello!");
    // });



    kprintln!("foo");

    // VMM.initialize();

    VMM.init_only();

    smp::run_on_all_cores(|| {
        VMM.setup();
    });

    kprintln!("Initing Scheduler");

    unsafe {
        SCHEDULER.initialize();
    };

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
        let proc = Process::kernel_process_old("net thread".to_owned(),network_thread).unwrap();
        SCHEDULER.add(proc);
    }

    {
        let proc = Process::kernel_process_old("led".to_owned(),led_blink).unwrap();
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
        kprintln!("Luanching");
        SCHEDULER.start();

        kprintln!("RIP RIP");
    });

    kprintln!("starting");

    SCHEDULER.start();
}
