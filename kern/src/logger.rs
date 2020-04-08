
use log::{Record, Level, Metadata, set_logger, LevelFilter};

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            kprintln!("[{}:{}] {}", record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {}
}

static LOGGER: SimpleLogger = SimpleLogger;

pub fn register_global_logger() {
    unsafe { log::set_logger_racy(&LOGGER) }.map(|()| log::set_max_level(LevelFilter::Trace));
}