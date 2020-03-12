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

// use std::path::{Path, PathBuf, Component};

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs,
}

/// A structure representing a single shell command.
pub struct Command<'a> {
    pub args: StackVec<'a, &'a str>,
}

impl<'a> Command<'a> {
    /// Parse a command from a string `s` using `buf` as storage for the
    /// arguments.
    ///
    /// # Errors
    ///
    /// If `s` contains no arguments, returns `Error::Empty`. If there are more
    /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
    fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split(' ').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }

        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Command { args })
    }

    /// Returns this command's path. This is equivalent to the first argument.
    fn path(&self) -> &str {
        self.args[0]
    }
}

pub struct Shell<'a, R: io::Read, W: io::Write> {
    pub prefix: &'a str,
    pub cwd: PathBuf,
    pub dead_shell: bool,
    pub reader: R,
    pub writer: W,
    pub commands: HashMap<&'a str, Option<Box<dyn FnMut(&mut Shell<R, W>, &Command) + 'a>>>,
}

type FEntry = fat32::vfat::Entry<crate::fs::PiVFatHandle>;

impl<'a, R: io::Read, W: io::Write> Shell<'a, R, W> {
    pub fn new(prefix: &'static str, reader: R, writer: W) -> Shell<'a, R, W> {
        let mut shell = Shell {
            prefix,
            cwd: PathBuf::from("/"),
            dead_shell: false,
            reader,
            writer,
            commands: HashMap::new(),
        };

        // shell.register("echo", Box)
        shell.register_func("echo", |sh, cmd| {
            for (i, arg) in cmd.args.iter().skip(1).enumerate() {
                if i > 0 {
                    write!(sh.writer, " ");
                }
                write!(sh.writer, "{}", arg);
            }
            writeln!(sh.writer, "");
        });

        shell.register_func("pwd", |sh, _cmd| {
            use alloc::borrow::ToOwned;
            let cwd = sh.cwd_str().to_owned();
            writeln!(&mut sh.writer, "{}", cwd);
        });

        shell.register_func("help", |sh, _cmd| {
            writeln!(&mut sh.writer, "Commands:");
            for (k, _) in sh.commands.iter() {
                writeln!(&mut sh.writer, "{}", *k);
            }
        });

        shell
    }

    pub fn register(&mut self, s: &'a str, func: Box<dyn FnMut(&mut Shell<R, W>, &Command) + 'a>) {
        self.commands.insert(s, Some(func));
    }

    pub fn register_func<T>(&mut self, s: &'a str, func: T) where T: FnMut(&mut Shell<R, W>, &Command) + 'a {
        self.commands.insert(s, Some(Box::new(func)));
    }

    pub fn cwd_str(&self) -> &str {
        self.cwd.to_str().unwrap()
    }

    fn open_file(&self, piece: &str) -> io::Result<FEntry> {
        let path = Path::new(piece);
        if path.has_root() {
            FILESYSTEM.open(path)
        } else {
            FILESYSTEM.open(self.cwd.join(path))
        }
    }

    fn load_process(&self, piece: &str) -> kernel_api::OsResult<Process> {
        let path = Path::new(piece);
        if path.has_root() {
            Process::load(path)
        } else {
            Process::load(self.cwd.join(path))
        }
    }

    fn describe_ls_entry(&mut self, entry: FEntry, show_all: bool) {
        if !show_all && (entry.metadata().hidden() || entry.name() == "." || entry.name() == "..") {
            return;
        }

        let mut line = String::new();
        if entry.is_dir() {
            line.push('d');
        } else if entry.metadata().attributes.volume_id() {
            line.push('V');
        } else {
            line.push('-');
        }

        if entry.metadata().hidden() {
            line.push('h');
        } else {
            line.push('-');
        }
        if entry.metadata().read_only() {
            line.push('r');
        } else {
            line.push('-');
        }

        let size = match &entry {
            fat32::vfat::Entry::<crate::fs::PiVFatHandle>::File(f) => f.size(),
            fat32::vfat::Entry::<crate::fs::PiVFatHandle>::Dir(_) => 0,
        };

        writeln!(self.writer, "{} {:>7} {} {}", line, size, entry.metadata().modified(), entry.name());
    }

    fn process_command(&mut self, command: &mut Command) -> io::Result<()> {
        match command.path() {
            "cat" => {
                if command.args.len() < 2 {
                    writeln!(self.writer, "expected: cat <path>");
                } else {
                    for arg in command.args[1..].iter() {
                        match self.open_file(arg) {
                            Ok(e) => {
                                if let Some(mut f) = e.into_file() {
                                    io::copy(&mut f, &mut self.writer)?;
                                } else {
                                    writeln!(self.writer, "error: not a file");
                                }
                            }
                            Err(e) => {
                                writeln!(self.writer, "error: {}", e);
                            }
                        }
                    }
                }
            }
            "cd" => {
                if command.args.len() < 2 {
                    writeln!(self.writer, "expected: cd <path>");
                } else {
                    for component in Path::new(command.args[1]).components() {
                        match component {
                            Component::Prefix(_) => return ioerr!(InvalidInput, "bad path component"),
                            Component::RootDir => {
                                self.cwd = PathBuf::from("/");
                            }
                            Component::CurDir => {}
                            Component::ParentDir => {
                                self.cwd.pop();
                            }
                            c @ Component::Normal(_) => {
                                let new = self.cwd.join(c);

                                if let fat32::vfat::Entry::Dir(d) = FILESYSTEM.open(new.to_str().unwrap())? {
                                    self.cwd.push(d.name);
                                } else {
                                    writeln!(self.writer, "error: invalid path");
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
            "ls" => {
                let mut dir: &str = self.cwd_str();
                let mut all = false;
                for arg in command.args[1..].iter() {
                    match *arg {
                        "-a" => all = true,
                        other => dir = other,
                    }
                }

                let entry = FILESYSTEM.open(dir)?;

                match &entry {
                    fat32::vfat::Entry::File(_) => self.describe_ls_entry(entry, true),
                    fat32::vfat::Entry::Dir(f) => {
                        let entries = f.entries()?;

                        for entry in entries {
                            self.describe_ls_entry(entry, all);
                        }
                    }
                }
            }
            "uptime" => {
                writeln!(self.writer, "Uptime: {:?}", timer::current_time());
            }
            "reboot" => {
                use pi::pm::reset;

                writeln!(self.writer, "Resetting");
                unsafe { reset(); }
            }
            "pi-info" => {
                use crate::mbox::with_mbox;

                use aarch64::SPSR_EL1;
                writeln!(self.writer, "DAIF: {:04b}", unsafe { SPSR_EL1.get_value(SPSR_EL1::D | SPSR_EL1::A | SPSR_EL1::I | SPSR_EL1::F) });


                with_mbox(|mbox| {
                    writeln!(self.writer, "Serial: {:?}", mbox.serial_number());
                    writeln!(self.writer, "MAC: {:?}", mbox.mac_address());
                    writeln!(self.writer, "Board Revision: {:?}", mbox.board_revision());
                    writeln!(self.writer, "Temp: {:?}", mbox.core_temperature());
                });

                let attrs: Vec<_> = aarch64::attr::iter_enabled().collect();
                writeln!(self.writer, "cpu attrs: {:?}", attrs);



            }
            "arp" => {
                if command.args.len() == 2 {
                    match command.args[1].parse() {
                        Ok(addr) => {
                            match NET.critical(|n| n.arp_request(addr)) {
                                Ok(mac) => {
                                    writeln!(self.writer, "existing entry at {}", mac);
                                }
                                Err(e) => {
                                    writeln!(self.writer, "error: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            writeln!(self.writer, "error: {}", e);
                        }
                    }
                } else {
                    let arp_table = NET.critical(|n| n.arp.copy_table());

                    writeln!(self.writer, "ARP Table:");
                    for entry in arp_table.iter() {
                        writeln!(self.writer, "{:04x} {} -> {}", (entry.0).0, (entry.0).1, entry.1);
                    }
                }
            }
            "panic" => {
                panic!("Oh no, panic!");
            }
            "exit" => {
                self.dead_shell = true;
            }
            "brk" => {
                aarch64::brk!(7);
            }
            "sleep" => {
                if command.args.len() == 2 {
                    let ms: u32 = match command.args[1].parse() {
                        Ok(ms) => ms,
                        Err(e) => {
                            writeln!(self.writer, "error: {}", e);
                            return Ok(());
                        }
                    };

                    let res = kernel_api::syscall::sleep(Duration::from_millis(ms as u64));
                    writeln!(self.writer, "-> {:?}", res);
                } else {
                    writeln!(self.writer, "usage: sleep <ms>");
                }
            }
            "run" => {
                if command.args.len() == 2 {
                    match self.load_process(command.args[1]) {
                        Ok(proc) => {
                            SCHEDULER.add(proc);
                        }
                        Err(e) => {
                            writeln!(self.writer, "error: {:?}", e);
                            return Ok(());
                        }
                    }
                } else {
                    writeln!(self.writer, "usage: run <program>");
                }
            }
            "procs" => {
                let mut snaps = Vec::new();
                SCHEDULER.critical(|p| p.get_process_snaps(&mut snaps));

                for snap in snaps.iter() {
                    writeln!(self.writer, "{:?}", snap);
                }
            }
            "current-el" => {
                let el = unsafe { aarch64::current_el() };
                writeln!(self.writer, "Current EL: {}", el);
            }
            "irqs" => {
                writeln!(self.writer, "System:");
                if let Some(stats) = IRQ.get_stats() {
                    for (i, stat) in stats.iter().enumerate() {
                        writeln!(self.writer, "{:?}: {:?}", Interrupt::from_index(i), stat);
                    }
                } else {
                    writeln!(self.writer, "timed out getting stats");
                }

                for core in 0..smp::MAX_CORES {
                    writeln!(self.writer, "Core {}:", core);
                    if let Some(stats) = IRQ.get_stats_core(core) {
                        for (i, stat) in stats.iter().enumerate() {
                            writeln!(self.writer, "{:?}: {:?}", CoreInterrupt::from_index(i), stat);
                        }
                    } else {
                        writeln!(self.writer, "timed out getting stats");
                    }
                }

            }
            path => {
                if self.commands.contains_key(path) {
                    let mut func = self.commands.get_mut(path).unwrap().take().expect("recursive command call?");

                    func(self, command);

                    // only add back function if new function not added and current function not removed.
                    if let Some(None) = self.commands.get(path) {
                        self.commands.get_mut(path).unwrap().replace(func);
                    }
                } else {
                    writeln!(self.writer, "unknown command: {}", path);
                }
            }
        }

        Ok(())
    }

    pub fn shell_loop(&mut self) {
        writeln!(self.writer);
        while !self.dead_shell {

            let core_id = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) };
            write!(self.writer, "{}{}", core_id, self.prefix);
            let mut raw_buf = [0u8; 512];
            let mut line_buf = StackVec::new(&mut raw_buf);

            'line_loop: loop {
                match self.read_byte() {
                    b'\r' | b'\n' => {
                        writeln!(self.writer);
                        break 'line_loop;
                    }
                    8u8 | 127u8 => {
                        if line_buf.len() > 0 {
                            self.backspace();
                            line_buf.pop();
                        } else {
                            self.bell();
                        }
                    }
                    // if we are in the first or fourth ASCII block and
                    // haven't already handled it, treat this as invalid.
                    byte if byte < 0x20 || byte >= 0x80 => self.bell(),
                    byte => match line_buf.push(byte) {
                        Ok(()) => {
                            self.writer.write(core::slice::from_ref(&byte));
                        }
                        Err(_) => self.bell(),
                    }
                }
            }

            let text_buf = core::str::from_utf8(line_buf.as_slice());
            let mut arg_buf = [""; 64];
            match Command::parse(text_buf.unwrap(), &mut arg_buf) {
                Err(Error::Empty) => {}
                Err(Error::TooManyArgs) => {
                    writeln!(self.writer, "error: too many arguments");
                }
                Ok(mut command) => {
                    if let Err(e) = self.process_command(&mut command) {
                        writeln!(self.writer, "error: {}", e);
                    }
                }
            }
        }
    }

    fn bell(&mut self) {
        write!(self.writer, "\x07");
    }

    fn backspace(&mut self) {
        write!(self.writer, "\x08 \x08");
    }

    fn read_byte(&mut self) -> u8 {

        // let core_id = unsafe { MPIDR_EL1.get_value(MPIDR_EL1::Aff0) };
        //
        // if core_id != 1 {
        //     loop{
        //         aarch64::wfe();
        //     }
        // }

        loop {
            let mut b: u8 = 0;
            if let Ok(n) = self.reader.read(core::slice::from_mut(&mut b)) {
                if n == 1 {
                    return b;
                }
            }
            timer::spin_sleep(Duration::from_millis(1));
        }
    }
}


pub fn serial_shell(prefix: &'static str) -> Shell<ConsoleSync, ConsoleSync> {
    Shell::new(prefix, ConsoleSync::new(), ConsoleSync::new())
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns.
pub fn shell(prefix: &'static str) {
    serial_shell(prefix).shell_loop();
}

