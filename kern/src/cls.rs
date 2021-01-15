use core::ops::Deref;
use core::cell::Cell;
use aarch64::MPIDR_EL1;
use crate::mutex::Mutex;
use crate::iosync::{Global, Lazy};

pub const CORE_COUNT: usize = 4;

#[repr(align(128))]
struct Aligned<T>(T);

pub struct CoreLocal<T>([Aligned<T>; CORE_COUNT]);

// nothing is actually shared
unsafe impl<T> Sync for CoreLocal<T> {}

impl<T> CoreLocal<T> {
    pub fn new_func<F: Fn() -> T>(init: F) -> Self {
        CoreLocal([
            Aligned(init()), Aligned(init()),
            Aligned(init()), Aligned(init()),
        ])
    }
}

impl<T: Clone> CoreLocal<T> {
    pub fn new(init: T) -> Self {
        Self::new_func(|| init.clone())
    }
}

impl<T: Copy> CoreLocal<T> {
    pub const fn new_copy(init: T) -> Self {
        CoreLocal([
            Aligned(init), Aligned(init),
            Aligned(init), Aligned(init),
        ])
    }
}

impl<T> CoreLocal<Global<T>> {
    #[track_caller]
    pub const fn new_global(init: fn() -> T) -> Self {
        CoreLocal([
            Aligned(Global::new(init)), Aligned(Global::new(init)),
            Aligned(Global::new(init)), Aligned(Global::new(init)),
        ])
    }

    pub fn cross(&self, core: usize) -> &Global<T> {
        &self.0[core].0
    }
}

impl<T> CoreLocal<Lazy<T>> {
    #[track_caller]
    pub const fn new_lazy(init: fn() -> T) -> Self {
        CoreLocal([
            Aligned(Lazy::new(init)), Aligned(Lazy::new(init)),
            Aligned(Lazy::new(init)), Aligned(Lazy::new(init)),
        ])
    }
}

impl<T: Copy> CoreLocal<Cell<T>> {
    pub const fn new_cell(init: T) -> Self {
        CoreLocal([
            Aligned(Cell::new(init)), Aligned(Cell::new(init)),
            Aligned(Cell::new(init)), Aligned(Cell::new(init)),
        ])
    }

    pub fn cross(&self, core: usize) -> &Cell<T> {
        &self.0[core].0
    }
}

impl<T: Default> Default for CoreLocal<T> {
    fn default() -> Self {
        Self::new_func(|| T::default())
    }
}

impl<T> Deref for CoreLocal<T>  {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let core_id = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize };
        &self.0[core_id].0
    }
}


pub struct CoreMutex<T>([Mutex<T>; CORE_COUNT]);

// nothing is actually shared
unsafe impl<T> Sync for CoreMutex<T> {}

impl<T> CoreMutex<T> {
    pub fn new_func<F: Fn() -> T>(init: F) -> Self {
        CoreMutex([mutex_new!(init()), mutex_new!(init()), mutex_new!(init()), mutex_new!(init())])
    }

    pub fn cross(&self, core: usize) -> &Mutex<T> {
        &self.0[core]
    }
}

impl<T: Clone> CoreMutex<T> {
    pub fn new(init: T) -> Self {
        Self::new_func(|| init.clone())
    }
}

impl<T: Copy> CoreMutex<T> {
    pub const fn new_copy(init: T) -> Self {
        CoreMutex([mutex_new!(init), mutex_new!(init), mutex_new!(init), mutex_new!(init)])
    }
}

impl<T: Default> Default for CoreMutex<T> {
    fn default() -> Self {
        Self::new_func(|| T::default())
    }
}

impl<T> Deref for CoreMutex<T>  {
    type Target = Mutex<T>;

    fn deref(&self) -> &Self::Target {
        let core_id = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize };
        &self.0[core_id]
    }
}

pub type CoreGlobal<T> = CoreLocal<Global<T>>;
pub type CoreLazy<T> = CoreLocal<Lazy<T>>;
