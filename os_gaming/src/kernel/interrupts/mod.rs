mod idt;
mod handlers;
mod apic;
pub(crate) mod irq;

use x86_64::structures::idt::InterruptDescriptorTable;
use lazy_static::lazy_static;
use spin::Mutex;

pub use self::handlers::*;
pub use self::irq::*;
// Add this to your existing interrupt constants
pub const SOUND_INTERRUPT_INDEX: u8 = 15; // Choose an appropriate interrupt number
pub const KEYBOARD_INTERRUPT_INDEX: u8 = 33; // Choose an appropriate interrupt number
pub const PICS: pic8259::ChainedPics = unsafe { pic8259::ChainedPics::new(32, 40) }; // Primary and secondary PIC offsets

lazy_static! {
    /// The global Interrupt Descriptor Table
    static ref IDT: Mutex<InterruptDescriptorTable> = Mutex::new(InterruptDescriptorTable::new());
}

/// Initialize the interrupt system
/// Initialize the interrupt system
pub fn init() {
    // Initialize the IDT with default handlers
    idt::init();
    
    // Initialize the interrupt controller (PIC or APIC)
    #[cfg(feature = "apic")]
    apic::init();
    
    #[cfg(not(feature = "apic"))]
    irq::pic::init();
    
    // Create a longer-lived guard
    let idt_guard = IDT.lock();
    
    // Load the IDT with the longer-lived guard
    unsafe {
        idt_guard.load_unsafe();
    }
    // The guard is dropped at the end of this function, but that's okay
    // because load_unsafe doesn't require the IDT to stay alive
    
    // Enable interrupts
    unsafe {
        x86_64::instructions::interrupts::enable();
    }
    
    #[cfg(feature = "std")]
    log::info!("Interrupt system initialized");
}

/// Disable interrupts and execute the given function
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R
{
    x86_64::instructions::interrupts::without_interrupts(f)
}

/// Check if interrupts are enabled
pub fn are_enabled() -> bool {
    x86_64::instructions::interrupts::are_enabled()
}

/// Register a custom interrupt handler
pub fn register_handler(
    interrupt: u8, 
    handler: extern "x86-interrupt" fn(x86_64::structures::idt::InterruptStackFrame)
) -> Result<(), &'static str> {
    let mut idt = IDT.lock();
    
    // Ensure we're not overwriting a critical interrupt
    if interrupt < 32 {
        return Err("Cannot override CPU exception handlers");
    }
    
    unsafe {
        idt[interrupt].set_handler_fn(handler);
    }
    
    Ok(())
}

pub fn set_irq_handler(
    interrupt: u8, 
    handler: extern "x86-interrupt" fn(x86_64::structures::idt::InterruptStackFrame)
) -> Result<(), &'static str> {
    let mut idt = IDT.lock();
    
    // Ensure we're not overwriting a critical interrupt
    if interrupt < 32 {
        return Err("Cannot override CPU exception handlers");
    }
    
    unsafe {
        idt[interrupt].set_handler_fn(handler);
    }
    
    Ok(())
}
