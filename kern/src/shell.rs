use shim::io;
use shim::path::{Path, PathBuf};

use stack_vec::StackVec;

use pi::atags::Atags;

use fat32::traits::FileSystem;
use fat32::traits::{Dir, Entry};

use crate::console::{kprint, kprintln, CONSOLE};
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
            Ok(mut command) => process_command(&mut command)
        }
    }
}

fn process_command(command: &mut Command) {
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
        "ls" => {
            if command.args.len() == 1 {
                kprintln!("gimme a path");
            } else {
                let dir = command.args[1];

                let entry = FILESYSTEM.open("/").expect("could not open");

                match entry {
                    fat32::vfat::Entry::File(f) => kprintln!("{:?}", f),
                    fat32::vfat::Entry::Dir(f) => {
                        kprintln!("{:?}", f);

                        let entries = f.entries();

                        kprintln!("got entries");

                        let entries = entries.expect("could not list");

                        kprintln!("unwrapped entries");

                        for entry in entries {
                            kprintln!("{:?}", entry);
                        }

                    },
                }

            }

        }

        path => {
            kprintln!("unknown command: {}", path);
        }
    }
}

