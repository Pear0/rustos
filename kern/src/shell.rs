use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::DerefMut;
use core::time::Duration;

use hashbrown::HashMap;

use fat32::traits::{Dir, Entry, File, Metadata};
use fat32::traits::FileSystem;
use pi::interrupt::{Interrupt, CoreInterrupt};
use shim::io;
use shim::ioerr;
use shim::path::{Component, Path, PathBuf};
use stack_vec::StackVec;

use crate::{IRQ, NET, SCHEDULER, timer};
use crate::FILESYSTEM;
use crate::io::{ConsoleSync, ReadWrapper, SyncRead, SyncWrite, WriteWrapper};
use crate::net::arp::ArpResolver;
use crate::process::Process;
use aarch64::MPIDR_EL1;
use crate::smp;

pub mod command_args;
pub mod command;
mod default_commands;
mod shell;

pub use command_args::CommandArgs as CommandArgs;
pub use shell::Shell as Shell;

// use std::path::{Path, PathBuf, Component};



pub fn serial_shell(prefix: &'static str) -> Shell<ConsoleSync, ConsoleSync> {
    Shell::new(prefix, ConsoleSync::new(), ConsoleSync::new())
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns.
pub fn shell(prefix: &'static str) {
    serial_shell(prefix).shell_loop();
}

