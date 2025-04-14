// Import modules
pub mod cpu;
pub mod memory;
pub mod interrupts;
pub mod drivers;
pub mod boot;

// Re-export important items
pub use cpu::init as cpu_init;
pub use memory::memory_init;
pub use interrupts::init as interrupts_init;

// Kernel initialization function
pub fn init() {
    // Initialize kernel components
    cpu_init();
    memory_init(0);
    interrupts_init();
    
    // Initialize drivers
    drivers::init();
    
    println!("Kernel initialized successfully");
}

// Kernel panic handler
#[cfg(not(test))]
pub fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("Kernel panic: {}", info);
    loop {}
}