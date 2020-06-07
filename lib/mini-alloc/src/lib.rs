#![feature(ptr_internals)]
#![feature(optin_builtin_traits)]
#![feature(negative_impls)]
#![feature(const_fn)]
#![cfg_attr(not(test), no_std)]

mod allocator;
mod minibox;
pub(crate) mod mutex;

pub use allocator::*;
pub use minibox::MiniBox;

