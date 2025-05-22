// Import modules
pub mod cpu;
pub mod memory;
pub mod interrupts;
pub mod drivers;
pub mod boot;

// Re-export important items
pub use cpu::init as cpu_init;
pub use memory::init as memory_init;
pub use interrupts::init as interrupts_init;
use crate::println;
use crate::boot::info::CustomBootInfo as BootInfo;

// Kernel initialization function
pub fn init(boot_info: &'static BootInfo) -> Result<(), &'static str> {
    
    // Initialize cpu
    cpu_init();
    // Initialize memory management subsystem
    memory::init(boot_info)?;
    
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