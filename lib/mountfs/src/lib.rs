#![feature(decl_macro)]
#![allow(unused_imports)]
#![cfg_attr(feature = "no_std", no_std)]

#[cfg(not(feature = "no_std"))]
extern crate core;

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate downcast_rs;

#[macro_use]
extern crate log;

pub mod fs;
pub(crate) mod meta;
pub mod mount;
pub(crate) mod null;

pub use null::NullFileSystem;
pub use meta::MetaFileSystem;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
