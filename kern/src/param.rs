use shim::{const_assert_eq, const_assert_size};

// we only support 64-bit
const_assert_size!(usize, 64 / 8);

use core::time::Duration;
pub use pi::common::*;

pub const PAGE_ALIGN: usize = 16;
pub const PAGE_SIZE: usize = 64 * 1024;
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);

pub const USER_MASK_BITS: usize = 34;
pub const KERNEL_MASK_BITS: usize = 32;

pub const USER_IMG_BASE: usize = 0xffff_ffff_c000_0000;
const_assert_eq!(
    USER_IMG_BASE,
    ((1 << USER_MASK_BITS) - 1) << (64 - USER_MASK_BITS)
);
pub const USER_STACK_BASE: usize = core::usize::MAX & PAGE_MASK; 
pub const USER_MAX_VM_SIZE: usize = 0x4000_0000;
const_assert_eq!(USER_IMG_BASE.wrapping_add(USER_MAX_VM_SIZE), 0);
pub const KERN_STACK_BASE: usize = 0x80_000;

/// The `tick` time.
// FIXME: When you're ready, change this to something more reasonable.
pub const TICK: Duration = Duration::from_secs(2);
