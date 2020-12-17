use alloc::boxed::Box;
use alloc::string::String;
use core::fmt::Write;
use core::time::Duration;

use crossbeam_utils::atomic::AtomicCell;
use dsx::sync::mutex::LockableMutex;
use hashbrown::HashMap;
use log::{Level, LevelFilter, Metadata, Record, set_logger};

use crate::{hw, smp};
use crate::collections::Rcu;
use crate::console::CONSOLE;
use crate::traps::IRQ_RECURSION_DEPTH;

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            smp::no_interrupt(|| {
                if let Some(mut lock) = CONSOLE.lock_timeout(Duration::from_secs(2)) {
                    writeln!(&mut lock, "[{}:{}] {}", record.level(), record.target(), record.args()).ok();

                    if record.metadata().level() <= Level::Error {
                        lock.flush();
                    }
                } else if record.metadata().level() <= Level::Error {
                    let mut uart = hw::arch().early_writer();
                    writeln!(&mut uart, "[RAW-{}:{}] {}", record.level(), record.target(), record.args()).ok();
                }
            });
        }
    }

    fn flush(&self) {}
}

pub struct ModuleLogger {
    module_info: Rcu<Option<Box<HashMap<&'static str, Level>>>>,
    default_level: Level,
}

impl ModuleLogger {
    const fn new() -> Self {
        Self {
            module_info: Rcu::new(None),
            default_level: Level::Info,
        }
    }

    fn update<F: Fn(&mut HashMap<&'static str, Level>)>(&self, func: F) {
        self.module_info.update(|old| {
            let mut new_copy = match old.as_ref() {
                Some(table) => table.clone(),
                None => Box::new(HashMap::new()),
            };

            func(&mut new_copy);

            Some(new_copy)
        });
    }

    fn leak_string(&self, s: &str) -> &'static str {
        let s = Box::leak(Box::new(String::from(s)));
        s.as_str()
    }

    fn level_for_target(&self, target: &str) -> Level {
        self.module_info.critical(|table| {
            match table.as_ref() {
                Some(table) =>
                    table.get(target).cloned().unwrap_or(self.default_level),
                None => self.default_level,
            }
        })
    }

    pub fn set_module_log_level(&self, target: &str, level: Level) {
        self.update(|table| {
            match table.get_mut(unsafe { &core::mem::transmute::<&str, &'static str>(target) }) {
                Some(val) => {
                    *val = level;
                }
                None => {
                    let key = self.leak_string(target);
                    table.insert(key, level);
                }
            }
        })
    }
}

impl log::Log for ModuleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level_for_target(metadata.target())
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            smp::no_interrupt(|| {
                if let Some(mut lock) = CONSOLE.lock_timeout(Duration::from_secs(2)) {
                    writeln!(&mut lock, "[{}:{}] {}", record.level(), record.target(), record.args()).ok();

                    if record.metadata().level() <= Level::Error {
                        lock.flush();
                    }
                } else if record.metadata().level() <= Level::Error {
                    let mut uart = hw::arch().early_writer();
                    writeln!(&mut uart, "[RAW-{}:{}] {}", record.level(), record.target(), record.args()).ok();
                }
            });
        }
    }

    fn flush(&self) {}
}

pub type LoggerType = ModuleLogger;

static LOGGER: LoggerType = LoggerType::new();

pub fn get_logger() -> &'static LoggerType {
    &LOGGER
}

pub fn register_global_logger() {
    unsafe { log::set_logger_racy(&LOGGER) }.map(|()| log::set_max_level(LevelFilter::Trace));
}