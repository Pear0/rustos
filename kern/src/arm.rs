use alloc::boxed::Box;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::sync::atomic::Ordering;
use core::time::Duration;

use dsx::sync::mutex::LockableMutex;
use enumset::EnumSet;
use karch::capability::ExecCapability;

use aarch64::regs::*;

use crate::EXEC_CONTEXT;
use crate::mutex::Mutex;

const NANOS_PER_SEC: u64 = 1_000_000_000;

pub trait GenericCounterImpl {
    fn set_interrupt_enabled(enabled: bool);

    fn interrupted() -> bool;

    fn set_timer(value: u64);

    fn set_compare(value: u64);

    /// Hz
    fn get_frequency() -> u64;

    fn get_counter() -> u64;

    // Optional

    fn set_timer_duration(dur: Duration) {
        let cycles = dur.as_nanos() * (Self::get_frequency() as u128) / (NANOS_PER_SEC as u128);
        Self::set_timer(cycles as u64)
    }
}

pub struct HyperPhysicalCounter();

impl GenericCounterImpl for HyperPhysicalCounter {
    fn set_interrupt_enabled(enabled: bool) {
        unsafe {
            let mut value = CNTHP_CTL_EL2.get();
            if enabled {
                value &= !CNTHP_CTL_EL2::IMASK;
            } else {
                value |= CNTHP_CTL_EL2::IMASK;
            }
            CNTHP_CTL_EL2.set(value | CNTHP_CTL_EL2::ENABLE);
        }
    }

    fn interrupted() -> bool {
        unsafe { CNTHP_CTL_EL2.get() & CNTHP_CTL_EL2::ISTATUS != 0 }
    }

    fn set_timer(value: u64) {
        unsafe { CNTHP_TVAL_EL2.set(value) };
    }

    fn set_compare(value: u64) {
        unsafe { CNTHP_CVAL_EL2.set(value) };
    }

    fn get_frequency() -> u64 {
        unsafe { CNTFRQ_EL0.get() }
    }

    fn get_counter() -> u64 {
        aarch64::isb();
        unsafe { CNTPCT_EL0.get() }
    }
}

pub struct PhysicalCounter();

impl GenericCounterImpl for PhysicalCounter {
    fn set_interrupt_enabled(enabled: bool) {
        unsafe {
            let mut value = CNTP_CTL_EL0.get();
            if enabled {
                value &= !CNTV_CTL_EL0::IMASK;
            } else {
                value |= CNTV_CTL_EL0::IMASK;
            }
            CNTP_CTL_EL0.set(value | CNTV_CTL_EL0::ENABLE);
        }
    }

    fn interrupted() -> bool {
        unsafe { CNTP_CTL_EL0.get() & CNTHP_CTL_EL2::ISTATUS != 0 }
    }

    fn set_timer(value: u64) {
        unsafe { CNTP_TVAL_EL0.set(value) };
    }

    fn set_compare(value: u64) {
        unsafe { CNTP_CVAL_EL0.set(value) };
    }

    fn get_frequency() -> u64 {
        unsafe { CNTFRQ_EL0.get() }
    }

    fn get_counter() -> u64 {
        aarch64::isb();
        unsafe { CNTPCT_EL0.get() }
    }
}

pub struct VirtualCounter();

impl GenericCounterImpl for VirtualCounter {
    fn set_interrupt_enabled(enabled: bool) {
        unsafe {
            let mut value = CNTV_CTL_EL0.get();
            if enabled {
                value &= !CNTV_CTL_EL0::IMASK;
            } else {
                value |= CNTV_CTL_EL0::IMASK;
            }
            CNTV_CTL_EL0.set(value | CNTV_CTL_EL0::ENABLE);
        }
    }

    fn interrupted() -> bool {
        unsafe { CNTV_CTL_EL0.get() & CNTHP_CTL_EL2::ISTATUS != 0 }
    }

    fn set_timer(value: u64) {
        unsafe { CNTV_TVAL_EL0.set(value) };
    }

    fn set_compare(value: u64) {
        unsafe { CNTV_CVAL_EL0.set(value) };
    }

    fn get_frequency() -> u64 {
        unsafe { CNTFRQ_EL0.get() }
    }

    fn get_counter() -> u64 {
        aarch64::isb();
        unsafe { CNTPCT_EL0.get() }
    }
}

pub struct TimerCtx<'a, T> {
    pub data: &'a mut T,
    remove: bool,
    no_reschedule: bool,
    new_period: Option<u64>,
    defer_timer: bool,
}

impl<'a, T> TimerCtx<'a, T> {
    fn new(data: &'a mut T) -> Self {
        Self {
            data,
            remove: false,
            no_reschedule: false,
            new_period: None,
            defer_timer: false,
        }
    }

    pub fn defer_timer(&mut self) {
        self.defer_timer = true;
    }

