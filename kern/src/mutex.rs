use core::cell::UnsafeCell;
use core::fmt;
use core::ops::{Deref, DerefMut, Drop};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use aarch64::{SCTLR_EL1, MPIDR_EL1, SP, SCTLR_EL2};
use core::time::Duration;
use core::fmt::Alignment::Left;
use core::sync::atomic::AtomicU64;
use crate::{smp, traps};
use pi::uart::MiniUart;

type EncUnit = u64;

struct Unit {
    core: u64, // max u2
    count: u64, // max u16
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

#[repr(align(32))]
pub struct Mutex<T> {
    data: UnsafeCell<T>,
    lock_unit: AtomicU64,
    owner: AtomicUsize,
    name: &'static str,
    locked_at: AtomicU64,
    lock_name: UnsafeCell<&'static str>,
    lock_trace: UnsafeCell<[u64; 50]>,
}

unsafe impl<T: Send> Send for Mutex<T> { }
unsafe impl<T: Send> Sync for Mutex<T> { }

pub struct MutexGuard<'a, T: 'a> {
    lock: &'a Mutex<T>,
    recursion_enabled_count: usize,
}

impl<'a, T> !Send for MutexGuard<'a, T> { }
unsafe impl<'a, T: Sync> Sync for MutexGuard<'a, T> { }

impl<T> Mutex<T> {
    pub const fn new(name: &'static str, val: T) -> Mutex<T> {
        Mutex {
            lock_unit: AtomicU64::new(0),
            owner: AtomicUsize::new(usize::max_value()),
            data: UnsafeCell::new(val),
            name,
            locked_at: AtomicU64::new(0),
            lock_name: UnsafeCell::new(""),
            lock_trace: UnsafeCell::new([0; 50]),
        }
    }
}

#[macro_export]
macro_rules! mutex_new {
    ($val:expr) => ($crate::mutex::Mutex::new(concat!(file!(), ":", line!()), $val))
}

#[macro_export]
macro_rules! m_lock {
    ($mutex:expr) => (($mutex).lock(concat!(file!(), ":", line!())))
}

#[macro_export]
macro_rules! m_lock_timeout {
    ($mutex:expr, $time:expr) => (($mutex).lock_timeout(concat!(file!(), ":", line!()), $time))
}



static ERR_LOCK: AtomicBool = AtomicBool::new(false);

impl<T> Mutex<T> {

    fn has_mmu() -> bool {
        // possibly slightly wrong, not sure exactly what shareability settings
        // enable advanced control

        if unsafe { aarch64::current_el() } == 2 {
            unsafe { SCTLR_EL2.get_value(SCTLR_EL2::M) != 0 }
        } else {
            unsafe { SCTLR_EL1.get_value(SCTLR_EL1::M) != 0 }
        }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub unsafe fn unsafe_leak(&self) -> &mut T {
        &mut *self.data.get()
    }

    // Once MMU/cache is enabled, do the right thing here. For now, we don't
    // need any real synchronization.
    pub fn try_lock(&self, name: &'static str) -> Option<MutexGuard<T>> {
        if Self::has_mmu() {
            let this = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize };
            let current_unit = self.lock_unit.load(Ordering::Relaxed);
            let mut unit = decode_unit(current_unit);

            // somebody is locking this lock and hasn't performed an unlock.
            if unit.count != unit.recursion {
                return None;
            }

            // recursive locking is not allowed across cores. but if count == 0, cores
            // doesn't matter (first lock).
            if unit.count > 0 && unit.core as usize != this {
                return None;
            }

            unit.core = this as u64;
            unit.count += 1;

            // can we acquire lock
            if self.lock_unit.compare_and_swap(current_unit, encode_unit(unit), Ordering::SeqCst) != current_unit {
                return None;
            }

            self.owner.store(this, Ordering::Relaxed);
            self.locked_at.store(pi::timer::current_time().as_millis() as u64, Ordering::SeqCst);

            unsafe { *self.lock_name.get() = name };

            let sp = SP.get();

            use crate::debug;
            // debug::read_into_slice_clear(unsafe { &mut *self.lock_trace.get() }, debug::stack_scanner(sp, None));
            // aarch64::dsb();

            Some(MutexGuard { lock: &self, recursion_enabled_count: 0 })

        } else {
            let this = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize };
            if self.lock_unit.load(Ordering::SeqCst) == 0 {
                self.lock_unit.store(encode_unit(Unit { recursion: 0, core: this as u64, count: 1 }), Ordering::SeqCst);
                self.owner.store(this, Ordering::Relaxed);
                Some(MutexGuard { lock: &self, recursion_enabled_count: 0 })
            } else {
                None
            }
        }
    }

