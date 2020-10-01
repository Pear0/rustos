use core::ops::{BitOrAssign, BitAndAssign};

#[repr(C, packed)]
pub struct Volatile<T>(T);

impl<T> Volatile<T> {
    pub fn get(&self) -> T {
        unsafe { (&self.0 as *const T).read_volatile() }
    }

    pub fn set(&mut self, value: T) {
        unsafe { ((&mut self.0) as *mut T).write_volatile(value) };
    }
}

impl<T: Clone> Clone for Volatile<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Default> Default for Volatile<T> {
    fn default() -> Self {
        Self(T::default())
    }
}

impl<T: BitOrAssign> BitOrAssign<T> for Volatile<T> {
    fn bitor_assign(&mut self, rhs: T) {
        let mut x = self.get();
        x |= rhs;
        self.set(x);
    }
}

impl<T: BitAndAssign> BitAndAssign<T> for Volatile<T> {
    fn bitand_assign(&mut self, rhs: T) {
        let mut x = self.get();
        x &= rhs;
        self.set(x);
    }
}


