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
    Str(&'static str),
}

impl From<core::num::ParseIntError> for CommandError {
    fn from(e: ParseIntError) -> Self {
        CommandError::ParseInt(e)
    }
}

impl From<&'static str> for CommandError {
    fn from(e: &'static str) -> Self {
        CommandError::Str(e)
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

