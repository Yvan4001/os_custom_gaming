//! Interrupt handlers for various interrupts

use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};
use crate::kernel::drivers::keyboard;
use crate::kernel::drivers::sound;
use crate::kernel::drivers::gamepad;
use crate::kernel::drivers::network;
use crate::kernel::drivers::timer as time;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0x20; // IST index for double fault stack

// CPU Exception Handlers
pub extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: DIVIDE BY ZERO\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    #[cfg(feature = "std")]
    log::debug!("EXCEPTION: DEBUG\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn nmi_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: NON-MASKABLE INTERRUPT\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    #[cfg(feature = "std")]
    log::debug!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: OVERFLOW\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: BOUND RANGE EXCEEDED\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: INVALID OPCODE\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: DEVICE NOT AVAILABLE\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!(
        "EXCEPTION: DOUBLE FAULT (error code: {})\n{:#?}",
        error_code, stack_frame
    );
}

pub extern "x86-interrupt" fn invalid_tss_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: INVALID TSS (error code: {})\n{:#?}",
        error_code, stack_frame
    );
}

pub extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: SEGMENT NOT PRESENT (error code: {})\n{:#?}",
        error_code, stack_frame
    );
}

pub extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: STACK SEGMENT FAULT (error code: {})\n{:#?}",
        error_code, stack_frame
    );
}

pub extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT (error code: {})\n{:#?}",
        error_code, stack_frame
    );
}

pub extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    panic!(
        "EXCEPTION: PAGE FAULT\nAccessed Address: {:?}\nError Code: {:?}\n{:#?}",
        Cr2::read(),
        error_code,
        stack_frame
    );
}

pub extern "x86-interrupt" fn floating_point_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: FLOATING POINT ERROR\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: ALIGNMENT CHECK (error code: {})\n{:#?}",
        error_code, stack_frame
    );
}

pub extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    panic!("EXCEPTION: MACHINE CHECK\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: SIMD FLOATING POINT ERROR\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn virtualization_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: VIRTUALIZATION EXCEPTION\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn security_exception_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: SECURITY EXCEPTION (error code: {})\n{:#?}",
        error_code, stack_frame
    );
}

// Hardware interrupt handlers
pub extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Update system timer ticks
    time::tick();
    
    // Send EOI (End of Interrupt) signal
    unsafe {
        super::irq::end_of_interrupt(32);
    }
}

pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Handle keyboard input
    keyboard::handle_interrupt();
    
    // Send EOI
    unsafe {
        super::irq::end_of_interrupt(33);
    }
}

pub extern "x86-interrupt" fn com1_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Handle COM1 serial port interrupt
    
    // Send EOI
    unsafe {
        super::irq::end_of_interrupt(36);
    }
}

pub extern "x86-interrupt" fn com2_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Handle COM2 serial port interrupt
    
    // Send EOI
    unsafe {
        super::irq::end_of_interrupt(37);
    }
}

pub extern "x86-interrupt" fn sound_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Handle sound card interrupt
    sound::handle_interrupt();
    
    // Send EOI
    unsafe {
        super::irq::end_of_interrupt(41);
    }
}

pub extern "x86-interrupt" fn gpu_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Handle GPU interrupt (like vsync or rendering completion)
    
    // Send EOI
    unsafe {
        super::irq::end_of_interrupt(42);
    }
}

pub extern "x86-interrupt" fn usb_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Handle USB controller interrupt
    
    // Send EOI
    unsafe {
        super::irq::end_of_interrupt(43);
    }
}

// Gaming-specific interrupt handlers
pub extern "x86-interrupt" fn gamepad_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Handle gamepad/controller input
    gamepad::handle_interrupt();
    
    // Send EOI
    unsafe {
        super::irq::end_of_interrupt(50);
    }
}

pub extern "x86-interrupt" fn network_gaming_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Handle network gaming events (like incoming multiplayer data)
    network::handle_gaming_interrupt();
    
    // Send EOI
    unsafe {
        super::irq::end_of_interrupt(51);
    }
}

// System call handler
pub extern "x86-interrupt" fn syscall_handler(_stack_frame: InterruptStackFrame) {
    // Handle system calls
    // In a real implementation, we would get the syscall number and parameters
    // from registers and dispatch to the appropriate handler
    
    // No EOI needed for software interrupts
}