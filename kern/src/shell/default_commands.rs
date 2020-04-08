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

use crate::{IRQ, NET, SCHEDULER, timer, ALLOCATOR};
use crate::FILESYSTEM;
use crate::io::{ConsoleSync, ReadWrapper, SyncRead, SyncWrite, WriteWrapper};
use crate::net::arp::ArpResolver;
use crate::process::Process;
use crate::shell::command::{Command, CommandBuilder};
use crate::smp;

use super::shell::Shell;
use crate::allocator::AllocStats;

pub fn register_commands<R: io::Read, W: io::Write>(sh: &mut Shell<R, W>) {

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

