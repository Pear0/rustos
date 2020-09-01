use core::fmt::Write;
use core::time::Duration;

use log::{Level, LevelFilter, Metadata, Record, set_logger};

use crate::console::CONSOLE;
use crate::{smp, hw};
use crate::traps::IRQ_RECURSION_DEPTH;
use crossbeam_utils::atomic::AtomicCell;
use hashbrown::HashMap;

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

struct ModuleLogger {
    module_info: AtomicCell<Box<HashMap<&'static str, Level>>>
}

impl ModuleLogger {

    fn update<F: Fn(&mut HashMap<&'static str, Level>)>(&self, func: F) {
        self.module_info.load()



        self.f.compare_and_swap()
    }

    fn leak_string(&self, s: &str) -> &'static str {
        let s = Box::leak(Box::new(String::from(s)));
        s.as_str()
    }
}

impl log::Log for ModuleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.target();
        self.module_info.load();

        core::mem::forget()
        core::mem::ManuallyDrop::new()

        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
    }

    fn flush(&self) {}
}


static LOGGER: SimpleLogger = SimpleLogger;

pub fn register_global_logger() {
    unsafe { log::set_logger_racy(&LOGGER) }.map(|()| log::set_max_level(LevelFilter::Trace));
}