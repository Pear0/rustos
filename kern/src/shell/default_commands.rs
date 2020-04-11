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
use fat32::vfat::{DynVFatHandle, DynWrapper, VFat};
use mountfs::MetaFileSystem;
use mountfs::mount::mfs;
use pi::interrupt::{CoreInterrupt, Interrupt};
use shim::io;
use shim::ioerr;
use shim::path::{Component, Path, PathBuf};
use stack_vec::StackVec;

use crate::{ALLOCATOR, IRQ, NET, SCHEDULER, timer};
use crate::allocator::AllocStats;
use crate::FILESYSTEM;
use crate::fs::sd;
use crate::io::{ConsoleSync, ReadWrapper, SyncRead, SyncWrite, WriteWrapper};
use crate::net::arp::ArpResolver;
use crate::process::Process;
use crate::shell::command::{Command, CommandBuilder};
use crate::smp;

use super::shell::Shell;
use crate::pigrate::bundle::ProcessBundle;

fn describe_ls_entry<W: io::Write, T: mfs::FileInfo>(writer: &mut W, entry: T, show_all: bool) {
    if !show_all && (entry.metadata().hidden == Some(true) || entry.name() == "." || entry.name() == "..") {
        return;
    }

    let mut line = String::new();
    if entry.is_directory() {
        line.push('d');
    } else {
        line.push('-');
    }

    if let Some(true) = entry.metadata().hidden {
        line.push('h');
    } else {
        line.push('-');
    }
    if let Some(true) = entry.metadata().read_only {
        line.push('r');
    } else {
        line.push('-');
    }

    let size = entry.size();

    writeln!(writer, "{} {:>7} {} {}", line, size, entry.metadata().modified.unwrap_or(Default::default()), entry.name());
}

#[derive(Debug, Serialize, Deserialize)]
struct Mascot {
    name: String,
    species: String,
    year_of_birth: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Mascot2 {
    name: String,
    species: String,
    year_of_birth: u32,
    month: u32,
}

pub fn register_commands<R: io::Read, W: io::Write>(sh: &mut Shell<R, W>) {
    sh.command()
        .name("serde")
        .help("")
        .func(|sh, cmd| {
            let ferris = Mascot {
                name: String::from("Ferris"),
                species: String::from("crab"),
                year_of_birth: 2015,
            };

            let serialized = serde_cbor::ser::to_vec(&ferris);
            match serialized {
                Ok(serialized) => {
                    info!("encoded: {}", hex::encode(serialized.as_slice()));

                    let tux: Result<Mascot2, _> = serde_cbor::de::from_slice(serialized.as_ref());

                    match tux {
                        Ok(tux) => {
                            info!("Decoded: {:?}", tux);
                        }
                        Err(e) => {
                            writeln!(sh.writer, "error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    writeln!(sh.writer, "error: {}", e);
                }
            }
        })
        .build();

    sh.command()
        .name("echo")
        .help("print arguments")
        .func(|sh, cmd| {
            for (i, arg) in cmd.args.iter().skip(1).enumerate() {
                if i > 0 {
                    write!(sh.writer, " ");
                }
                write!(sh.writer, "{}", arg);
            }
            writeln!(sh.writer, "");
        })
        .build();

    sh.command()
        .name("pwd")
        .help("print working directory")
        .func(|sh, _cmd| {
            use alloc::borrow::ToOwned;
            let cwd = sh.cwd_str().to_owned();
            writeln!(&mut sh.writer, "{}", cwd);
        })
        .build();

    sh.command()
        .name("alloc-dump")
        .help("print allocator info dump")
        .func(|sh, _cmd| {
            ALLOCATOR.with_internal(|a| {
                a.dump(&mut sh.writer)
            });
            // use alloc::borrow::ToOwned;
            // let cwd = sh.cwd_str().to_owned();
            // writeln!(&mut sh.writer, "{}", cwd);
        })
        .build();

    sh.command()
        .name("runstring")
        .help("run a program from a save string")
        .func_result(|sh, _cmd| {
            writeln!(sh.writer, "Enter a save string:");

            let mut save: Vec<u8> = Vec::new();
            sh.read_line(&mut save);

            writeln!(sh.writer, "Save string len: {}", save.len());

            let decoded = hex::decode(save)?;

            use compression::prelude::*;
            let comp: Vec<_> = decoded.iter().cloned().decode(&mut GZipDecoder::new()).collect::<Result<Vec<_>, _>>()?;

            writeln!(sh.writer, "Uncompressed len: {}", comp.len());

            let bundle: ProcessBundle = serde_cbor::de::from_slice(comp.as_slice())?;

            writeln!(sh.writer, "unbundled process: {}", &bundle.name);

            let proc = Process::from_bundle(&bundle)?;

            proc.dump(&mut sh.writer);

            writeln!(sh.writer, "Launching process...");

            let res = SCHEDULER.add(proc);
            writeln!(sh.writer, "pid: {:?}", res);

            Ok(())
        })
        .build();

    sh.command()
        .name("lsd")
        .help("")
        .func(|sh, cmd| {
            let sd = unsafe { sd::Sd::new() }.expect("failed to init sd card2");
            let vfat = VFat::<DynVFatHandle>::from(sd).expect("failed to init vfat2");

            let mut f = mountfs::fs::FileSystem::new();
            f.mount(PathBuf::from("/"), Box::new(MetaFileSystem::new()));
            f.mount(PathBuf::from("/fat"), Box::new(DynWrapper(vfat)));


            let mut dir: &str = sh.cwd_str();
            let mut all = false;
            for arg in cmd.args[1..].iter() {
                match *arg {
                    "-a" => all = true,
                    other => dir = other,
                }
            }

            match f.open(dir) {
                Ok(entry) => {
                    match &entry {
                        mfs::Entry::File(_) => describe_ls_entry(&mut sh.writer, entry, true),
                        mfs::Entry::Dir(f) => {
                            match f.entries() {
                                Ok(entries) => {
                                    for entry in entries {
                                        describe_ls_entry(&mut sh.writer, entry, all);
                                    }
                                }
                                Err(e) => {
                                    writeln!(sh.writer, "error: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    writeln!(sh.writer, "error: {}", e);
                }
            }
        })
        .build();

    sh.command()
        .name("help")
        .func(|sh, _cmd| {
            writeln!(&mut sh.writer, "Commands:");

            let width = sh.commands.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

            for (k, cmd) in sh.commands.iter() {
                let help = cmd.as_ref()
                    .map(|c| c.help)
                    .filter(|c| !c.is_empty());

                if let Some(help) = help {
                    writeln!(&mut sh.writer, "{:width$} - {}", *k, help, width = width);
                } else {
                    writeln!(&mut sh.writer, "{:width$}", *k, width = width);
                }
            }
        })
        .build();

    for (name, help, val) in [("suspend", "suspend execution of a process", true), ("resume", "resume execution of a process", false)].iter() {
        let (name, help, val) = (*name, *help, *val);
        sh.command()
            .name(name)
            .help(help)
            .func_result(move |sh, cmd| {
                if cmd.args.len() == 2 {
                    let addr: u64 = cmd.args[1].parse()?;
                    SCHEDULER.crit_process(addr, |p| {
                        match p {
                            Some(p) => {
                                p.request_suspend = val;
                                Some(())
                            }
                            None => None
                        }
                    }).ok_or("could not find process")?;
                } else {
                    writeln!(sh.writer, "usage: requires process id");
                }

                Ok(())
            })
            .build();
    }
}

