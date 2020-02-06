#![cfg_attr(feature = "no_std", no_std)]
#![feature(str_internals)]
#![feature(optin_builtin_traits)]
#![feature(never_type)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "no_std")]
mod no_std;
#[cfg(feature = "no_std")]
pub use self::no_std::*;

#[cfg(not(feature = "no_std"))]
mod std;
#[cfg(not(feature = "no_std"))]
pub use self::std::*;

#[macro_use]
pub mod macros;

#[cfg(test)]
mod tests;