    pub fn remove_timer(&mut self) {
        self.remove = true;
    }

    pub fn no_reschedule(&mut self) {
        self.no_reschedule = true;
    }

    pub fn set_period(&mut self, period: u64) {
        self.new_period = Some(period);
    }
}

type TimerFunc<T> = Box<dyn Fn(&mut TimerCtx<T>) + Send>;

struct Timer<T> {
    priority: u64,
    cycle_period: u64,
    next_compare: u64,
    enabled: bool,
    stat_check_count: usize,
    func: Option<TimerFunc<T>>,
    is_deferred: bool,
}

#[derive(Debug, Clone)]
pub struct TimerInfo {
    pub priority: u64,
    pub cycle_period: u64,
    pub next_compare: u64,
    pub enabled: bool,
    pub stat_check_count: usize,
}

struct TimerControllerImpl<T, C: GenericCounterImpl> {
    timers: Vec<Timer<T>>,
    min_priority: u64,
    _phantom: PhantomData<C>,
}

impl<T, C: GenericCounterImpl> TimerControllerImpl<T, C> {
    fn set_compare(&mut self) {
        let min = self.timers.iter()
            .filter(|x| x.enabled && !x.is_deferred)
            .map(|x| x.next_compare)
            .min();
        if let Some(min) = min {
            C::set_compare(min);
        }
        C::set_interrupt_enabled(self.timers.len() > 0);
    }
}

pub struct TimerController<T, C: GenericCounterImpl> {
    inner: Mutex<TimerControllerImpl<T, C>>
}

impl<T, C: GenericCounterImpl> TimerController<T, C> {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(TimerControllerImpl {
                timers: Vec::new(),
                min_priority: 0,
                _phantom: PhantomData::default(),
            })
        }
    }

    pub fn add(&self, priority: u64, period: u64, func: TimerFunc<T>) {
        let mut lock = self.inner.lock();

        let compare = C::get_counter() + period;
        lock.timers.push(Timer {
            priority,
            cycle_period: period,
            next_compare: compare,
            enabled: true,
            stat_check_count: 0,
            func: Some(func),
            is_deferred: false,
        });
        lock.timers.sort_by_key(|x| x.cycle_period);

        lock.set_compare();
    }

    // returns true if interrupts must be disabled.
    //noinspection RsDropRef
    #[inline(never)]
    pub fn process_timers<F>(&self, data: &mut T, mut int_func: F) -> bool where F: FnMut(&mut dyn FnMut()) {
        if !C::interrupted() {
            return false;
        }

        let mut lock = self.inner.lock();

        let mut updated_compare = false;

        let now = C::get_counter();
        let mut i = 0;
        while i < lock.timers.len() {
            let lock_min_priority = lock.min_priority;
            let mut timer = &mut lock.timers[i];
            let timer_priority = timer.priority;

            timer.stat_check_count += 1;

            if timer.enabled && now >= timer.next_compare && timer_priority >= lock_min_priority {
                let mut ctx = TimerCtx::new(data);

                // take ownership of function then drop reference.
                // This way we can access raw lock again and open a lock recursion context.
                let mut func = core::mem::replace(&mut timer.func, None);
                core::mem::drop(timer);

                // upgrade priority so that only higher priority timers execute.
                lock.min_priority = timer_priority + 1;

                lock.recursion(|| {
                    int_func(&mut || {
                        (func.as_mut().unwrap())(&mut ctx);
                    });
                });

                lock.min_priority = lock_min_priority;

                let mut timer = &mut lock.timers[i];
                timer.func = func;

                timer.is_deferred = ctx.defer_timer;
                updated_compare |= ctx.defer_timer;

                if ctx.remove {
                    timer.enabled = false;
                    continue;
                }

                if let Some(period) = &ctx.new_period {
                    timer.cycle_period = *period;
                }

                if !ctx.no_reschedule {
                    timer.next_compare = C::get_counter() + timer.cycle_period;
                    updated_compare = true;
                }
            }

            i += 1;
        }

        if updated_compare {
            lock.set_compare();

            let any_deferred = lock.timers.iter().any(|x| x.is_deferred);
            EXEC_CONTEXT.yielded_timers.store(any_deferred, Ordering::Release);
        }

        !updated_compare
    }

    pub fn get_timer_info(&self, infos: &mut Vec<TimerInfo>) -> usize {
        let lock = self.inner.lock();

        infos.clear();
        for timer in &lock.timers {
            if infos.len() == infos.capacity() {
                break;
            }

            infos.push(TimerInfo {
                priority: timer.priority,
                cycle_period: timer.cycle_period,
                next_compare: timer.next_compare,
                enabled: timer.enabled,
                stat_check_count: timer.stat_check_count,
            })
        }

        lock.timers.len()
    }
}






