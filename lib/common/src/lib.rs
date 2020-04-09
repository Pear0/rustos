#![feature(decl_macro)]
#![feature(optin_builtin_traits)]
#![cfg_attr(feature = "no_std", no_std)]

#[cfg(not(feature = "no_std"))]
extern crate core;

#[macro_use]
extern crate alloc;

pub mod mutex;
pub mod smp;