    // Once MMU/cache is enabled, do the right thing here. For now, we don't
    // need any real synchronization.
    #[inline(always)]
    pub fn lock(&self, name: &'static str) -> MutexGuard<T> {
        use core::fmt::Write;

        if let Some(g) = self.lock_timeout(name, Duration::from_secs(30)) {
            return g;
        }

        // grab lock
        while ERR_LOCK.compare_and_swap(false, true, Ordering::SeqCst) != false {}

        let mut uart = MiniUart::new_opt_init(false);

        let locked_at = Duration::from_millis(self.locked_at.load(Ordering::SeqCst));
        let now = pi::timer::current_time();
        writeln!(&mut uart, "Lock {} locked for {:?}", self.name, now - locked_at);

        let owner = self.owner.load(Ordering::SeqCst);
        let mut locker = unsafe { *self.lock_name.get() };

        writeln!(&mut uart, "locker trace: {} @ {}", owner, locker);
        for addr in unsafe {&*self.lock_trace.get()}.iter().take_while(|x| **x != 0) {
            writeln!(&mut uart, "0x{:08x}", *addr);
        }

        let sp = aarch64::SP.get();

        let core = smp::core();
        let irq = traps::irq_depth();

        writeln!(&mut uart, "my trace: {} @ {}    irqd={}", core, name, irq);
        for addr in crate::debug::stack_scanner(sp, None) {
            writeln!(&mut uart, "0x{:08x}", addr);
        }

        if irq > 0 {
            use aarch64::regs::*;
            let el = traps::irq_el().unwrap_or(0);
            let esr = traps::irq_esr();
            let info = traps::irq_info();
            writeln!(&mut uart, "irq: 0x{:x}   {:?}    {:?}", el, esr, info);
        }

        ERR_LOCK.store(false, Ordering::SeqCst);
        panic!("failed to acquire lock: {}", self.name)
    }

    #[inline(never)]
    pub fn lock_timeout(&self, name: &'static str, timeout: Duration) -> Option<MutexGuard<T>> {
        let end = pi::timer::current_time() + timeout;
        let mut wait_amt = Duration::from_micros(1);
        loop {
            match self.try_lock(name) {
                Some(guard) => return Some(guard),
                None => {
                    pi::timer::spin_sleep(wait_amt);
                    wait_amt += wait_amt; // double wait amt

                    if pi::timer::current_time() > end {
                        return None
                    }
                }
            }
        }
    }

    fn increment_recursion(&self) {
        if !Self::has_mmu() {
            panic!("cannot use increment_recursion() before CAS is available");
        }
        let mut unit = decode_unit(self.lock_unit.load(Ordering::Relaxed));
        unit.recursion += 1;
        self.lock_unit.store(encode_unit(unit), Ordering::Relaxed);
    }

    fn decrement_recursion(&self) {
        if !Self::has_mmu() {
            panic!("cannot use increment_recursion() before CAS is available");
        }
        let mut unit = decode_unit(self.lock_unit.load(Ordering::Relaxed));
        unit.recursion -= 1;
        self.lock_unit.store(encode_unit(unit), Ordering::Relaxed);
    }

    fn unlock(&self) {
        if Self::has_mmu() {
            self.owner.store(0, Ordering::SeqCst);

            let mut unit = decode_unit(self.lock_unit.load(Ordering::Relaxed));
            unit.count -= 1;

            self.lock_unit.store(encode_unit(unit), Ordering::SeqCst);
        } else {
            self.lock_unit.store(0, Ordering::Relaxed);
        }
    }
}

impl<'a, T: 'a> MutexGuard<'a, T> {

    pub fn recursion<R, F: FnOnce() -> R>(&mut self, func: F) -> R {
        if self.recursion_enabled_count == 0 {
            self.lock.increment_recursion();
        }
        self.recursion_enabled_count += 1;

        let result = func();

        self.recursion_enabled_count -= 1;

        if self.recursion_enabled_count == 0 {
            self.lock.decrement_recursion();
        }

        result
    }

}

impl<'a, T: 'a> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { & *self.lock.data.get() }
    }
}

impl<'a, T: 'a> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T: 'a> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.unlock()
    }
}

impl<T: fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock("fmt::Debug for Mutex") {
            Some(guard) => f.debug_struct("Mutex").field("data", &&*guard).finish(),
            None => f.debug_struct("Mutex").field("data", &"<locked>").finish()
        }
    }
}
