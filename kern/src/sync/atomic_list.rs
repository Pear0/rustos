use core::sync::atomic::AtomicPtr;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::marker::PhantomData;
use core::sync::atomic::AtomicBool;

#[derive(Debug, Default)]
pub struct InternSentinel<T: InternList> {
    next: AtomicUsize,
    iter_locked: AtomicBool,
    _phantom: PhantomData<T>,
}

impl<T: InternList> InternSentinel<T> {
    fn mark_for_deletion(&self) {
        loop {
            let old = self.next.load(Ordering::Relaxed);
            let new = old | 1;
            if self.next.compare_and_swap(old, new, Ordering::AcqRel) == old {
                return;
            }
        }
    }

    fn raw_set_next(&self, val: *const T) {
        self.next.store(val as usize, Ordering::Relaxed);
    }

    fn is_next_tail(&self) -> bool {
        self.next().is_none()
    }

    fn insert_next(&self, next: &T) {
        loop {
            let old_next = self.next.load(Ordering::Relaxed);
            next.get_sentinel().next.store(old_next, Ordering::Relaxed);

            let new_next = next as *const T as usize;

            if self.next.compare_and_swap(old_next, new_next, Ordering::Relaxed) == old_next {
                break;
            }
        }
    }

    fn next(&self) -> Option<*const T> {
        match self.next.load(Ordering::Relaxed) & !1 {
            0 => None,
            a => Some(a as *const T)
        }
    }
}

pub trait InternList: Sized + Sync {
    fn get_sentinel(&self) -> &InternSentinel<Self>;
}

pub struct AtomicList<T: InternList> {
    head: InternSentinel<T>,
}

impl<T: InternList> AtomicList<T> {
    pub const fn new() -> Self {
        Self {
            head: InternSentinel::default(),
        }
    }

    pub fn search(&self, ptr: *const T) -> (*const T, *const T) {
        type Ptr = *const InternSentinel<T>;
        type Ref = &'static InternSentinel<T>;
        unsafe {
            'search: loop {
                let mut t = (&self.head) as Ptr;
                let mut t_next = (t as Ref).next().unwrap_or(core::ptr::null());

                /* 1: Find left_node and right_node */
                loop {
                    if t_next.is_null() {




                    }


                }


            }
        }
    }

    pub fn insert(&self) {}

    pub fn delete(&self, val: &T) {
        let sentinel = val.get_sentinel();

        // Grab the iter lock, this way nobody can try to use the item while we remove it.
        loop {
            if sentinel.iter_locked.compare_and_swap(false, true, Ordering::Acquire) == false {
                break;
            }
        }

        sentinel.mark_for_deletion();

        let old_next = sentinel.next.load(Ordering::Relaxed);

        loop {



        }

    }


}