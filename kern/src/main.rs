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
use alloc::string::String;


pub mod allocator;
pub mod console;
pub mod fs;
pub mod mutex;
pub mod shell;

use console::kprintln;

use pi::{gpio, timer};
use core::time::Duration;
use pi::uart::MiniUart;

use fat32::traits::BlockDevice;

use crate::console::CONSOLE;

use allocator::Allocator;
use fs::FileSystem;

use fat32::traits::FileSystem as fs32FileSystem;
use fat32::traits::{Dir, Entry};
use crate::fs::sd::Sd;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();

fn kmain() -> ! {

    // This is so that the host computer can attach serial console/screen whatever.
    timer::spin_sleep(Duration::from_millis(100));

    timer::spin_sleep(Duration::from_millis(1000));

    for atag in pi::atags::Atags::get() {
        kprintln!("{:?}", atag);
    }

    unsafe {
        kprintln!("Initing allocator");

        ALLOCATOR.initialize();

        kprintln!("Initing filesystem");

        FILESYSTEM.initialize();
    }

    // FIXME: Start the shell.

//    let mut pin = gpio::Gpio::new(16).into_output();
    // pin.set();

    let entry = FILESYSTEM.open("/").expect("could not open");

    match entry {
        fat32::vfat::Entry::File(f) => kprintln!("{:?}", f),
        fat32::vfat::Entry::Dir(f) => {
            kprintln!("{:?}", f);

            let entries = f.entries();

            kprintln!("got entries");

            let entries = entries.expect("could not list");

            kprintln!("unwrapped entries");

            for entry in entries {
                kprintln!("{:?}", entry);
            }

        },
    }

    shell::shell("> ");
}
