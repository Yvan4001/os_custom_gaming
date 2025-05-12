// Import modules
pub mod cpu;
pub mod memory;
pub mod interrupts;
pub mod drivers;
pub mod boot;

use bootloader::BootInfo;
// Re-export important items
pub use cpu::init as cpu_init;
pub use memory::memory_init;
pub use interrupts::init as interrupts_init;
use crate::println;

// Kernel initialization function
pub fn init(boot_info: &'static BootInfo) -> Result<(), &'static str> {
    
    // Initialize cpu
    cpu_init();
    // Initialize memory management subsystem
    memory::memory_init(boot_info)?;
    
    // Interrupt Init
    interrupts::init();
    
    // Initialize driver
    drivers::init();
    
    println!("Kernel initialized successfully!");

    Ok(())
}


// Kernel panic handler
#[cfg(not(test))]
pub fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("Kernel panic: {}", info);
    loop {}
}