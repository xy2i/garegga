//! Log data with various log levels.
//! NIH from log crate:

use core::fmt::{self, write, Write};

use crate::serial::COM1;

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub enum Level {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

pub struct Record<'a> {
    pub line: u32,
    pub file: &'a str,
    pub level: Level,
}

struct Logger;
impl Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        COM1.lock().write_str(s).unwrap();
        Ok(())
    }
}

pub fn _log(args: core::fmt::Arguments, record: Record) {
    let mut logger = Logger {};

    match record.level {
        Level::Trace => logger.write_str("\x1b[1mTRACE"),
        Level::Debug => logger.write_str("\x1b[1;36mDEBUG"),
        Level::Info => logger.write_str("\x1b[1;34mINFO "),
        Level::Warn => logger.write_str("\x1b[1;33mWARN "),
        Level::Error => logger.write_str("\x1b[1;31mERROR"),
    }
    .unwrap();

    logger.write_str("\x1b[1;39m").unwrap();
    write!(&mut logger, " [{}:{}] ", record.file, record.line).unwrap();
    logger.write_str("\x1b[0m").unwrap();

    write(&mut logger, args).unwrap();
    logger.write_str("\n").unwrap();
}

#[macro_export]
macro_rules! log_macro {
    ($level: expr, $($arg:tt)*) => (
        $crate::logger::_log(format_args!($($arg)*),
        crate::logger::Record { line: line!(), file: file!(), level: $level })
    );
}

macro_rules! trace {
    ($($arg:tt)+) => (log_macro!($crate::logger::Level::Trace, $($arg)+))
}
macro_rules! debug {
    ($($arg:tt)+) => (log_macro!($crate::logger::Level::Debug, $($arg)+))
}
macro_rules! log {
    ($($arg:tt)+) => (log_macro!($crate::logger::Level::Info, $($arg)+))
}
macro_rules! warn {
    ($($arg:tt)+) => (log_macro!($crate::logger::Level::Warn, $($arg)+))
}
macro_rules! error {
    ($($arg:tt)+) => (log_macro!($crate::logger::Level::Error, $($arg)+))
}
