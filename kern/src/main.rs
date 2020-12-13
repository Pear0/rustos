#![feature(alloc_error_handler)]
#![feature(const_fn)]
// #![feature(const_fn_fn_ptr_basics)]
#![feature(core_intrinsics)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(llvm_asm)]
#![feature(track_caller)]
#![feature(global_asm)]
#![feature(coerce_unsized)]
#![feature(optin_builtin_traits)]
#![feature(negative_impls)]
#![feature(ptr_internals)]
#![feature(raw_vec_internals)]
#![feature(panic_info_message)]
#![feature(c_variadic)]
#![feature(naked_functions)]
#![feature(const_caller_location)]

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(safe_packed_borrows)]
#![allow(dead_code)]
#![allow(unused_must_use)]

#![allow(unreachable_code)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_parens)]
#![allow(unused_braces)]
#![allow(unused_variables)]


#[macro_use]
extern crate aarch64;
extern crate alloc;
#[macro_use]
extern crate downcast_rs;
#[macro_use]
extern crate enumset;
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
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
use crate::init::EL1_IN_HYPERVISOR;
use crate::iosync::{ReadWrapper, SyncRead, SyncWrite, WriteWrapper};
use crate::mbox::with_mbox;
use crate::mutex::Mutex;
use crate::net::GlobalNetHandler;
use crate::param::PAGE_SIZE;
use crate::process::{Id, KernelImpl, Process, Stack};
use crate::process::fd::FileDescriptor;
use crate::traps::syndrome::Syndrome;
use crate::arm::PhysicalCounter;
use crate::fs2::FileSystem2;
use crate::traps::IRQ_RECURSION_DEPTH;
use crate::allocator::{MpAllocator, MpThreadLocal};
use mpalloc::NULL_ALLOC;
use crate::cls::{CoreLocal, CoreLazy};
use enumset::EnumSet;
use karch::capability::ExecCapability;
use serde::de::Unexpected::Enum;
use crossbeam_utils::atomic::AtomicCell;
use kernel_api::syscall::exit;

#[macro_use]
pub mod console;
#[macro_use]
pub mod mutex;

#[cfg(not(test))]
mod init;

pub mod allocator;
pub mod arm;
pub mod cls;
pub mod collections;
mod compat;
pub mod debug;
mod device_tree;
pub mod display;
mod display_manager;
pub mod driver;
pub mod fs;
pub mod fs2;
pub mod hw;
mod hyper;
pub mod iosync;
mod kernel;
pub mod kernel_call;
mod logger;
pub mod mbox;
pub mod mini_allocators;
pub mod net;
pub mod perf;
pub mod pigrate_server;
pub mod shell;
pub mod smp;
pub mod sync;
pub mod param;
pub mod process;
pub mod timing;
pub mod traps;
pub mod usb;
pub mod virtualization;
pub mod vm;


pub static ALLOCATOR: Allocator = Allocator::uninitialized();

#[cfg_attr(not(test), global_allocator)]
pub static MP_ALLOC: MpAllocator = MpAllocator::new();

// pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();
pub static FILESYSTEM2: FileSystem2 = FileSystem2::uninitialized();
pub static NET: GlobalNetHandler = GlobalNetHandler::uninitialized();
pub static VMM: VMManager = VMManager::uninitialized();

static BOOT_VARIANT: AtomicUsize = AtomicUsize::new(BootVariant::Unknown as usize);

static EXEC_CONTEXT: CoreLazy<ExecContext> = CoreLocal::new_lazy(|| ExecContext {
    capabilities: AtomicCell::new(ExecCapability::empty_set()),
    yielded_timers: AtomicBool::new(false),
});

#[derive(Debug)]
pub struct ExecContext {
    capabilities: AtomicCell<EnumSet<ExecCapability>>,
    yielded_timers: AtomicBool,
}

impl ExecContext {
    fn cas_loop<F>(&self, f: F) -> EnumSet<ExecCapability>
        where F: Fn(EnumSet<ExecCapability>) -> EnumSet<ExecCapability> {
        let mut existing = self.capabilities.load();
        loop {
            let new = f(existing);
            let old = self.capabilities.compare_and_swap(existing, new);
            if old == existing {
                return new;
            }
            existing = old;
        }
    }

    #[track_caller]
    #[inline(always)]
    pub fn lock_capability<F, R>(&self, caps: EnumSet<ExecCapability>, func: F) -> R
        where F: FnOnce() -> R {

        if !self.has_capabilities(caps) {
            panic!("attempt to lock_capabilities({:?}) without {:?}",
                   caps, caps.difference(self.get_capabilities()));
        }

        self.remove_capabilities(caps);

        let r = func();

        self.restore_capabilities(caps);
        r
    }

    pub fn yield_for_timers(&self) {
        if self.yielded_timers.load(Ordering::Acquire) {
            crate::kernel_call::syscall::yield_for_timers();
        }
    }

    pub fn add_capabilities(&self, caps: EnumSet<ExecCapability>) {
        self.cas_loop(|c| c.union(caps));
    }

    pub fn restore_capabilities(&self, caps: EnumSet<ExecCapability>) {
        self.add_capabilities(caps);
        self.yield_for_timers();
    }

    pub fn remove_capabilities(&self, caps: EnumSet<ExecCapability>) {
        self.cas_loop(|c| c.difference(caps));
    }

    pub fn get_capabilities(&self) -> EnumSet<ExecCapability> {
        self.capabilities.load()
    }

    pub fn has_capability(&self, cap: ExecCapability) -> bool {
        self.get_capabilities().contains(cap)
    }

    pub fn has_capabilities(&self, cap: EnumSet<ExecCapability>) -> bool {
        self.get_capabilities().is_superset(cap)
    }
}

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

    pub fn kernel_in_hypervisor() -> bool {
        Self::kernel() && EL1_IN_HYPERVISOR.load(Ordering::Relaxed)
    }
}

fn init_jtag() {
    use gpio::{Function, Gpio};

    for pin in 22..=27 {
        Gpio::new(pin).into_alt(Function::Alt4);
    }
}

pub fn can_make_syscall() -> bool {
    IRQ_RECURSION_DEPTH.get() == 0
}

fn kmain(boot_hypervisor: bool) -> ! {
    // init_jtag();
    use crate::arm::GenericCounterImpl;

    kprintln!("earlier boot");

    kprintln!("foo {}", PhysicalCounter::get_counter());

    // This is so that the host computer can attach serial console/screen whatever.
    timing::sleep_phys(Duration::from_millis(500));

    kprintln!("early boot");
    logger::register_global_logger();

    info!("hello");

    unsafe {
        debug!("init allocator");
        ALLOCATOR.initialize();

        EXEC_CONTEXT.add_capabilities(EnumSet::only(ExecCapability::Allocation));

        MP_ALLOC.initialize(&ALLOCATOR, MpThreadLocal::default())

        // debug!("init filesystem");
        // FILESYSTEM.initialize();
    }

    if let hw::ArchVariant::Khadas(_) = hw::arch_variant() {
        // TODO read this from device tree within karch.
        if !ALLOCATOR.with_internal_mut(|a| a.register_reserved_region((0x05000000, 0x300000))) {
            info!("failed to mark region 'secmon_reserved' as reserved.");
        }
    }

    if boot_hypervisor {
        BOOT_VARIANT.store(BootVariant::Hypervisor as usize, Ordering::SeqCst);
        hyper::hyper_main();
    } else {
        BOOT_VARIANT.store(BootVariant::Kernel as usize, Ordering::SeqCst);
        kernel::kernel_main();
    }
}
