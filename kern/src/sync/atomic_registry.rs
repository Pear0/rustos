use core::sync::atomic::AtomicUsize;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::sync::atomic::AtomicBool;
use alloc::sync::Arc;
use crossbeam_utils::atomic::AtomicCell;
use core::sync::atomic::Ordering;


pub struct RegistryGuard<T: RegistryGuarded> {
    register_index: AtomicUsize,
    registry: AtomicCell<Option<Arc<Registry<T>>>>,
}

impl<T: RegistryGuarded> RegistryGuard<T> {
    pub const fn new() -> Self {
        Self {
            register_index: AtomicUsize::new(usize::max_value()),
            registry: AtomicCell::new(None),
        }
    }

    pub fn is_registered(&self) -> bool {
        self.register_index.load(Ordering::Relaxed) != usize::max_value()
    }
}

impl<T: RegistryGuarded> Default for RegistryGuard<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl <T: RegistryGuarded> Drop for RegistryGuard<T> {
    fn drop(&mut self) {
        let reg = self.registry.swap(None);
        if let Some(reg) = reg {

            let idx = self.register_index.load(Ordering::Relaxed);
            if idx != usize::max_value() {

                loop {
                    let addr = reg.entries[idx].load(Ordering::Relaxed);

                    // somebody is holding us...
                    if addr & 1 == 1 {
                        continue;
                    }

                    if reg.entries[idx].compare_and_swap(addr, 0, Ordering::Relaxed) == addr {
                        break;
                    }

                }
            }
        }
    }
}

pub trait RegistryGuarded: Sized {
    fn guard(&self) -> &RegistryGuard<Self>;
}

pub struct Registry<T: RegistryGuarded> {
    entries: Vec<AtomicUsize>,
    _phantom: PhantomData<T>,
}

impl<T: RegistryGuarded> Registry<T> {
    pub fn new_size(size: usize) -> Arc<Registry<T>> {
        assert!(AtomicCell::<Option<Arc<Registry<T>>>>::is_lock_free());

        let mut s = Registry::<T> {
            entries: Vec::new(),
            _phantom: PhantomData::default(),
        };

        s.entries.reserve_exact(size);
        for i in 0..size {
            s.entries.push(Default::default());
        }
        Arc::new(s)
    }

    pub fn register(self: &Arc<Self>, t: &T) -> bool {
        let guard = t.guard();
        if guard.is_registered() {
            panic!("cannot double register");
        }

        for (i, entry) in self.entries.iter().enumerate() {
            // entry taken.
            if entry.load(Ordering::Relaxed) != 0 {
                continue;
            }

            let addr = t as *const T as usize;

            // someone beat us to this location.
            if entry.compare_and_swap(0, addr | 1, Ordering::Acquire) != 0 {
                continue;
            }

            // We own the location and have ownership.

            guard.register_index.store(i, Ordering::Relaxed);
            guard.registry.store(Some(self.clone()));

            // release ownership
            entry.store(addr, Ordering::Release);

            return true;
        }

        false
    }

    // func(None) -> prepare for a new entry, before any locks are taken
    // func(Some(ref)) -> use reference, holding destruction lock...
    pub fn for_all<F: FnMut(Option<&T>)>(&self, mut func: F) {

        'entries: for entry in self.entries.iter() {
            loop {
                let addr = entry.load(Ordering::Relaxed);
                if addr == 0 {
                    continue 'entries;
                }
                // someone holding lock...
                if addr & 1 == 1 {
                    continue;
                }

                func(None);

                // someone beat us to acquiring...
                if entry.compare_and_swap(addr, addr | 1, Ordering::Acquire) != addr {
                    continue;
                }

                func(Some(unsafe { &*(addr as *const T) }));

                entry.store(addr, Ordering::Release);
                break;
            }
        }

    }

}




