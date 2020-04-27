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

use crate::{NET, timer};
use crate::FILESYSTEM;
use crate::iosync::{ConsoleSync, ReadWrapper, SyncRead, SyncWrite, WriteWrapper, TeeingWriter};
use crate::kernel::KERNEL_IRQ;
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
use crate::display::text::TextPainter;
use crate::display_manager::GlobalDisplay;
use crate::display::Painter;

// use std::path::{Path, PathBuf, Component};



pub fn serial_shell(prefix: &'static str) -> Shell<ConsoleSync, TeeingWriter<ConsoleSync, TextPainter<GlobalDisplay>>> {

    let p = Painter::new(GlobalDisplay::new());

    let painter = TextPainter::new(p, 86, 116);

    let writer = TeeingWriter::new(ConsoleSync::new(), painter);

    Shell::new(prefix, ConsoleSync::new(), writer)
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns.
pub fn shell(prefix: &'static str) {
    serial_shell(prefix).shell_loop();
}

