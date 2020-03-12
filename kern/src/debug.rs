

extern "C" {
    static __code_beg: u8;
    static __code_end: u8;
}

pub fn address_maybe_code(num: u64) -> bool {
    unsafe { num >= (&__code_beg as *const u8 as u64) && num <= (&__code_end as *const u8 as u64) }
}

#[inline(never)]
pub fn stack_scanner(mut sp: usize, stack_top: Option<usize>) -> impl Iterator<Item=u64> {
    sp = crate::allocator::util::align_up(sp, 8);
    let mut top: usize;
    if let Some(t) = stack_top {
        top = t;
    } else {
        top = core::cmp::min(sp + 4096, crate::allocator::util::align_up(sp, 64 * 1024));
    }

    let slice = unsafe { core::slice::from_raw_parts(sp as *const u64, ((top - sp) / 8) as usize) };

    slice.iter().map(|x| *x).filter(|n| address_maybe_code(*n))
}

pub fn read_into_slice_clear<T: Default, I: Iterator<Item=T>>(slice: &mut [T], iter: I) {
    for i in 0..slice.len() {
        slice[i] = T::default();
    }
    read_into_slice(slice, iter)
}

pub fn read_into_slice<T, I: Iterator<Item=T>>(slice: &mut [T], iter: I) {
    for (i, n) in iter.enumerate().take(slice.len()) {
        slice[i] = n;
    }
}