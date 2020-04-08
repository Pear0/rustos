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


/// Error type for `Command` parse failures.
#[derive(Debug)]
pub enum Error {
    Empty,
    TooManyArgs,
}

/// A structure representing a single shell command.
pub struct CommandArgs<'a> {
    pub args: StackVec<'a, &'a str>,
}

impl<'a> CommandArgs<'a> {
    /// Parse a command from a string `s` using `buf` as storage for the
    /// arguments.
    ///
    /// # Errors
    ///
    /// If `s` contains no arguments, returns `Error::Empty`. If there are more
    /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
    pub fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<CommandArgs<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split(' ').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }

        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(CommandArgs { args })
    }

    /// Returns this command's path. This is equivalent to the first argument.
    pub fn path(&self) -> &str {
        self.args[0]
    }
}
