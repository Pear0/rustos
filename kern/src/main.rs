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

use allocator::Allocator;
//use fs::FileSystem;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
//pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();

fn kmain() -> ! {

    // This is so that the host computer can attach serial console/screen whatever.
    timer::spin_sleep(Duration::from_millis(100));

    for atag in pi::atags::Atags::get() {
        kprintln!("{:?}", atag);
    }

    unsafe {
        ALLOCATOR.initialize();
//        FILESYSTEM.initialize();
    }

    // FIXME: Start the shell.

//    let mut pin = gpio::Gpio::new(16).into_output();
    // pin.set();

    let mut v = vec![];
    for i in 0..50 {
        v.push(i);
        kprintln!("{:?}", v);
    }

    shell::shell("> ");
}
