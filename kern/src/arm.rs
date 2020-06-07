use alloc::boxed::Box;
use alloc::vec::Vec;
use core::marker::PhantomData;

use aarch64::regs::*;

pub trait GenericCounterImpl {
    fn set_interrupt_enabled(enabled: bool);

    fn interrupted() -> bool;

    fn set_timer(value: u64);

    fn set_compare(value: u64);

    /// Hz
    fn get_frequency() -> u64;

    fn get_counter() -> u64;
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

pub struct VirtualCounter();


pub struct TimerCtx<'a, T> {
    pub data: &'a mut T,
    remove: bool,
}

impl<'a, T> TimerCtx<'a, T> {
    fn new(data: &'a mut T) -> Self {
        Self { data, remove: false }
    }

    pub fn remove_timer(&mut self) {
        self.remove = true;
    }
}

type TimerFunc<T> = Box<dyn Fn(&mut TimerCtx<T>) + Send>;

struct Timer<T> {
    cycle_period: u64,
    next_compare: u64,
    func: TimerFunc<T>,
}

pub struct TimerController<T, C: GenericCounterImpl> {
    timers: Vec<Timer<T>>,
    remove_list: Vec<usize>,
    _phantom: PhantomData<C>,
}

impl<T, C: GenericCounterImpl> TimerController<T, C> {
    pub fn new() -> Self {
        Self { timers: Vec::new(), remove_list: Vec::new(), _phantom: PhantomData::default() }
    }

    fn set_compare(&mut self) {
        let min = self.timers.iter().map(|x| x.next_compare).min();
        if let Some(min) = min {
            C::set_compare(min);
        }
        C::set_interrupt_enabled(self.timers.len() > 0);
    }

    pub fn add(&mut self, period: u64, func: TimerFunc<T>) {
        let compare = C::get_counter() + period;
        self.timers.push(Timer { cycle_period: period, next_compare: compare, func });
        self.timers.sort_by_key(|x| x.cycle_period);

        self.set_compare();
    }

    pub fn process_timers(&mut self, data: &mut T) {
        if !C::interrupted() {
            return;
        }

        self.remove_list.clear();

        let now = C::get_counter();
        for (i, timer) in self.timers.iter_mut().enumerate() {
            if now >= timer.next_compare {
                let mut ctx = TimerCtx::new(data);
                (timer.func)(&mut ctx);

                if ctx.remove {
                    self.remove_list.push(i);
                }

                timer.next_compare = C::get_counter() + timer.cycle_period;
            }
        }

        for i in self.remove_list.drain(..).rev() {
            self.timers.remove(i);
        }

        self.set_compare();

    }
}






