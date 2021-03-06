#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(core_intrinsics)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(llvm_asm)]
#![feature(global_asm)]
#![feature(coerce_unsized)]
#![feature(auto_traits)]
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

use alloc::alloc::{GlobalAlloc, Layout};
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use core::time::Duration;

use crossbeam_utils::atomic::AtomicCell;
use enumset::EnumSet;

use aarch64::{CNTP_CTL_EL0, SP};
use allocator::Allocator;
use dsx::sys::{set_system_hooks, SystemHooks};
use fs::FileSystem;
use karch::capability::ExecCapability;
use kernel_api::syscall::exit;
use mpalloc::{NULL_ALLOC, ThreadedAlloc, ThreadLocalAlloc};
use net::ipv4;
use pi::{gpio, timer};
use pi::interrupt::CoreInterrupt;
use pigrate::Error;
use process::GlobalScheduler;
use shim::{io, ioerr};
use vm::VMManager;

use crate::allocator::{FullThreadLocal, MpAllocator, MpThreadLocal};
use crate::arm::PhysicalCounter;
use crate::cls::{CoreLazy, CoreLocal};
use crate::fs2::FileSystem2;
use crate::fs::handle::{SinkWrapper, SourceWrapper};
use crate::init::EL1_IN_HYPERVISOR;
use crate::iosync::{ReadWrapper, SyncRead, SyncWrite, WriteWrapper};
use crate::mbox::with_mbox;
use crate::mutex::Mutex;
use crate::net::GlobalNetHandler;
use crate::param::PAGE_SIZE;
use crate::process::{Id, KernelImpl, Process, Stack};
use crate::process::fd::FileDescriptor;
use crate::traps::IRQ_RECURSION_DEPTH;
use crate::traps::syndrome::Syndrome;

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
pub mod tasks;
pub mod timing;
pub mod traps;
pub mod usb;
pub mod virtualization;
pub mod vm;


pub static ALLOCATOR: Allocator = Allocator::uninitialized();

#[derive(Default)]
struct Foo;

impl ThreadLocalAlloc for Foo {
    unsafe fn alloc(&mut self, layout: Layout, del: &'static dyn GlobalAlloc) -> *mut u8 {
        del.alloc(layout)
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout, del: &'static dyn GlobalAlloc) {
        del.dealloc(ptr, layout)
    }
}

#[cfg_attr(not(test), global_allocator)]
pub(crate) static FOO_ALLOC: ThreadedAlloc<Foo, FullThreadLocal<Foo>> = ThreadedAlloc::<_, _>::new(FullThreadLocal::new_default());

// #[cfg_attr(not(test), global_allocator)]
pub static MP_ALLOC: MpAllocator = MpAllocator::new();

// pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();
pub static FILESYSTEM2: FileSystem2 = FileSystem2::uninitialized();
pub static NET: GlobalNetHandler = GlobalNetHandler::uninitialized();
pub static VMM: VMManager = VMManager::uninitialized();

static BOOT_VARIANT: AtomicUsize = AtomicUsize::new(BootVariant::Unknown as usize);

static EXEC_CONTEXT: CoreLazy<ExecContext> = CoreLocal::new_lazy(|| ExecContext {
    capabilities: AtomicU32::new(0),
    yielded_timers: AtomicBool::new(false),
});

#[derive(Debug)]
pub struct ExecContext {
    capabilities: AtomicU32,
    yielded_timers: AtomicBool,
}

impl ExecContext {
    fn to_u32(exec: EnumSet<ExecCapability>) -> u32 {
        unsafe { core::mem::transmute((exec, [0u8; 3])) }
    }

    fn to_enum(num: u32) -> EnumSet<ExecCapability> {
        unsafe { core::mem::transmute::<_, (EnumSet<ExecCapability>, [u8; 3])>(num) }.0
    }

    fn cas_loop<F>(&self, f: F) -> EnumSet<ExecCapability>
        where F: Fn(EnumSet<ExecCapability>) -> EnumSet<ExecCapability> {
        let mut existing = self.capabilities.load(Ordering::Relaxed);
        loop {
            let new = Self::to_u32(f(Self::to_enum(existing)));
            let old = self.capabilities.compare_and_swap(existing, new, Ordering::Relaxed);
            if old == existing {
                return Self::to_enum(new);
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

    pub fn add_capabilities_racy(&self, caps: EnumSet<ExecCapability>) {
        let combined = Self::to_enum(self.capabilities.load(Ordering::Relaxed)).union(caps);
        self.capabilities.store(Self::to_u32(combined), Ordering::Release);
    }

    pub fn restore_capabilities(&self, caps: EnumSet<ExecCapability>) {
        self.add_capabilities(caps);
        self.yield_for_timers();
    }

    pub fn remove_capabilities(&self, caps: EnumSet<ExecCapability>) {
        self.cas_loop(|c| c.difference(caps));
    }

    pub fn get_capabilities(&self) -> EnumSet<ExecCapability> {
        Self::to_enum(self.capabilities.load(Ordering::Relaxed))
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

struct DsxSystemHooks;

impl SystemHooks for DsxSystemHooks {
    fn current_time(&self) -> Duration {
        timing::clock_time_phys()
    }
}

static DSX_SYSTEM_HOOKS: DsxSystemHooks = DsxSystemHooks;


fn kmain(boot_hypervisor: bool) -> ! {
    // init_jtag();
    use crate::arm::GenericCounterImpl;

    kprintln!("earlier boot");

    kprintln!("foo {}", PhysicalCounter::get_counter());

    // This is so that the host computer can attach serial console/screen whatever.
    timing::sleep_phys(Duration::from_millis(500));

    unsafe { set_system_hooks(&DSX_SYSTEM_HOOKS) };

    kprintln!("early boot");
    logger::register_global_logger();

    info!("hello");

    unsafe {
        info!("init allocator");
        ALLOCATOR.initialize();

        info!("add alloc capability");
        EXEC_CONTEXT.add_capabilities_racy(EnumSet::only(ExecCapability::Allocation));

        info!("set alloc delegate");
        FOO_ALLOC.set_delegate(&ALLOCATOR);

        info!("init mpalloc");
        MP_ALLOC.initialize(&ALLOCATOR, MpThreadLocal::default())

        // debug!("init filesystem");
        // FILESYSTEM.initialize();
    }

    info!("registering reserved memory regions");

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
