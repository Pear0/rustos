use shim::io;

use crate::shell::{CommandArgs, Shell};
use crate::shell::command::CommandError;
use core::marker::PhantomData;
use core::num::ParseIntError;
use crate::NET;

pub struct MemCmd<R: io::Read, W: io::Write> {
    _r: PhantomData<R>,
    _w: PhantomData<W>,
}

impl<R: io::Read, W: io::Write> MemCmd<R, W> {

    fn parse_num(mut s: &str) -> Result<u64, ParseIntError>  {
        let mut radix = 10;
        if s.starts_with("0x") {
            radix = 16;
            s = &s[2..];
        }
        u64::from_str_radix(s, radix)
    }

    fn read(sh: &mut Shell<R, W>, cmd: &CommandArgs) -> Result<(), CommandError> {
        if cmd.args.len() < 4 {
            writeln!(sh.writer, "not enough arguments")?;
            return Ok(())
        }

        if cmd.args[2] != "u32" {
            writeln!(sh.writer, "unsupported type")?;
            return Ok(())
        }

        let addr = Self::parse_num(cmd.args[3])?;

        let value = unsafe { (addr as *const u32).read_volatile() };

        writeln!(sh.writer, "read: {:#x}", value)?;

        Ok(())
    }

    fn write(sh: &mut Shell<R, W>, cmd: &CommandArgs) -> Result<(), CommandError> {
        if cmd.args.len() < 5 {
            writeln!(sh.writer, "not enough arguments")?;
            return Ok(())
        }

        if cmd.args[2] != "u32" {
            writeln!(sh.writer, "unsupported type")?;
            return Ok(())
        }

        let addr = Self::parse_num(cmd.args[3])?;
        let value = Self::parse_num(cmd.args[4])? as u32;

        unsafe { (addr as *mut u32).write_volatile(value) };

        writeln!(sh.writer, "write: Ok")?;

        Ok(())
    }

    pub fn process(sh: &mut Shell<R, W>, cmd: &CommandArgs) -> Result<(), CommandError> {
        // cmd.args == ["mem", "sub-command", ""]

        if cmd.args.len() < 2 {
            writeln!(sh.writer, "usage: mem <subcommand>")?;
            writeln!(sh.writer, "    mem read u32 <addr>          -  read u32 at address")?;
            writeln!(sh.writer, "    mem write u32 <addr> <data>  -  write u32 at address")?;
            return Ok(())
        }

        match cmd.args[1] {
            "read" => Self::read(sh, cmd),
            "write" => Self::write(sh, cmd),
            c => {
                writeln!(sh.writer, "unknown subcommand: {}", c)?;
                Ok(())
            }
        }
    }

}


