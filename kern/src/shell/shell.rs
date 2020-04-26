use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::DerefMut;
use core::time::Duration;

use hashbrown::HashMap;

use aarch64::MPIDR_EL1;
use fat32::traits::{Dir, Entry, File, Metadata};
use fat32::traits::FileSystem;
use pi::interrupt::{CoreInterrupt, Interrupt};
use shim::io;
use shim::ioerr;
use shim::path::{Component, Path, PathBuf};
use stack_vec::StackVec;

use crate::{IRQ, NET, KERNEL_SCHEDULER, timer, hw};
use crate::FILESYSTEM;
use crate::iosync::{ConsoleSync, ReadWrapper, SyncRead, SyncWrite, WriteWrapper};
use crate::net::arp::ArpResolver;
use crate::process::{Process, Id, KernelProcess};
use crate::shell::command::{Command, CommandBuilder};
use crate::smp;

use super::command_args::{CommandArgs, Error};
use super::default_commands;
use pi::atags::{Atag, Atags};

pub struct Shell<'a, R: io::Read, W: io::Write> {
    pub prefix: &'a str,
    pub cwd: PathBuf,
    pub dead_shell: bool,
    pub reader: R,
    pub writer: W,
    pub commands: HashMap<&'a str, Option<Command<'a, R, W>>>,
    buffered_byte: Option<u8>,
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
            buffered_byte: None,
        };

        default_commands::register_commands(&mut shell);

        shell
    }

    pub fn command<'c>(&'c mut self) -> CommandBuilder<'c, 'a, R, W> {
        CommandBuilder::new(self)
    }

    // pub fn register(&mut self, s: &'a str, func: Box<dyn FnMut(&mut Shell<R, W>, &CommandArgs) + 'a>) {
    //     self.commands.insert(s, Some(func));
    // }

    // pub fn register_func<T>(&mut self, s: &'a str, func: T) where T: FnMut(&mut Shell<R, W>, &CommandArgs) + 'a {
    //     self.commands.insert(s, Some(Box::new(func)));
    // }

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

    fn load_process(&self, piece: &str) -> kernel_api::OsResult<KernelProcess> {
        let path = Path::new(piece);
        if path.has_root() {
            KernelProcess::load(path)
        } else {
            KernelProcess::load(self.cwd.join(path))
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

    fn process_command(&mut self, command: &mut CommandArgs) -> io::Result<()> {
        match command.path() {
            "cat" => {
                if command.args.len() < 2 {
                    writeln!(self.writer, "expected: cat <path>")?;
                } else {
                    for arg in command.args[1..].iter() {
                        match self.open_file(arg) {
                            Ok(e) => {
                                if let Some(mut f) = e.into_file() {
                                    io::copy(&mut f, &mut self.writer)?;
                                } else {
                                    writeln!(self.writer, "error: not a file")?;
                                }
                            }
                            Err(e) => {
                                writeln!(self.writer, "error: {}", e)?;
                            }
                        }
                    }
                }
            }
            "cd" => {
                if command.args.len() < 2 {
                    writeln!(self.writer, "expected: cd <path>")?;
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
                                    writeln!(self.writer, "error: invalid path")?;
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
            "ls" => {
                // debug!("A");
                let mut dir: &str = self.cwd_str();
                // debug!("B");
                let mut all = false;
                for arg in command.args[1..].iter() {
                    match *arg {
                        "-a" => all = true,
                        other => dir = other,
                    }
                }

                // debug!("C");

                let entry = FILESYSTEM.open(dir)?;

                // debug!("D");

                match &entry {
                    fat32::vfat::Entry::File(_) => self.describe_ls_entry(entry, true),
                    fat32::vfat::Entry::Dir(f) => {
                        let entries = f.entries()?;
                        // debug!("E");

                        for entry in entries {
                            self.describe_ls_entry(entry, all);
                        }
                    }
                }
            }
            "uptime" => {
                writeln!(self.writer, "Uptime: {:?}", timer::current_time())?;
            }
            "reboot" => {
                use pi::pm::reset;

                writeln!(self.writer, "Resetting")?;
                unsafe { reset(); }
            }
            "pi-info" => {
                use crate::mbox::with_mbox;

                use aarch64::SPSR_EL1;
                writeln!(self.writer, "DAIF: {:04b}", unsafe { SPSR_EL1.get_value(SPSR_EL1::D | SPSR_EL1::A | SPSR_EL1::I | SPSR_EL1::F) })?;

                writeln!(self.writer, "CPU implementor: {:?}", aarch64::Implementor::hardware())?;

                writeln!(self.writer, "MIDR_EL1: {:#x}", unsafe { aarch64::MIDR_EL1.get() })?;

                with_mbox(|mbox| {
                    writeln!(self.writer, "Serial: {:x?}", mbox.serial_number());
                    writeln!(self.writer, "MAC: {:x?}", mbox.mac_address());
                    writeln!(self.writer, "Board Revision: {:x?}", mbox.board_revision());
                    writeln!(self.writer, "Temp: {:?}", mbox.core_temperature());
                });

                let attrs: Vec<_> = aarch64::attr::iter_enabled().collect();
                writeln!(self.writer, "cpu attrs: {:?}", attrs)?;

                writeln!(self.writer, "Atags:")?;
                for atag in Atags::get() {
                    writeln!(self.writer, "{:?}", atag)?;
                }

                writeln!(self.writer, "Is QEMU: {}", hw::is_qemu())?;


            }
            "arp" => {
                if command.args.len() == 2 {
                    match command.args[1].parse() {
                        Ok(addr) => {
                            match NET.critical(|n| n.arp_request(addr)) {
                                Ok(mac) => {
                                    writeln!(self.writer, "existing entry at {}", mac)?;
                                }
                                Err(e) => {
                                    writeln!(self.writer, "error: {:?}", e)?;
                                }
                            }
                        }
                        Err(e) => {
                            writeln!(self.writer, "error: {}", e)?;
                        }
                    }
                } else {
                    let arp_table = NET.critical(|n| n.arp.copy_table());

                    writeln!(self.writer, "ARP Table:")?;
                    for entry in arp_table.iter() {
                        writeln!(self.writer, "{:04x} {} -> {}", (entry.0).0, (entry.0).1, entry.1)?;
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
                            writeln!(self.writer, "error: {}", e)?;
                            return Ok(());
                        }
                    };

                    let res = kernel_api::syscall::sleep(Duration::from_millis(ms as u64));
                    writeln!(self.writer, "-> {:?}", res)?;
                } else {
                    writeln!(self.writer, "usage: sleep <ms>")?;
                }
            }
            "run" => {
                if command.args.len() == 2 {
                    match self.load_process(command.args[1]) {
                        Ok(proc) => {
                            let id = KERNEL_SCHEDULER.add(proc);

                            if let Some(id) = id {
                                kernel_api::syscall::waitpid(id);
                            } else {
                                writeln!(self.writer, "scheduler: failed to start process")?;
                            }

                        }
                        Err(e) => {
                            writeln!(self.writer, "error: {:?}", e)?;
                            return Ok(());
                        }
                    }
                } else {
                    writeln!(self.writer, "usage: run <program>")?;
                }
            }
            "runb" => {
                if command.args.len() == 2 {
                    match self.load_process(command.args[1]) {
                        Ok(proc) => {
                            KERNEL_SCHEDULER.add(proc);
                        }
                        Err(e) => {
                            writeln!(self.writer, "error: {:?}", e)?;
                            return Ok(());
                        }
                    }
                } else {
                    writeln!(self.writer, "usage: run <program>")?;
                }
            }
            "procs" => {

                if command.args.len() == 2 {

                    match command.args[1].parse() {
                        Ok(id) => {
                            KERNEL_SCHEDULER.crit_process(id, |proc| {
                                match proc {
                                    Some(proc) => {
                                        proc.dump(&mut self.writer);

                                        match proc.to_bundle() {
                                            Ok(bundle) => {

                                                match serde_cbor::ser::to_vec(&bundle) {
                                                    Ok(bundle) => {

                                                        writeln!(self.writer, "Serialized bundle size: {}", bundle.len());

                                                        use compression::prelude::*;

                                                        let comp: Result<Vec<_>, _> = bundle.iter().cloned().encode(&mut GZipEncoder::new(), Action::Finish).collect();

                                                        match comp {
                                                            Ok(comp) => {
                                                                writeln!(self.writer, "Compressed size: {}", comp.len());
                                                                writeln!(self.writer, "hex: {}", hex::encode(comp.as_slice()));
                                                            }
                                                            Err(e) => {
                                                                writeln!(self.writer, "compression error: {:?}", e);
                                                            }
                                                        }

                                                    }
                                                    Err(e) => {
                                                        writeln!(self.writer, "serialization error: {:?}", e);
                                                    }
                                                }

                                            }
                                            Err(e) => {
                                                writeln!(self.writer, "bundle error: {:?}", e);
                                            }
                                        }

                                    }
                                    None => {
                                        writeln!(self.writer, "no process found for id");
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            writeln!(self.writer, "error: {:?}", e)?;
                        }
                    }

                } else {
                    let mut snaps = Vec::new();
                    KERNEL_SCHEDULER.critical(|p| p.get_process_snaps(&mut snaps));

                    for snap in snaps.iter() {
                        writeln!(self.writer, "{:?}", snap)?;
                    }
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
                        writeln!(self.writer, "{:>6?}: {:?}", Interrupt::from_index(i), stat);
                    }
                } else {
                    writeln!(self.writer, "timed out getting stats");
                }

                for core in 0..smp::MAX_CORES {
                    writeln!(self.writer, "Core {}:", core);
                    if let Some(stats) = IRQ.get_stats_core(core) {
                        for (i, stat) in stats.iter().enumerate() {
                            writeln!(self.writer, "{:>14?}: {:?}", CoreInterrupt::from_index(i), stat);
                        }
                    } else {
                        writeln!(self.writer, "timed out getting stats");
                    }
                }
            }
            path => {
                if self.commands.contains_key(path) {
                    let mut com = self.commands.get_mut(path).unwrap().take().expect("recursive command call?");

                    (com.func)(self, command);

                    // only add back function if new function not added and current function not removed.
                    if let Some(None) = self.commands.get(path) {
                        self.commands.get_mut(path).unwrap().replace(com);
                    }
                } else {
                    writeln!(self.writer, "unknown command: {}", path);
                }
            }
        }

        Ok(())
    }

    pub fn cancel_requested(&mut self) -> bool {
        while self.has_byte() {
            if self.read_byte() == b'\x03' {
                return true;
            }
        }
        false
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
                    b'\x12' => { // Ctrl-R  -> trigger reset
                        use pi::pm::reset;
                        writeln!(self.writer, "Resetting");
                        unsafe { reset(); }
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
                    byte if byte < 0x20 || byte >= 0x80 => {
                        write!(self.writer, "\\x{:02x}", byte);
                        self.bell()
                    },
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
            match CommandArgs::parse(text_buf.unwrap(), &mut arg_buf) {
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

    pub fn read_line(&mut self, line: &mut Vec<u8>) -> io::Result<usize> {
        let mut size = 0;

        'line_loop: loop {
            match self.read_byte() {
                b'\r' | b'\n' => {
                    writeln!(self.writer);
                    break 'line_loop;
                }
                8u8 | 127u8 => {
                    if size > 0 {
                        self.backspace();
                        line.pop();
                        size -= 1;
                    } else {
                        self.bell();
                    }
                }
                // if we are in the first or fourth ASCII block and
                // haven't already handled it, treat this as invalid.
                byte if byte < 0x20 || byte >= 0x80 => self.bell(),
                byte => {
                    line.push(byte);
                    size += 1;
                    self.writer.write(core::slice::from_ref(&byte));
                }
            }
        }

        Ok(size)
    }

    pub fn bell(&mut self) {
        write!(self.writer, "\x07");
    }

    fn backspace(&mut self) {
        write!(self.writer, "\x08 \x08");
    }

    fn has_byte(&mut self) -> bool {
        if self.buffered_byte.is_some() {
            return true;
        }

        let mut b: u8 = 0;
        if let Ok(n) = self.reader.read(core::slice::from_mut(&mut b)) {
            if n == 1 {
                self.buffered_byte = Some(b);
                return true;
            }
        }

        return false;
    }

    fn read_byte(&mut self) -> u8 {
        if let Some(b) = self.buffered_byte.take() {
            return b;
        }

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

