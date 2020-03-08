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

use crate::net::GlobalNetHandler;
use crate::process::Process;
use alloc::sync::Arc;
use crate::io::{SyncWrite, ConsoleSync, ReadWrapper, SyncRead, WriteWrapper};
use crate::net::tcp::{SHELL_READ, SHELL_WRITE};

#[cfg(not(test))]
mod init;

pub mod allocator;
mod compat;
pub mod console;
pub mod fs;
pub mod io;
pub mod mbox;
pub mod mutex;
pub mod net;
pub mod shell;
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
    unsafe {
        NET.initialize();
    }

    loop {
        if !NET.critical(|n| n.dispatch()) {
            kernel_api::syscall::sleep(Duration::from_micros(1000)).ok();
        }
    }
}

fn my_thread() {

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
    timer::spin_sleep(Duration::from_millis(100));

    // for atag in pi::atags::Atags::get() {
    //     kprintln!("{:?}", atag);
    // }

    unsafe {
        ALLOCATOR.initialize();
        FILESYSTEM.initialize();
    }

    IRQ.initialize();

    VMM.initialize();
    kprintln!("Initing Scheduler");

    unsafe {
        SCHEDULER.initialize();


        // NET.initialize();
    };

    {
        let proc = Process::kernel_process(my_thread).unwrap();
        SCHEDULER.add(proc);
    }

    {
        let proc = Process::kernel_process(my_net_thread).unwrap();
        SCHEDULER.add(proc);
    }

    {
        let proc = Process::kernel_process(network_thread).unwrap();
        SCHEDULER.add(proc);
    }

    {
        let proc = Process::kernel_process(led_blink).unwrap();
        SCHEDULER.add(proc);
    }

    // {
    //     let mut proc = Process::load("/fib.bin").expect("failed to load");
    //     SCHEDULER.add(proc);
    // }

    SCHEDULER.start();
}
