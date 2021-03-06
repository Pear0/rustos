use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::min;
use core::ops::DerefMut;
use core::sync::atomic::Ordering;
use core::time::Duration;

use hashbrown::HashMap;
use log::Level;
use xmas_elf::sections::ShType;

use aarch64::{LR, MPIDR_EL1, SP};
use common::fmt::ByteSize;
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

use crate::{ALLOCATOR, BootVariant, FILESYSTEM2, hw, MP_ALLOC, NET, perf, timer, timing};
use crate::allocator::AllocStats;
use crate::arm::PhysicalCounter;
use crate::fs::handle::{Sink, Source};
use crate::fs::sd;
use crate::fs::service::PipeService;
use crate::hyper::HYPER_SCHEDULER;
use crate::iosync::{ConsoleSync, ReadWrapper, SyncRead, SyncWrite, WriteWrapper};
use crate::kernel::{KERNEL_IRQ, KERNEL_CORES};
use crate::kernel::KERNEL_SCHEDULER;
use crate::mutex::Mutex;
use crate::net::arp::ArpResolver;
use crate::perf::PERF_EVENTS_ENABLED;
use crate::pigrate::bundle::ProcessBundle;
use crate::pigrate_server::{pigrate_server, register_pigrate};
use crate::process::{Process, SnapProcess};
use crate::shell::command::{Command, CommandBuilder};
use crate::shell::shortcut::sleep_until_key;
use crate::smp;
use crate::sync::atomic_registry::Registry;
use crate::traps::coreinfo::exc_ratio;
use crate::traps::IRQ_RECURSION_DEPTH;

use super::shell::Shell;
use dsx::collections::spsc_queue::SpscQueue;
use crate::kernel_call::syscall::exec_in_exc;

