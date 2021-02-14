#![feature(decl_macro)]
#![feature(auto_traits)]
#![feature(negative_impls)]
#![cfg_attr(feature = "no_std", no_std)]

#[cfg(not(feature = "no_std"))]
extern crate core;

#[macro_use]
extern crate alloc;

pub mod fmt;
pub mod mutex;
pub mod smp;