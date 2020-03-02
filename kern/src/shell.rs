use shim::io;
use shim::ioerr;
use shim::path::{Path, PathBuf, Component};
// use std::path::{Path, PathBuf, Component};

use alloc::string::String;

use stack_vec::StackVec;
use core::ops::DerefMut;
use core::borrow::Borrow;

use pi::atags::Atags;

use fat32::traits::FileSystem;
use fat32::traits::{Dir, Entry, File, Metadata};

use crate::console::{kprint, kprintln, CONSOLE};
use crate::timer;
use crate::ALLOCATOR;
use crate::FILESYSTEM;

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs,
}

/// A structure representing a single shell command.
struct Command<'a> {
    args: StackVec<'a, &'a str>,
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

struct Shell {
    cwd: PathBuf
}

type FEntry = fat32::vfat::Entry<crate::fs::PiVFatHandle>;

impl Shell {
    pub fn new() -> Shell {
        Shell {
            cwd: PathBuf::from("/"),
        }
    }

    pub fn cwd_str(&self) -> &str {
        self.cwd.to_str().unwrap()
    }

    fn open_file(&self, piece: &str) -> io::Result<FEntry> {
        let mut path = Path::new(piece);
        if path.has_root() {
            FILESYSTEM.open(path)
        } else {
            FILESYSTEM.open(self.cwd.join(path))
        }
    }

    fn describe_ls_entry(&self, entry: FEntry, show_all: bool) {
        if !show_all && (entry.metadata().hidden() || entry.name() == "." || entry.name() == "..") {
            return
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
            fat32::vfat::Entry::<crate::fs::PiVFatHandle>::Dir(d) => 0,
        };

        kprintln!("{} {:>7} {} {}", line, size, entry.metadata().modified(), entry.name());

    }

    pub fn process_command(&mut self, command: &mut Command) -> io::Result<()> {
        match command.path() {
            "echo" => {
                for (i, arg) in command.args.iter().skip(1).enumerate() {
                    if i > 0 {
                        kprint!(" ");
                    }
                    kprint!("{}", arg);
                }
                kprintln!();
            }
            "pwd" => {
                kprintln!("{}", self.cwd_str());
            }
            "cat" => {
                if command.args.len() < 2 {
                    kprintln!("expected: cat <path>");
                } else {
                    for arg in command.args[1..].iter() {
                        match self.open_file(arg) {
                            Ok(e) => {

                                if let Some(mut f) = e.into_file() {
                                    let mut lock = CONSOLE.lock();
                                    io::copy(&mut f, lock.deref_mut())?;
                                } else {
                                    kprintln!("error: not a file");
                                }
                            }
                            Err(e) => {
                                kprintln!("error: {}", e);
                            }
                        }
                    }
                }
            }
            "cd" => {
                if command.args.len() < 2 {
                    kprintln!("expected: cd <path>");
                } else {

                    for component in Path::new(command.args[1]).components() {
                        match component {
                            Component::Prefix(_) => return ioerr!(InvalidInput, "bad path component"),
                            Component::RootDir => {
                                self.cwd = PathBuf::from("/");
                            },
                            Component::CurDir => {},
                            Component::ParentDir => {
                                self.cwd.pop();
                            },
                            c @ Component::Normal(_) => {
                                let new = self.cwd.join(c);

                                if let fat32::vfat::Entry::Dir(d) = FILESYSTEM.open(new.to_str().unwrap())? {
                                    self.cwd.push(d.name);
                                } else {
                                    kprintln!("error: invalid path");
                                    return Ok(())
                                }

                            },
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
                    fat32::vfat::Entry::File(f) => self.describe_ls_entry(entry, true),
                    fat32::vfat::Entry::Dir(f) => {

                        let entries = f.entries()?;

                        for entry in entries {
                            self.describe_ls_entry(entry, all);
                        }
                    }
                }
            }
            "uptime" => {
                kprintln!("Uptime: {:?}", timer::current_time());

            }
            "reboot" => {
                use pi::pm::reset;

                kprintln!("Resetting");
                unsafe { reset(); }

            }
            "pi-info" => {
                use pi::mbox::MBox;
                kprintln!("Serial: {:?}", MBox::serial_number());
                kprintln!("MAC: {:?}", MBox::mac_address());
                kprintln!("Board Revision: {:?}", MBox::board_revision());
            }
            "panic" => {
                panic!("Oh no, panic!");
            }
            path => {
                kprintln!("unknown command: {}", path);
            }
        }

        Ok(())
    }
}

fn bell() {
    CONSOLE.lock().write_byte(7);
}

fn backspace() {
    let mut console = CONSOLE.lock();
    console.write_byte(8);
    console.write_byte(b' ');
    console.write_byte(8);
}

fn read_byte() -> u8 {
    CONSOLE.lock().read_byte()
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns.
pub fn shell(prefix: &str) -> ! {
    let mut shell = Shell::new();
    kprintln!();
    loop {
        kprint!("{}", prefix);
        let mut raw_buf = [0u8; 512];
        let mut line_buf = StackVec::new(&mut raw_buf);

        'line_loop: loop {
            match read_byte() {
                b'\r' | b'\n' => {
                    kprintln!();
                    break 'line_loop;
                }
                8u8 | 127u8 => {
                    if line_buf.len() > 0 {
                        backspace();
                        line_buf.pop();
                    } else {
                        bell();
                    }
                }
                // if we are in the first or fourth ASCII block and
                // haven't already handled it, treat this as invalid.
                byte if byte < 0x20 || byte >= 0x80 => bell(),
                byte => match line_buf.push(byte) {
                    Ok(()) => CONSOLE.lock().write_byte(byte),
                    Err(_) => bell(),
                }
            }
        }

        let text_buf = core::str::from_utf8(line_buf.as_slice());
        let mut arg_buf = [""; 64];
        match Command::parse(text_buf.unwrap(), &mut arg_buf) {
            Err(Error::Empty) => {}
            Err(Error::TooManyArgs) => {
                kprintln!("error: too many arguments");
            }
            Ok(mut command) => {
                if let Err(e) = shell.process_command(&mut command) {
                    kprintln!("error: {}", e);
                }
            }
        }
    }
}

