#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(coerce_unsized)]
#![feature(optin_builtin_traits)]
#![feature(ptr_internals)]
#![feature(raw_vec_internals)]
#![feature(panic_info_message)]
#![feature(c_variadic)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#![allow(unused_imports)]

extern crate alloc;
#[macro_use]
extern crate log;
#[macro_use]
extern crate modular_bitfield;
extern crate pigrate_core as pigrate;
#[macro_use]
extern crate serde;
extern crate serde_cbor;
#[macro_use]
extern crate shim;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::time::Duration;

use aarch64::{CNTP_CTL_EL0, SP};
use allocator::Allocator;
use fs::FileSystem;
use net::ipv4;
use pi::{gpio, timer};
use pi::interrupt::CoreInterrupt;
use pigrate::Error;
use process::GlobalScheduler;
use shim::{io, ioerr};
use vm::VMManager;

use crate::fs::handle::{SinkWrapper, SourceWrapper};
use crate::iosync::{ReadWrapper, SyncRead, SyncWrite, WriteWrapper};
use crate::mbox::with_mbox;
use crate::mutex::Mutex;
use crate::net::GlobalNetHandler;
use crate::param::PAGE_SIZE;
use crate::process::{Id, KernelImpl, Process, Stack};
use crate::process::fd::FileDescriptor;
use crate::traps::syndrome::Syndrome;
use core::sync::atomic::{AtomicUsize, Ordering};

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
pub mod display;
mod display_manager;
pub mod fs;
pub mod hw;
pub mod iosync;
mod kernel;
pub mod kernel_call;
mod logger;
pub mod mbox;
pub mod mini_allocators;
pub mod net;
pub mod pigrate_server;
pub mod shell;
pub mod smp;
pub mod sync;
pub mod param;
pub mod process;
pub mod traps;
pub mod vm;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();
pub static NET: GlobalNetHandler = GlobalNetHandler::uninitialized();
pub static VMM: VMManager = VMManager::uninitialized();

static BOOT_VARIANT: AtomicUsize = AtomicUsize::new(BootVariant::Unknown as usize);

#[derive(Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum BootVariant {
    Unknown,
    Kernel,
    Hypervisor,
}

impl BootVariant {
    pub fn get_variant() -> BootVariant {
        unsafe { core::mem::transmute(BOOT_VARIANT.load(Ordering::Relaxed)) }
    }

    pub fn kernel() -> bool {
        Self::get_variant() == BootVariant::Kernel
    }
}

fn init_jtag() {
    use gpio::{Function, Gpio};

    for pin in 22..=27 {
        Gpio::new(pin).into_alt(Function::Alt4);
    }
}

fn kmain() -> ! {
    init_jtag();

    // This is so that the host computer can attach serial console/screen whatever.
    timer::spin_sleep(Duration::from_millis(500));

    kprintln!("early boot");
    logger::register_global_logger();

    info!("hello");

    unsafe {
        debug!("init allocator");
        ALLOCATOR.initialize();
        debug!("init filesystem");
        FILESYSTEM.initialize();
    }

    BOOT_VARIANT.store(BootVariant::Kernel as usize, Ordering::SeqCst);
    kernel::kernel_main();
}
