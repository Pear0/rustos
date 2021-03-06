use core::fmt;
use alloc::collections::VecDeque;
use crate::smp;

pub struct CapacityRingBuffer<T> {
    deque: VecDeque<T>,
    capacity: usize,
    dropped_items: usize,
    total_queued_items: usize,
}

impl<T> CapacityRingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            deque: VecDeque::new(),
            capacity,
            dropped_items: 0,
            total_queued_items: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.deque.len()
    }

    pub fn insert(&mut self, item: T) -> bool {
        self.total_queued_items += 1;
        if self.deque.len() >= self.capacity {
            self.dropped_items += 1;
            return false;
        }
        self.deque.push_back(item);
        true
    }

    pub fn remove(&mut self) -> Option<T> {
        self.deque.pop_front()
    }

}

impl<T: fmt::Debug> fmt::Debug for CapacityRingBuffer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CapacityRingBuffer")
            .field("deque", &self.deque)
            .field("capacity", &self.capacity)
            .field("dropped_items", &self.dropped_items)
            .field("total_queued_items", &self.total_queued_items)
            .finish()
    }
}

pub struct MyKernRcu;

impl dsx::kern::KernInterruptRcu for MyKernRcu {
    fn critical_region<R, F: FnOnce() -> R>(func: F) -> R {
        smp::no_interrupt(func)
    }

    fn get_core_irq_count(core: usize) -> u32 {
        0
    }

    fn core_count() -> usize {
        1
    }

    fn my_core_index() -> usize {
        0
    }

    fn yield_for_other_cores() {
    }

    fn memory_barrier<T>(value: &T) {
        aarch64::dsb();
    }

    fn lock_failed() {
    }
}

pub type Rcu<T> = dsx::kern::InterruptRcu<T, MyKernRcu>;

