use shim::io;

use core::fmt::Write;
use alloc::string::String;

use crate::shell::{CommandArgs, Shell};
use crate::shell::command::CommandError;
use core::marker::PhantomData;
use crate::NET;

pub struct NetCmd<R: io::Read, W: io::Write> {
    _r: PhantomData<R>,
    _w: PhantomData<W>,
}

impl<R: io::Read, W: io::Write> NetCmd<R, W> {

    fn tcp(sh: &mut Shell<R, W>, _cmd: &CommandArgs) -> Result<(), CommandError> {
        let data = NET.critical(|net| net.tcp.print_info());
        write!(sh.writer, "{}", data)?;
        Ok(())
    }

    fn phy(sh: &mut Shell<R, W>, _cmd: &CommandArgs) -> Result<(), CommandError> {
        let mut data = String::new();
        data.reserve(10000);
        if let Err(_) = NET.critical(|net| net.usb.debug_dump(&mut data)) {
            data.clear();
            writeln!(&mut data, "error during printing")?;
        }
        write!(sh.writer, "{}", data)?;
        Ok(())
    }

    pub fn process(sh: &mut Shell<R, W>, cmd: &CommandArgs) -> Result<(), CommandError> {
        // cmd.args == ["net", "sub-command", ""]

        if cmd.args.len() < 2 {
            writeln!(sh.writer, "usage: net <subcommand>")?;
            writeln!(sh.writer, "    net tcp -  details of active tcp connections")?;
            writeln!(sh.writer, "    net phy -  details about phy")?;
            return Ok(())
        }

        match cmd.args[1] {
            "tcp" => Self::tcp(sh, cmd),
            "phy" => Self::phy(sh, cmd),
            c => {
                writeln!(sh.writer, "unknown subcommand: {}", c)?;
                Ok(())
            }
        }
    }

}


