#![feature(ptr_internals)]
#![feature(auto_traits)]
#![feature(negative_impls)]
#![feature(const_fn)]
#![feature(const_fn_fn_ptr_basics)]
#![cfg_attr(not(test), no_std)]

mod allocator;
mod minibox;
pub(crate) mod mutex;

pub use allocator::*;
pub use minibox::MiniBox;

pub type AllocRef = &'static dyn Alloc;
