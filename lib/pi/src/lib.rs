#![feature(core_intrinsics)]
#![feature(const_fn)]
#![feature(asm)]
#![feature(decl_macro)]
#![feature(never_type)]
#![no_std]

#[macro_use]
extern crate aarch64;

pub mod atags;
pub mod common;
pub mod dma;
pub mod gpio;
pub mod interrupt;
pub mod mbox;
pub mod pm;
pub mod timer;
pub mod types;
pub mod uart;
pub mod usb;
