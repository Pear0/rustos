/// Align `addr` downwards to the nearest multiple of `align`.
///
/// The returned usize is always <= `addr.`
///
/// # Panics
///
/// Panics if `align` is not a power of 2.
pub fn align_down(addr: usize, align: usize) -> usize {
    if align.count_ones() != 1 {
        panic!("invalid align: {}", align);
    }

    addr & !(align - 1)
}

/// Align `addr` upwards to the nearest multiple of `align`.
///
/// The returned `usize` is always >= `addr.`
///
/// # Panics
///
/// Panics if `align` is not a power of 2
/// or aligning up overflows the address.
pub fn align_up(addr: usize, align: usize) -> usize {
    if align.count_ones() != 1 {
        panic!("invalid align: {}", align);
    }

    if align_down(addr, align) < addr {
        align_down(addr, align) + align
    } else {
        align_down(addr, align)
    }
}
