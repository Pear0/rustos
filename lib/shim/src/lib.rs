#![cfg_attr(feature = "no_std", no_std)]
#![feature(str_internals)]
#![feature(auto_traits)]
#![feature(never_type)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "no_std")]
mod no_std;

// we don't use no_std::* because intellij-rust cant infer around it properly.
#[cfg(all(feature = "no_std", feature = "alloc"))]
pub use self::no_std::ffi;
#[cfg(feature = "no_std")]
pub use self::no_std::io;
#[cfg(all(feature = "no_std", feature = "alloc"))]
pub use self::no_std::path;

#[cfg(not(feature = "no_std"))]
mod std;
#[cfg(not(feature = "no_std"))]
pub use self::std::*;

#[macro_use]
pub mod macros;

#[cfg(test)]
mod tests;
