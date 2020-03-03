#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
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

use console::kprintln;

use pi::{gpio, timer};
use core::time::Duration;
use core::ops::DerefMut;
use pi::uart::MiniUart;
use shim::io::{self, Write, Read};

use fat32::traits::{BlockDevice};

use crate::console::CONSOLE;

use allocator::Allocator;
use fs::FileSystem;

use fat32::vfat::{VFatHandle, Dir as VDir, Metadata};
use fat32::traits::FileSystem as fs32FileSystem;
use fat32::traits::{Dir, Entry, File};
use crate::fs::sd::Sd;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();

fn init_jtag() {
   use gpio::{Function, Gpio};

   for pin in 22..=27 {
      Gpio::new(pin).into_alt(Function::Alt4);
   }
}

fn kmain() -> ! {

    init_jtag();

    // This is so that the host computer can attach serial console/screen whatever.
    timer::spin_sleep(Duration::from_millis(100));

    for atag in pi::atags::Atags::get() {
        kprintln!("{:?}", atag);
    }

    unsafe {
        kprintln!("Initing allocator");

        ALLOCATOR.initialize();

        kprintln!("Initing filesystem");

        FILESYSTEM.initialize();
    }

    shell::shell("> ");
}
