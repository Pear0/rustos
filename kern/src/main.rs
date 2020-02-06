#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

pub mod console;
pub mod mutex;
pub mod shell;

use console::kprintln;
use pi::{gpio, timer};
use core::time::Duration;
use pi::uart::MiniUart;

// FIXME: You need to add dependencies here to
// test your drivers (Phase 2). Add them as needed.

fn kmain() -> ! {
    // FIXME: Start the shell.

    let mut pin = gpio::Gpio::new(16).into_output();
    // pin.set();

    let mut uart = MiniUart::new();

    let mut toggle = false;

    for _ in 0..10 {
        pin.set();
        timer::spin_sleep(Duration::from_millis(100));
        pin.clear();
        timer::spin_sleep(Duration::from_millis(100));
    }

    loop {
//        pin.set();
         let byte = uart.read_byte();
//        pin.clear();
        uart.write_byte(byte);
        if toggle {
            pin.clear();
        } else {
            pin.set();
        }
        toggle = !toggle;
        timer::spin_sleep(Duration::from_millis(100));
    }

//    loop {
//        pin.set();
//
//        timer::spin_sleep(Duration::from_millis(100));
//
//        pin.clear();
//
//        timer::spin_sleep(Duration::from_millis(100));
//
//    }





}
