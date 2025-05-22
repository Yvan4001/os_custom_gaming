use uart_16550::SerialPort;
use spin::Mutex;
use log::{Record, Level, Metadata, LevelFilter, SetLoggerError};
use core::fmt::Write; // Required for writeln!

// Standard I/O port for COM1 serial port
const SERIAL_IO_PORT: u16 = 0x3F8;

static SERIAL_LOGGER: SerialLogger = SerialLogger {
    port: Mutex::new(None),
    level: LevelFilter::Trace, // Ensure this is verbose
};

struct SerialLogger {
    port: Mutex<Option<SerialPort>>,
    level: LevelFilter,
}

impl SerialLogger {
    // Initializes the serial port. This should be called once.
    // It's unsafe because it involves direct port I/O.
    pub unsafe fn init_port(&self) {
        let mut port = SerialPort::new(SERIAL_IO_PORT);
        port.init();
        *self.port.lock() = Some(port);
    }
}

impl log::Log for SerialLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // MODIFIED: Corrected the nested Option handling
            let mut guard = self.port.lock(); // Lock the Mutex
            if let Some(port) = guard.as_mut() { // Get a mutable reference to SerialPort if Some
                // Using writeln! requires core::fmt::Write
                // Disable interrupts temporarily if writeln! or serial operations are not interrupt-safe
                // x86_64::instructions::interrupts::without_interrupts(|| {
                //     let _ = writeln!(port, "[{}] {}:{} - {}",
                //         record.level(),
                //         record.file().unwrap_or("?"),
                //         record.line().unwrap_or(0),
                //         record.args()
                //     );
                // });
                // For now, assuming writeln! on SerialPort is safe enough for early boot logging
                // without explicit interrupt disabling here. If you get deadlocks or issues
                // during interrupt handling, you might need to make this section interrupt-safe.
                let _ = writeln!(port, "[{}] {}:{} - {}",
                    record.level(),
                    record.module_path().unwrap_or("?"), // Using module_path for better context
                    record.line().unwrap_or(0),
                    record.args()
                );
            } else {
                // Fallback if port not initialized (e.g., print to VGA or do nothing)
                // This case should ideally not happen if logger::init() is called successfully.
            }
        }
    }

    fn flush(&self) {
        // Serial port writes are usually unbuffered at this level,
        // but if there were buffering, it would be flushed here.
    }
}

/// Initializes the serial logger.
/// This function should be called once, very early in the kernel's `_start` or `kernel_main`.
pub fn init() -> Result<(), SetLoggerError> {
    unsafe {
        SERIAL_LOGGER.init_port(); // Initialize the serial port hardware
    }
    log::set_logger(&SERIAL_LOGGER)?;
    log::set_max_level(SERIAL_LOGGER.level); // Set the max level for the log crate
    Ok(())
}

/// Sets the global logging level filter. (Placeholder for advanced usage)
pub fn set_log_level(level: LevelFilter) {
    // For a truly dynamic level change that affects log::set_max_level,
    // this would need to re-call log::set_max_level(level).
    // And SERIAL_LOGGER.level would ideally be an Atomic type for thread-safety
    // if changed after scheduler starts.
    // For now, this function is a placeholder as the primary level is set during init().
    // The `SERIAL_LOGGER.enabled()` method will respect changes to `SERIAL_LOGGER.level`
    // if it were made mutable (e.g. via Mutex or Atomic), but the global `log::max_level`
    // filter is static after `log::set_max_level`.
    log::warn!("set_log_level is mostly a placeholder. Log level primarily set during init. Current SERIAL_LOGGER.level will be used by its enabled() method.");
    // Example if SERIAL_LOGGER.level was, e.g., Mutex<LevelFilter>:
    // *SERIAL_LOGGER.level.lock() = level;
    // log::set_max_level(level); // To update the global filter too
}