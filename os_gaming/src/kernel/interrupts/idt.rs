//! Interrupt Descriptor Table setup and management
use core::fmt;
use crate::kernel::interrupts::handlers;
use crate::kernel::interrupts::IDT;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

extern "x86-interrupt" fn default_handler(stack_frame: InterruptStackFrame) {
    panic!("Unhandled interrupt: {:#?}", stack_frame);
}
/// Initialize the Interrupt Descriptor Table with default handler

pub fn init() {
    let mut idt = IDT.lock();
    
    // Set up CPU exception handlers (interrupts 0-31)
    idt.divide_error.set_handler_fn(handlers::divide_error_handler);
    idt.debug.set_handler_fn(handlers::debug_handler);
    idt.non_maskable_interrupt.set_handler_fn(handlers::nmi_handler);
    idt.breakpoint.set_handler_fn(handlers::breakpoint_handler);
    idt.overflow.set_handler_fn(handlers::overflow_handler);
    idt.bound_range_exceeded.set_handler_fn(handlers::bound_range_exceeded_handler);
    idt.invalid_opcode.set_handler_fn(handlers::invalid_opcode_handler);
    idt.device_not_available.set_handler_fn(handlers::device_not_available_handler);
    
    // Double fault handler with separate stack
    unsafe {
        idt.double_fault
            .set_handler_fn(handlers::double_fault_handler)
            .set_stack_index(handlers::DOUBLE_FAULT_IST_INDEX);
    }
    
    idt.invalid_tss.set_handler_fn(handlers::invalid_tss_handler);
    idt.segment_not_present.set_handler_fn(handlers::segment_not_present_handler);
    idt.stack_segment_fault.set_handler_fn(handlers::stack_segment_fault_handler);
    idt.general_protection_fault.set_handler_fn(handlers::general_protection_fault_handler);
    idt.page_fault.set_handler_fn(handlers::page_fault_handler);
    idt.x87_floating_point.set_handler_fn(handlers::floating_point_handler);
    idt.alignment_check.set_handler_fn(handlers::alignment_check_handler);
    idt.machine_check.set_handler_fn(handlers::machine_check_handler);
    idt.simd_floating_point.set_handler_fn(handlers::simd_floating_point_handler);
    idt.virtualization.set_handler_fn(handlers::virtualization_handler);
    idt.security_exception.set_handler_fn(handlers::security_exception_handler);
    
    // Set up the hardware interrupt handlers (32-47)
    idt[32].set_handler_fn(handlers::timer_interrupt_handler);
    idt[33].set_handler_fn(handlers::keyboard_interrupt_handler);
    
    // COM ports (useful for debugging)
    idt[36].set_handler_fn(handlers::com1_interrupt_handler);
    idt[37].set_handler_fn(handlers::com2_interrupt_handler);
    
    // Add more hardware interrupts as needed for your gaming OS
    idt[41].set_handler_fn(handlers::sound_interrupt_handler);
    idt[42].set_handler_fn(handlers::gpu_interrupt_handler);
    idt[43].set_handler_fn(handlers::usb_interrupt_handler);
    
    // Gaming-specific interrupts (custom IRQs)
    idt[50].set_handler_fn(handlers::gamepad_interrupt_handler);
    idt[51].set_handler_fn(handlers::network_gaming_interrupt_handler);
    
    // System call interrupt (typically int 0x80 for compatibility)
    idt[0x80].set_handler_fn(handlers::syscall_handler);
}