mod mem;
mod net;

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
        .name("foo")
        .help("random stuff")
        .func(|sh, _cmd| {

            info!("core merry-go-round");
            for core_i in 0..*KERNEL_CORES {
                exec_in_exc(|exc| {
                    info!("[{}] Setting core affinity to {}", smp::core(), core_i);
                    KERNEL_SCHEDULER.crit_process(exc.pid, |proc| {
                        let proc = proc.unwrap();
                        proc.affinity.set_only(core_i);
                    });
                });

                // trigger switch so that we are sent to the new core affinity.
                kernel_api::syscall::sched_yield();

                info!("Hello from core {}", smp::core());

            }
            info!("merry-go-round done");

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

            let res = KERNEL_SCHEDULER.add(proc);
            writeln!(sh.writer, "pid: {:?}", res);

            Ok(())
        })
        .build();

    sh.command()
        .name("connect")
        .help("connect to running process TTYs.")
        .func_result(|sh, cmd| {
            if cmd.args.len() < 2 {
                writeln!(sh.writer, "usage: connect <pid>")?;
                return Ok(());
            }

            let pid: u64 = cmd.args[1].parse()?;

            HYPER_SCHEDULER.crit_process(pid, |proc| {
                if let Some(proc) = proc {
                    proc.detail.serial = Some((Arc::new(Sink::KernSerial), Arc::new(Source::KernSerial)));
                    Ok(())
                } else {
                    Err("pid not found")
                }
            })?;

            sleep_until_key(b'\x03');

            writeln!(sh.writer, "[disconnected]")?;

            HYPER_SCHEDULER.crit_process(pid, |proc| {
                if let Some(proc) = proc {
                    proc.detail.serial = None;
                }
            });

            Ok(())
        })
        .build();

    sh.command()
        .name("verbose")
        .help("")
        .func(|sh, cmd| {
            use core::sync::atomic::Ordering;
            use crate::traps::hyper::*;

            error!("IRQ_DEPTH: {}", IRQ_RECURSION_DEPTH.get());


            VERBOSE_CORE.store(true, Ordering::Relaxed);
        })
        .build();

    sh.command()
        .name("perf-dump")
        .help("Dump current profiling data")
        .func(|_sh, _cmd| {
            perf::dump_events();
        })
        .build();

    sh.command()
        .name("perf")
        .help("Enable/Disable profile streaming")
        .func_result(|sh, cmd| {
            if cmd.args.len() < 2 {
                writeln!(sh.writer, "expected: perf on/off")?;
                return Ok(());
            }

            let new_state = match cmd.args[1] {
                "on" => true,
                "off" => false,
                _ => {
                    writeln!(sh.writer, "unknown argument")?;
                    return Ok(());
                }
            };

            PERF_EVENTS_ENABLED.store(new_state, Ordering::Relaxed);

            Ok(())
        })
        .build();

    sh.command()
        .name("alloc-oom")
        .help("Trigger OOM by causing an allocation using NULL_ALLOC")
        .func(|_sh, _cmd| {
            error!("this allocation should panic...");

            MP_ALLOC.with_allocator(&mpalloc::NULL_ALLOC, || {
                String::from("this allocation should panic")
            });

            error!("no panic ???");
        })
        .build();

    sh.command()
        .name("ls")
        .help("List files")
        .func_result(|sh, cmd| {
            let mut dir: &str = sh.cwd_str();
            let mut all = false;
            for arg in cmd.args[1..].iter() {
                match *arg {
                    "-a" => all = true,
                    other => dir = other,
                }
            }

            let entry = FILESYSTEM2.open(dir)?;
            match &entry {
                mfs::Entry::File(_) => describe_ls_entry(&mut sh.writer, entry, true),
                mfs::Entry::Dir(f) => {
                    let entries = FILESYSTEM2.critical(|fs| fs.entries(f.clone()))?;
                    for entry in entries {
                        describe_ls_entry(&mut sh.writer, entry, all);
                    }
                }
            }

            Ok(())
        })
        .build();

    sh.command()
        .name("lsmnt")
        .help("List mounted filesystems")
        .func_result(|sh, cmd| {
            let mount_info = FILESYSTEM2.critical(|fs| fs.get_mounts());
            writeln!(sh.writer, "Mounts:")?;
            for mount in mount_info.iter() {
                writeln!(sh.writer, "  {:?} - {:?}", mount.path, mount.fs_name)?;
            }

            Ok(())
        })
        .build();

    sh.command()
        .name("mem")
        .help("network stack utilities")
        .func_result(|sh, cmd| mem::MemCmd::process(sh, cmd))
        .build();

    sh.command()
        .name("net")
        .help("network stack utilities")
        .func_result(|sh, cmd| net::NetCmd::process(sh, cmd))
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

    sh.command()
        .name("cores")
        .func_result(|sh, _cmd| {
            writeln!(&mut sh.writer, "Cores:")?;

            let exc_ratios = exc_ratio();

            for i in 0..4 {
                let info = &exc_ratios[i];
                let usage = info.0.get_average();
                writeln!(&mut sh.writer, "Core {}: {}.{}%", i, usage / 10, usage % 10)?;

                for j in 0..min(info.1.len(), 20) {
                    let tuple = info.1[j].1;
                    writeln!(&mut sh.writer, "  {:x?}: {:?} -> {:?}", info.1[j].0, tuple, (tuple.0) / (tuple.1 as u32))?;
                }

                writeln!(&mut sh.writer, "");
            }

            Ok(())
        })
        .build();

    sh.command()
        .name("loadelf")
        .func_result(|sh, cmd| {
            if cmd.args.len() < 2 {
                writeln!(sh.writer, "expected: loadelf <path>")?;
                return Ok(());
            }

            let path = sh.handle_path(cmd.args[1]);

            crate::debug::load_from_file(&path)?;

            Ok(())
        })
        .build();

    sh.command()
        .name("elf")
        .func_result(|sh, _cmd| {
            let debug_info = crate::debug::debug_ref().ok_or("Debug info not loaded")?;

            let mut lr = crate::debug::base_pointer() as u64;

            for _ in 0..20 {
                if lr == 0 {
                    break;
                }

                let addr = unsafe { ((lr + 8) as *const u64).read() };
                lr = unsafe { (lr as *const u64).read() };

                writeln!(&mut sh.writer, "addr: {:#x}, lr: {:#x}", addr, lr)?;

                let mut l = debug_info.context.find_frames(addr)?;

                for i in 0..100 {
                    let elem = match l.next()? {
                        Some(l) => l,
                        None => break,
                    };

                    for _ in 0..i {
                        write!(&mut sh.writer, " ")?;
                    }

                    let name = elem.function.as_ref().map(|x| x.demangle());

                    match elem.location {
                        Some(l) => writeln!(&mut sh.writer, "Loc: {:?} {:?}:{:?} -> {:?}", l.file, l.line, l.column, name)?,
                        None => writeln!(&mut sh.writer, "Loc: None")?,
                    }
                }
            }


            Ok(())
        })
        .build();

    sh.command()
        .name("reg")
        .func_result(|sh, _cmd| {
            let _guard = smp::interrupt_guard();
            use crate::mutex::{KERN_MUTEX_HOOKS, MUTEX_INFOS};

            info!("lock count: {}", KERN_MUTEX_HOOKS.lock_count.load(Ordering::Relaxed));
            info!("lock op count: {}", KERN_MUTEX_HOOKS.lock_op_count.load(Ordering::Relaxed));

            let infos = unsafe { MUTEX_INFOS.as_ref().unwrap() };

            for info in infos.iter() {
                if !info.assigned.load(Ordering::Acquire) {
                    continue;
                }

                let lock_name = unsafe { &*info.lock_name.get() };
                let locker_name = unsafe { &*info.locker_name.get() };

                info!("  lock {:?}:", lock_name);
                info!("      locked by: {:?}", locker_name);
                info!("      locked count: {}", info.lock_op_count.load(Ordering::Relaxed));
            }

            // info!("registry op count: {}, size: {}", MUTEX_REGISTRY.op_count(), MUTEX_REGISTRY.size());
            //
            // for entry in MUTEX_REGISTRY.iter_ref() {
            //     info!("Mutex: {} -> waiting: {:?}", entry.name, timing::cycles_to_time::<PhysicalCounter>(entry.total_waiting_time.load(Ordering::Relaxed)));
            // }

            Ok(())
        })
        .build();

    sh.command()
        .name("dtb")
        .func_result(|sh, _cmd| {
            if let hw::ArchVariant::Khadas(khadas) = hw::arch_variant() {
                let dtb = khadas.dtb_reader()?;
            }


            Ok(())
        })
        .build();

    sh.command()
        .name("logl")
        .func_result(|sh, cmd| {
            if cmd.args.len() < 3 {
                writeln!(sh.writer, "expected: logl <module> <error|warn|info|debug|trace")?;
                return Ok(());
            }

            let level_str = cmd.args[2].to_lowercase();
            let level = match level_str.as_str() {
                "error" => Level::Error,
                "warn" => Level::Warn,
                "info" => Level::Info,
                "debug" => Level::Debug,
                "trace" => Level::Trace,
                _ => Err("unknown log level")?,
            };

            crate::logger::get_logger().set_module_log_level(cmd.args[1], level);

            Ok(())
        })
        .build();

    sh.command()
        .name("proc2")
        .help("print processes in a table")
        .func_result(|sh, cmd| {
            let mut repeat: bool = false;
            if let Some(arg) = cmd.args.get(1) {
                repeat = *arg == "-w";
            }

            if !repeat && cmd.args.len() == 2 {
                let id = cmd.args[1].parse()?;

                if BootVariant::kernel() {
                    KERNEL_SCHEDULER.crit_process(id, |proc| {
                        match proc {
                            Some(proc) => {
                                proc.dump(&mut sh.writer);
                            }
                            None => {
                                writeln!(sh.writer, "no process found for id");
                            }
                        }
                    });
                } else {
                    HYPER_SCHEDULER.crit_process(id, |proc| {
                        match proc {
                            Some(proc) => {
                                proc.dump(&mut sh.writer);
                            }
                            None => {
                                writeln!(sh.writer, "no process found for id");
                            }
                        }
                    });
                }

                return Ok(());
            }

            let cols = [
                String::from("  core"),
                String::from("  pid"), String::from("     state"), String::from("      name"),
                String::from("     cpu time"), String::from("cpu %"), String::from("waiting %"),
                String::from("ready %"), String::from("slice time"), String::from("task switches"),
                String::from("    lr"),
            ].to_vec();

            loop {
                let mut table = shutil::TableWriter::new(&mut sh.writer, 180, cols.clone());
                write!(table.get_writer(), "\x1b[2K")?;
                table.print_header()?;

                let mut snaps: Vec<SnapProcess> = Vec::new();
                if BootVariant::kernel() {
                    KERNEL_SCHEDULER.get_all_process_snaps(&mut snaps);
                } else {
                    HYPER_SCHEDULER.get_process_snaps(&mut snaps);
                }

                snaps.sort_by(|a, b| a.tpidr.cmp(&b.tpidr));

                for snap in snaps.iter() {
                    // write!(table.get_writer(), "\x1b[2K")?;
                    table.print(snap.core)?
                        .print(snap.tpidr)?
                        .print_debug(snap.state)?
                        .print(&snap.name)?
                        .print_debug(snap.cpu_time)?
                        .print(&format_args!("{}.{}%", snap.cpu_usage / 10, snap.cpu_usage % 10))?
                        .print(&format_args!("{}.{}%", snap.waiting_usage / 10, snap.waiting_usage % 10))?
                        .print(&format_args!("{}.{}%", snap.ready_usage / 10, snap.ready_usage % 10))?
                        .print_debug(snap.avg_run_slice)?
                        .print(snap.task_switches)?
                        .print(&format_args!("0x{:x}", snap.lr))?
                        .finish()?;
                }

                if !repeat || sh.cancel_requested() {
                    break;
                }

                kernel_api::syscall::sleep(Duration::from_millis(500));

                if sh.cancel_requested() {
                    break;
                }

                write!(sh.writer, "\x1b[{}F", snaps.len() + 1)?;
            }

            Ok(())
        })
        .build();

    sh.command()
        .name("color-test")
        .help("print small ANSI color grid")
        .func_result(|sh, cmd| {
            for i in 0..11 {
                for j in 0..10 {
                    let n = 10 * i + j;
                    if n > 108 {
                        break;
                    }
                    write!(sh.writer, "\x1b[{}m {:3}\x1b[m", n, n)?;
                }
                writeln!(sh.writer, "")?;
            }


            Ok(())
        })
        .build();

    sh.command()
        .name("kill")
        .func_result(|sh, cmd| {
            if cmd.args.len() < 2 {
                writeln!(sh.writer, "usage: requires process id");
                return Ok(());
            }

            let addr: u64 = cmd.args[1].parse()?;
            KERNEL_SCHEDULER.crit_process(addr, |p| {
                match p {
                    Some(p) => {
                        p.request_kill();
                        Some(())
                    }
                    None => None
                }
            }).ok_or("could not find process")?;

            Ok(())
        })
        .build();

    sh.command()
        .name("pigrate")
        .func(|sh, _cmd| {
            register_pigrate();
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
                    KERNEL_SCHEDULER.crit_process(addr, |p| {
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

