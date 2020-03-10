use core::ops::Deref;
use core::cell::Cell;
use aarch64::MPIDR_EL1;

const CORE_COUNT: usize = 4;

pub struct CoreLocal<T>([Cell<T>; CORE_COUNT]);

impl<T> CoreLocal<T> {
    pub fn new_func<F: Fn() -> T>(init: F) -> Self {
        CoreLocal([Cell::new(init()), Cell::new(init()), Cell::new(init()), Cell::new(init())])
    }
}

impl<T: Clone> CoreLocal<T> {
    pub fn new(init: T) -> Self {
        Self::new_func(|| init.clone())
    }
}

impl<T: Default> Default for CoreLocal<T> {
    fn default() -> Self {
        Self::new_func(|| T::default())
    }
}

impl<T> Deref for CoreLocal<T>  {
    type Target = Cell<T>;

    fn deref(&self) -> &Self::Target {
        let core_id = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) as usize };
        &self.0[core_id]
    }
}


