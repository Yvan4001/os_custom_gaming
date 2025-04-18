#![no_std]
#![cfg_attr(feature = "std", allow(unused_imports))]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::sync::atomic::{AtomicUsize, Ordering};

mod serial;
pub use serial::SerialLogger;

/// The level of a log message
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Level {
    /// An error that should be investigated
    Error,
    /// A warning that should be noted
    Warn,
    /// Information of general interest
    Info,
    /// Information useful for debugging
    Debug,
    /// Information useful for tracing
    Trace,
}

impl Level {
    /// Returns the string representation of the level
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }
}

/// A trait for logging implementations
pub trait Log: Send + Sync {
    /// Determines if a log message with the specified level would be logged
    fn enabled(&self, metadata: &Metadata) -> bool;

    /// Logs the `Record`
    fn log(&self, record: &Record);

    /// Flushes any buffered records
    fn flush(&self);
}

/// Metadata about a log message
pub struct Metadata<'a> {
    level: Level,
    target: &'a str,
}

impl<'a> Metadata<'a> {
    /// Creates a new metadata instance
    pub const fn new(level: Level, target: &'a str) -> Self {
        Self { level, target }
    }

    /// Returns the level of the log message
    pub fn level(&self) -> Level {
        self.level
    }

    /// Returns the target of the log message
    pub fn target(&self) -> &str {
        self.target
    }
}

/// A log record
pub struct Record<'a> {
    metadata: Metadata<'a>,
    args: fmt::Arguments<'a>,
    module_path: Option<&'a str>,
    file: Option<&'a str>,
    line: Option<u32>,
}

impl<'a> Record<'a> {
    /// Creates a new record
    pub fn new(
        metadata: &'a Metadata<'a>,
        args: fmt::Arguments<'a>,
        module_path: Option<&'a str>,
        file: Option<&'a str>,
        line: Option<u32>,
    ) -> Self {
        Self {
            metadata: *metadata,
            args,
            module_path,
            file,
            line,
        }
    }

    /// Returns the metadata of the record
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Returns the arguments of the record
    pub fn args(&self) -> &fmt::Arguments {
        &self.args
    }

    /// Returns the module path of the record
    pub fn module_path(&self) -> Option<&str> {
        self.module_path
    }

    /// Returns the file of the record
    pub fn file(&self) -> Option<&str> {
        self.file
    }

    /// Returns the line of the record
    pub fn line(&self) -> Option<u32> {
        self.line
    }
}

/// A static logger instance
static LOGGER: StaticLogger = StaticLogger::new();

/// A static logger implementation
struct StaticLogger {
    level: AtomicUsize,
}

impl StaticLogger {
    /// Creates a new static logger
    pub const fn new() -> Self {
        Self {
            level: AtomicUsize::new(Level::Info as usize),
        }
    }

    /// Sets the maximum log level
    pub fn set_max_level(&self, level: Level) {
        self.level.store(level as usize, Ordering::SeqCst);
    }

    /// Gets the maximum log level
    pub fn max_level(&self) -> Level {
        match self.level.load(Ordering::SeqCst) {
            0 => Level::Error,
            1 => Level::Warn,
            2 => Level::Info,
            3 => Level::Debug,
            4 => Level::Trace,
            _ => Level::Error,
        }
    }
}

impl Log for StaticLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() as usize <= self.level.load(Ordering::SeqCst)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // For now, we'll just use a simple format
            // In a real implementation, you would send this to a serial port or other output
            let _ = fmt::format(format_args!(
                "[{}] {}: {}\n",
                record.metadata().level().as_str(),
                record.metadata().target(),
                record.args()
            ));
        }
    }

    fn flush(&self) {
        // Nothing to flush in this simple implementation
    }
}

/// Sets the global logger
pub fn set_logger(logger: &'static dyn Log) -> Result<(), SetLoggerError> {
    // In a real implementation, you would store the logger in a static
    // For now, we'll just use our static logger
    Ok(())
}

/// A type for errors that can occur when setting the logger
pub struct SetLoggerError(());

impl fmt::Debug for SetLoggerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetLoggerError").finish()
    }
}

impl fmt::Display for SetLoggerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("failed to set logger")
    }
}

/// Macro for logging at the error level
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ({
        let level = $crate::Level::Error;
        if $crate::LOGGER.enabled(&$crate::Metadata::new(level, module_path!())) {
            $crate::LOGGER.log(&$crate::Record::new(
                &$crate::Metadata::new(level, module_path!()),
                format_args!($($arg)*),
                Some(module_path!()),
                Some(file!()),
                Some(line!()),
            ));
        }
    })
}

/// Macro for logging at the warn level
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ({
        let level = $crate::Level::Warn;
        if $crate::LOGGER.enabled(&$crate::Metadata::new(level, module_path!())) {
            $crate::LOGGER.log(&$crate::Record::new(
                &$crate::Metadata::new(level, module_path!()),
                format_args!($($arg)*),
                Some(module_path!()),
                Some(file!()),
                Some(line!()),
            ));
        }
    })
}

/// Macro for logging at the info level
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ({
        let level = $crate::Level::Info;
        if $crate::LOGGER.enabled(&$crate::Metadata::new(level, module_path!())) {
            $crate::LOGGER.log(&$crate::Record::new(
                &$crate::Metadata::new(level, module_path!()),
                format_args!($($arg)*),
                Some(module_path!()),
                Some(file!()),
                Some(line!()),
            ));
        }
    })
}

/// Macro for logging at the debug level
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ({
        let level = $crate::Level::Debug;
        if $crate::LOGGER.enabled(&$crate::Metadata::new(level, module_path!())) {
            $crate::LOGGER.log(&$crate::Record::new(
                &$crate::Metadata::new(level, module_path!()),
                format_args!($($arg)*),
                Some(module_path!()),
                Some(file!()),
                Some(line!()),
            ));
        }
    })
}

/// Macro for logging at the trace level
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => ({
        let level = $crate::Level::Trace;
        if $crate::LOGGER.enabled(&$crate::Metadata::new(level, module_path!())) {
            $crate::LOGGER.log(&$crate::Record::new(
                &$crate::Metadata::new(level, module_path!()),
                format_args!($($arg)*),
                Some(module_path!()),
                Some(file!()),
                Some(line!()),
            ));
        }
    })
} 