use core::fmt::Write;

use log::{Level, LevelFilter, Metadata, Record, set_logger};

use crate::console::CONSOLE;
use crate::smp;

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            smp::no_interrupt(|| {
                let mut lock = m_lock!(CONSOLE);

                writeln!(&mut lock, "[{}:{}] {}", record.level(), record.target(), record.args()).ok();

                if record.metadata().level() <= Level::Error {
                    lock.flush();
                }
            });
        }
    }

    fn flush(&self) {}
}

static LOGGER: SimpleLogger = SimpleLogger;

pub fn register_global_logger() {
    unsafe { log::set_logger_racy(&LOGGER) }.map(|()| log::set_max_level(LevelFilter::Trace));
}