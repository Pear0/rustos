use core::cell::UnsafeCell;
use core::fmt;
use core::ops::{Deref, DerefMut, Drop};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use aarch64::{SCTLR_EL1, MPIDR_EL1, SP};
use core::time::Duration;
use core::fmt::Alignment::Left;
use core::sync::atomic::AtomicU64;
use crate::console::kprintln;
use crate::{smp, traps};

#[repr(align(32))]
pub struct Mutex<T> {
    data: UnsafeCell<T>,
    lock: AtomicBool,
    owner: AtomicUsize,
    name: &'static str,
    locked_at: AtomicU64,
    lock_name: UnsafeCell<&'static str>,
    lock_trace: UnsafeCell<[u64; 50]>,
}

unsafe impl<T: Send> Send for Mutex<T> { }
unsafe impl<T: Send> Sync for Mutex<T> { }

pub struct MutexGuard<'a, T: 'a> {
    lock: &'a Mutex<T>
}

impl<'a, T> !Send for MutexGuard<'a, T> { }
unsafe impl<'a, T: Sync> Sync for MutexGuard<'a, T> { }

impl<T> Mutex<T> {
    pub const fn new(name: &'static str, val: T) -> Mutex<T> {
        Mutex {
            lock: AtomicBool::new(false),
            owner: AtomicUsize::new(usize::max_value()),
            data: UnsafeCell::new(val),
            name,
            locked_at: AtomicU64::new(0),
            lock_name: UnsafeCell::new(""),
            lock_trace: UnsafeCell::new([0; 50]),
        }
    }
}

pub macro mutex_new {
    ($val:expr) => (Mutex::new(concat!(file!(), ":", line!()), $val))
}

pub macro m_lock {
($mutex:expr) => (($mutex).lock(concat!(file!(), ":", line!())))
}

pub macro m_lock_timeout {
($mutex:expr, $time:expr) => (($mutex).lock_timeout(concat!(file!(), ":", line!()), $time))
}



static ERR_LOCK: AtomicBool = AtomicBool::new(false);

impl<T> Mutex<T> {

    fn has_mmu(&self) -> bool {
        // possibly slightly wrong, not sure exactly what shareability settings
        // enable advanced control
        unsafe { SCTLR_EL1.get_value(SCTLR_EL1::M) != 0 }
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
        if self.has_mmu() {
            if self.lock.compare_and_swap(false, true, Ordering::SeqCst) == false {
                let this = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize };
                self.owner.store(this, Ordering::Relaxed);
                self.locked_at.store(pi::timer::current_time().as_millis() as u64, Ordering::SeqCst);

                unsafe { *self.lock_name.get() = name };

                let sp = SP.get();

                use crate::debug;
                debug::read_into_slice_clear(unsafe { &mut *self.lock_trace.get() }, debug::stack_scanner(sp, None));
                aarch64::dsb();

                Some(MutexGuard { lock: &self })
            } else {
                None
            }

        } else {
            let this = 0;
            if !self.lock.load(Ordering::Relaxed) || self.owner.load(Ordering::Relaxed) == this {
                self.lock.store(true, Ordering::Relaxed);
                self.owner.store(this, Ordering::Relaxed);
                Some(MutexGuard { lock: &self })
            } else {
                None
            }
        }
    }

    // Once MMU/cache is enabled, do the right thing here. For now, we don't
    // need any real synchronization.
    #[inline(always)]
    pub fn lock(&self, name: &'static str) -> MutexGuard<T> {
        // Wait until we can "aquire" the lock, then "acquire" it.
        // loop {
        //     match self.try_lock() {
        //         Some(guard) => return guard,
        //         None => continue
        //     }
        // }
        if let Some(g) = self.lock_timeout(name, Duration::from_secs(30)) {
            return g;
        }

        // grab lock
        while ERR_LOCK.compare_and_swap(false, true, Ordering::SeqCst) != false {}

        let locked_at = Duration::from_millis(self.locked_at.load(Ordering::SeqCst));
        let now = pi::timer::current_time();
        kprintln!("Lock {} locked for {:?}", self.name, now - locked_at);

        let owner = self.owner.load(Ordering::SeqCst);
        let mut locker = unsafe { *self.lock_name.get() };

        kprintln!("locker trace: {} @ {}", owner, locker);
        for addr in unsafe {&*self.lock_trace.get()}.iter().take_while(|x| **x != 0) {
            kprintln!("0x{:08x}", *addr);
        }

        let sp = aarch64::SP.get();

        let core = smp::core();
        let irq = traps::irq_depth();

        kprintln!("my trace: {} @ {}    irqd={}", core, name, irq);
        for addr in crate::debug::stack_scanner(sp, None) {
            kprintln!("0x{:08x}", addr);
        }

        if irq > 0 {
            use aarch64::regs::*;
            let el = traps::irq_el().unwrap_or(0);
            let esr = traps::irq_esr();
            let info = traps::irq_info();
            kprintln!("irq: 0x{:x}   {:?}    {:?}", el, esr, info);
        }

        ERR_LOCK.store(false, Ordering::SeqCst);
        panic!("failed to acquire lock: {}", self.name)
    }

    #[inline(never)]
    pub fn lock_timeout(&self, name: &'static str, timeout: Duration) -> Option<MutexGuard<T>> {
        let end = pi::timer::current_time() + timeout;
        loop {
            match self.try_lock(name) {
                Some(guard) => return Some(guard),
                None => {
                    if pi::timer::current_time() > end {
                        return None
                    }
                }
            }
        }
    }

    fn unlock(&self) {
        if self.has_mmu() {
            self.owner.store(0, Ordering::SeqCst);
            self.lock.store(false, Ordering::SeqCst);
        } else {
            self.lock.store(false, Ordering::Relaxed);
        }
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
