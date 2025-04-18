use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};
use super::{Level, Log, Metadata, Record};

/// A logger that outputs to a serial port
pub struct SerialLogger {
    initialized: AtomicBool,
}

impl SerialLogger {
    /// Creates a new serial logger
    pub const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
        }
    }

    /// Initializes the serial port
    pub fn init(&self) {
        if !self.initialized.load(Ordering::SeqCst) {
            // In a real implementation, you would initialize your serial port here
            // For example:
            // unsafe {
            //     // Initialize UART
            //     // Set baud rate, etc.
            // }
            self.initialized.store(true, Ordering::SeqCst);
        }
    }

    /// Writes a string to the serial port
    fn write_str(&self, s: &str) {
        if self.initialized.load(Ordering::SeqCst) {
            // In a real implementation, you would write to your serial port here
            // For example:
            // for &byte in s.as_bytes() {
            //     unsafe {
            //         // Wait for UART to be ready
            //         while !UART.is_ready() {}
            //         // Write byte
            //         UART.write_byte(byte);
            //     }
            // }
        }
    }
}

impl Log for SerialLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut buf = [0u8; 1024];
            let mut writer = fmt::Formatter::new(&mut buf[..]);
            let _ = write!(
                writer,
                "[{}] {}: {}\n",
                record.metadata().level().as_str(),
                record.metadata().target(),
                record.args()
            );
            if let Some(file) = record.file() {
                if let Some(line) = record.line() {
                    let _ = write!(writer, "    at {}:{}", file, line);
                }
            }
            self.write_str(unsafe { core::str::from_utf8_unchecked(&buf[..writer.len()]) });
        }
    }

    fn flush(&self) {
        // Nothing to flush in this simple implementation
    }
} 