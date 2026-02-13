pub use log::*;

pub fn init() {
    // Simple logger init for now, could be replaced by env_logger or similar
    let _ = log::set_logger(&NOOP_LOGGER);
    log::set_max_level(log::LevelFilter::Info);
}

struct NoopLogger;
static NOOP_LOGGER: NoopLogger = NoopLogger;

impl log::Log for NoopLogger {
    fn enabled(&self, _: &Metadata) -> bool { false }
    fn log(&self, _: &Record) {}
    fn flush(&self) {}
}
