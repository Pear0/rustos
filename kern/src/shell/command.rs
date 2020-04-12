use alloc::boxed::Box;

use shim::io;

use crate::shell::CommandArgs;
use crate::shell::Shell;
use core::fmt;
use alloc::string::String;
use core::num::ParseIntError;

#[derive(Debug)]
pub enum CommandError {
    ParseInt(core::num::ParseIntError),
    HexError(hex::FromHexError),
    SerdeCbor(serde_cbor::Error),
    Compression(compression::prelude::CompressionError),
    Os(kernel_api::OsError),
    Str(&'static str),
    Io(io::Error),
}

impl From<core::num::ParseIntError> for CommandError {
    fn from(e: ParseIntError) -> Self {
        CommandError::ParseInt(e)
    }
}

impl From<hex::FromHexError> for CommandError {
    fn from(e: hex::FromHexError) -> Self {
        CommandError::HexError(e)
    }
}

impl From<serde_cbor::Error> for CommandError {
    fn from(e: serde_cbor::Error) -> Self {
        CommandError::SerdeCbor(e)
    }
}

impl From<compression::prelude::CompressionError> for CommandError {
    fn from(e: compression::prelude::CompressionError) -> Self {
        CommandError::Compression(e)
    }
}

impl From<kernel_api::OsError> for CommandError {
    fn from(e: kernel_api::OsError) -> Self {
        CommandError::Os(e)
    }
}

impl From<&'static str> for CommandError {
    fn from(e: &'static str) -> Self {
        CommandError::Str(e)
    }
}

impl From<io::Error> for CommandError {
    fn from(e: io::Error) -> Self {
        CommandError::Io(e)
    }
}

pub struct Command<'a, R: io::Read, W: io::Write> {
    pub name: &'a str,
    pub help: &'a str,
    pub func: Box<dyn FnMut(&mut Shell<R, W>, &CommandArgs) + 'a>,
}

pub struct CommandBuilder<'a, 'b: 'a, R: io::Read, W: io::Write> {
    shell: &'a mut Shell<'b, R, W>,
    name: Option<&'b str>,
    help: Option<&'b str>,
    func: Option<Box<dyn FnMut(&mut Shell<R, W>, &CommandArgs) + 'b>>,
}

impl<'a, 'b: 'a, R: io::Read, W: io::Write> CommandBuilder<'a, 'b, R, W> {
    pub fn new(shell: &'a mut Shell<'b, R, W>) -> Self {
        Self {
            shell,
            name: None,
            help: None,
            func: None,
        }
    }

    pub fn name(mut self, name: &'b str) -> Self {
        self.name.replace(name);
        self
    }

    pub fn help(mut self, help: &'b str) -> Self {
        self.help.replace(help);
        self
    }

    pub fn func<T>(mut self, func: T) -> Self where T: FnMut(&mut Shell<R, W>, &CommandArgs) + 'b {
        self.func.replace(Box::new(func));
        self
    }

    pub fn func_result<T>(mut self, mut func: T) -> Self where T: FnMut(&mut Shell<R, W>, &CommandArgs) -> Result<(), CommandError> + 'b {
        self.func.replace(Box::new(move |sh, cmd| {
            if let Err(e) = func(sh, cmd) {
                kprintln!("error: {:?}", e);
            }
        }));
        self
    }

    pub fn build(self) {
        let name = self.name.expect("name is required");
        let func = self.func.expect("func is required");

        self.shell.commands.insert(name, Some(Command {
            name,
            func,
            help: self.help.unwrap_or(""),
        }));
    }
}

