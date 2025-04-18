#![no_std]

use core::marker::PhantomData;

/// Represents a CPU register.
#[derive(Debug, Clone, Copy)]
pub struct Register<T> {
    _phantom: PhantomData<T>,
}

impl<T> Register<T> {
    /// Creates a new register.
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

/// Represents the general-purpose registers.
pub mod gpr {
    use super::*;

    /// The RAX register.
    pub const RAX: Register<u64> = Register::new();
    
    /// The RBX register.
    pub const RBX: Register<u64> = Register::new();
    
    /// The RCX register.
    pub const RCX: Register<u64> = Register::new();
    
    /// The RDX register.
    pub const RDX: Register<u64> = Register::new();
    
    /// The RSI register.
    pub const RSI: Register<u64> = Register::new();
    
    /// The RDI register.
    pub const RDI: Register<u64> = Register::new();
    
    /// The RBP register.
    pub const RBP: Register<u64> = Register::new();
    
    /// The RSP register.
    pub const RSP: Register<u64> = Register::new();
    
    /// The R8 register.
    pub const R8: Register<u64> = Register::new();
    
    /// The R9 register.
    pub const R9: Register<u64> = Register::new();
    
    /// The R10 register.
    pub const R10: Register<u64> = Register::new();
    
    /// The R11 register.
    pub const R11: Register<u64> = Register::new();
    
    /// The R12 register.
    pub const R12: Register<u64> = Register::new();
    
    /// The R13 register.
    pub const R13: Register<u64> = Register::new();
    
    /// The R14 register.
    pub const R14: Register<u64> = Register::new();
    
    /// The R15 register.
    pub const R15: Register<u64> = Register::new();
} 