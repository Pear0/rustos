use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::fmt;
use core::fmt::Alignment::Left;
use core::intrinsics::likely;
use core::ops::{Deref, DerefMut, Drop};
use core::panic::Location;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::sync::atomic::AtomicU64;
use core::time::Duration;

use crossbeam_utils::atomic::AtomicCell;

use aarch64::{MPIDR_EL1, SCTLR_EL1, SCTLR_EL2, SP};
use dsx::collections::registry_list::{IntrusiveInfo, IntrusiveNode, RegistryList};
use dsx::sync::mutex::{BootInfo, BootMutex, HookedMutex, HOOKS, LightMutex, LockableMutex, LockContext, LockHooks, MutexDataContainerSync, MutexDataSync};

use crate::{hw, smp, timing, traps};
use crate::arm::PhysicalCounter;

type EncUnit = u64;

struct Unit {
    /// max u2
    core: u64,

    /// max u16
    count: u64,
    recursion: u64, // max u16
}

fn encode_unit(unit: Unit) -> EncUnit {
    unit.core | (unit.count << 2) | (unit.recursion << 18)
}

fn decode_unit(unit: u64) -> Unit {
    Unit {
        core: unit & 0b11,
        count: (unit >> 2) & 0xFF_FF,
        recursion: (unit >> 18) & 0xFF_FF,
    }
}

pub struct KernBootInfo;

impl BootInfo for KernBootInfo {
    fn core() -> usize {
        unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize }
    }

    fn can_use_cas() -> bool {
        if unsafe { aarch64::current_el() } == 2 {
            unsafe { SCTLR_EL2.get_value(SCTLR_EL2::M) != 0 }
        } else {
            unsafe { SCTLR_EL1.get_value(SCTLR_EL1::M) != 0 }
        }
    }

    fn can_lock_without_cas() -> bool {
        Self::core() == 0
    }
}

struct Placeholder;

impl MutexDataSync for Placeholder {}

#[repr(align(128))]
pub struct LockInfo {
    pub assigned: AtomicBool,
    pub lock_op_count: AtomicUsize,
    pub lock_name: UnsafeCell<Option<&'static Location<'static>>>,
    pub locker_name: UnsafeCell<Option<&'static Location<'static>>>,
}

unsafe impl Sync for LockInfo {}

impl MutexDataSync for LockInfo {}

impl LockInfo {
    pub fn new() -> Self {
        Self {
            assigned: AtomicBool::new(false),
            lock_op_count: AtomicUsize::new(0),
            lock_name: UnsafeCell::new(None),
            locker_name: UnsafeCell::new(None),
        }
    }

    pub fn reset(&self) {
        unsafe {
            *self.locker_name.get() = None;
            *self.lock_name.get() = None;
        }
        self.lock_op_count.store(0, Ordering::Relaxed);
        self.assigned.store(false, Ordering::Release);
    }

    pub fn initialize(&self, ctx: &LockContext) {
        unsafe {
            *self.lock_name.get() = Some(ctx.lock_name);
        }
        self.lock_op_count.store(0, Ordering::Relaxed);
        self.on_locked(ctx);
    }

    pub fn on_locked(&self, ctx: &LockContext) {
        unsafe {
            *self.locker_name.get() = Some(ctx.locked_by);
        }
        self.lock_op_count.fetch_add(1, Ordering::Relaxed);
    }
}

pub struct KernMutexHooks {
    pub lock_count: AtomicUsize,
    pub lock_op_count: AtomicUsize,
}

impl LockHooks for KernMutexHooks {
    fn on_locked(&self, item: &mut MutexDataContainerSync, ctx: &LockContext) {
        self.lock_op_count.fetch_add(1, Ordering::Relaxed);
        if item.has_data() {
            if let Some(info) = item.as_ref::<LockInfo>() {
                info.on_locked(ctx);
            }
            return;
        }

        self.lock_count.fetch_add(1, Ordering::Relaxed);

        let infos = unsafe { MUTEX_INFOS.as_ref().unwrap() };

        for info in infos.iter() {
            match info.assigned.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed) {
                Ok(_) => {
                    info.initialize(ctx);
                    item.replace(info.clone());
                    return;
                }
                Err(_) => continue,
            }
        }

        // assign placeholder since we couldn't acquire a lock info
        item.replace(unsafe { MUTEX_PLACEHOLDER.as_ref().unwrap().clone() });
    }

    fn lock_dropped(&self, item: MutexDataContainerSync) {
        if item.has_data() {
            if let Some(info) = item.as_ref::<LockInfo>() {
                info.reset();
            }

            self.lock_count.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

pub static KERN_MUTEX_HOOKS: KernMutexHooks = KernMutexHooks {
    lock_count: AtomicUsize::new(0),
    lock_op_count: AtomicUsize::new(0),
};

static mut MUTEX_PLACEHOLDER: Option<Arc<Placeholder>> = None;
pub static mut MUTEX_INFOS: Option<Vec<Arc<LockInfo>>> = None;

pub fn register_hooks() {
    unsafe {
        MUTEX_PLACEHOLDER = Some(Arc::new(Placeholder));

        {
            let mut infos = Vec::new();
            infos.reserve_exact(32);
            for _ in 0..32 {
                infos.push(Arc::new(LockInfo::new()));
            }

            MUTEX_INFOS = Some(infos);
        }

        HOOKS = &KERN_MUTEX_HOOKS;
    }
}

#[derive(Debug)]
pub struct Mutex<T>(HookedMutex<BootMutex<T, KernBootInfo>>);

impl<T> Mutex<T> {
    #[track_caller]
    pub const fn new(value: T) -> Self {
        Mutex(HookedMutex::new(BootMutex::new(value)))
    }
}

impl<T> Deref for Mutex<T> {
    type Target = HookedMutex<BootMutex<T, KernBootInfo>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Mutex<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[macro_export]
macro_rules! mutex_new {
    ($val:expr) => ($crate::mutex::Mutex::new($val))
}

#[macro_export]
macro_rules! m_lock {
    ($mutex:expr) => (($mutex).lock())
}

#[macro_export]
macro_rules! m_lock_timeout {
    ($mutex:expr, $time:expr) => (($mutex).lock_timeout($time))
}
