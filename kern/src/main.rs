#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![feature(ptr_internals)]
#![feature(raw_vec_internals)]
#![feature(panic_info_message)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use alloc::string::String;


pub mod allocator;
pub mod console;
pub mod fs;
pub mod mutex;
pub mod shell;
pub mod param;
pub mod process;
pub mod traps;
pub mod vm;

use console::{kprint, kprintln};

use pi::{gpio, timer};
use core::time::Duration;
use core::ops::DerefMut;
use pi::uart::MiniUart;
use shim::io::{self, Write, Read};

use fat32::traits::{BlockDevice};

use crate::console::CONSOLE;

use allocator::Allocator;
use fs::FileSystem;
use process::GlobalScheduler;
use traps::irq::Irq;
use vm::VMManager;

use fat32::vfat::{VFatHandle, Dir as VDir, Metadata};
use fat32::traits::FileSystem as fs32FileSystem;
use fat32::traits::{Dir, Entry, File};
use crate::fs::sd::Sd;
use crate::process::Process;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();
pub static SCHEDULER: GlobalScheduler = GlobalScheduler::uninitialized();
pub static VMM: VMManager = VMManager::uninitialized();
pub static IRQ: Irq = Irq::uninitialized();

fn init_jtag() {
   use gpio::{Function, Gpio};

   for pin in 22..=27 {
      Gpio::new(pin).into_alt(Function::Alt4);
   }
}

fn my_thread() -> ! {

    shell::shell("$ ");

    kprintln!("Shell died????, restarting pi...");
    unsafe { pi::pm::reset() };
}

fn led_blink() -> ! {

    let mut g = gpio::Gpio::new(29).into_output();
    loop {
        g.set();
        kernel_api::syscall::sleep(Duration::from_millis(250));
        // timer::spin_sleep(Duration::from_millis(250));
        g.clear();
        kernel_api::syscall::sleep(Duration::from_millis(250));
        // timer::spin_sleep(Duration::from_millis(250));
    }
}

fn hello_tas() -> ! {

    kernel_api::syscall::sleep(Duration::from_secs(3));

    for _ in 0..3 {
        kprintln!("Hello TAs! Use the `help` command for more info");
        kernel_api::syscall::sleep(Duration::from_secs(3));
    }

    kernel_api::syscall::exit();
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

    unsafe { SCHEDULER.initialize() };

    {
        let mut proc = Process::kernel_process_old(String::from("my_thread"), my_thread).unwrap();
        SCHEDULER.add(proc);
    }

    {
        let mut proc = Process::kernel_process_old(String::from("led blink"), led_blink).unwrap();
        SCHEDULER.add(proc);
    }

    {
        let mut proc = Process::kernel_process_old(String::from("hello tas"), hello_tas).unwrap();
        SCHEDULER.add(proc);
    }

    {
        let mut proc = Process::load("/fib.bin").expect("failed to load");
        SCHEDULER.add(proc);
    }

    SCHEDULER.start();

}
