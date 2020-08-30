use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::DerefMut;
use core::time::Duration;

use hashbrown::HashMap;

use aarch64::MPIDR_EL1;
pub use command_args::CommandArgs as CommandArgs;
use fat32::traits::{Dir, Entry, File, Metadata};
use fat32::traits::FileSystem;
use pi::interrupt::{CoreInterrupt, Interrupt};
pub use shell::Shell as Shell;
use shim::io;
use shim::ioerr;
use shim::path::{Component, Path, PathBuf};
use stack_vec::StackVec;

use crate::{NET, timer};
use crate::display::Painter;
use crate::display::text::TextPainter;
use crate::display_manager::GlobalDisplay;
use crate::iosync::{ConsoleSync, ReadWrapper, SyncRead, SyncWrite, TeeingWriter, WriteWrapper};
use crate::kernel::KERNEL_IRQ;
use crate::net::arp::ArpResolver;
use crate::process::Process;
use crate::smp;

pub mod command_args;
pub mod command;
mod default_commands;
mod shell;
pub mod shortcut;

// use std::path::{Path, PathBuf, Component};


pub fn serial_shell(prefix: &'static str) -> Shell<ConsoleSync, ConsoleSync> {
    Shell::new(prefix, ConsoleSync::new(), ConsoleSync::new())
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns.
pub fn shell(prefix: &'static str) {
    serial_shell(prefix).shell_loop();
}

