#![no_std]
#![cfg_attr(feature = "abi_x86_interrupt", allow(stable_features))]
#![cfg_attr(feature = "step_trait", allow(stable_features))]

// Re-export the modules
pub mod addr;
pub mod instructions;
pub mod registers;
pub mod structures;
pub mod VirtAddr;
pub mod PhysAddr;
pub mod Page;
pub mod frame;
pub mod paging;

// Re-export commonly used types
pub use instructions::port::Port;
pub use registers::Register;
pub use structures::{Gdt, Idt, IdtEntry};
pub use frame::Frame;
pub use paging::PageTableEntry;