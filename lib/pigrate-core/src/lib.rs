#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[macro_use]
extern crate serde;

extern crate serde_cbor;

pub mod bundle;
mod error;
mod frame;
pub mod message;
pub mod stream;
pub mod util;

pub use error::Error;
pub use frame::FakeTrapFrame;

pub type Result<T> = core::result::Result<T, Error>;

