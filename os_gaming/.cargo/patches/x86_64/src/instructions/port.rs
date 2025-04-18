use core::marker::PhantomData;

/// A port-mapped I/O interface.
///
/// This type is similar to the `Port` type from the `x86` crate.
/// It provides a safe interface to port-mapped I/O.
#[derive(Debug, Clone, Copy)]
pub struct Port<T> {
    port: u16,
    _phantom: PhantomData<T>,
}

impl<T> Port<T> {
    /// Creates a new port interface.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the given port
    /// is valid and not used by any other part of the program.
    pub const unsafe fn new(port: u16) -> Port<T> {
        Port {
            port,
            _phantom: PhantomData,
        }
    }
}

impl Port<u8> {
    /// Reads a byte from the port.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the port is
    /// valid and that reading from it is safe.
    pub unsafe fn read(&self) -> u8 {
        let mut value: u8;
        core::arch::asm!("in al, dx", out("al") value, in("dx") self.port, options(nomem, nostack));
        value
    }

    /// Writes a byte to the port.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the port is
    /// valid and that writing to it is safe.
    pub unsafe fn write(&self, value: u8) {
        core::arch::asm!("out dx, al", in("dx") self.port, in("al") value, options(nomem, nostack));
    }
}

impl Port<u16> {
    /// Reads a word from the port.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the port is
    /// valid and that reading from it is safe.
    pub unsafe fn read(&self) -> u16 {
        let mut value: u16;
        core::arch::asm!("in ax, dx", out("ax") value, in("dx") self.port, options(nomem, nostack));
        value
    }

    /// Writes a word to the port.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the port is
    /// valid and that writing to it is safe.
    pub unsafe fn write(&self, value: u16) {
        core::arch::asm!("out dx, ax", in("dx") self.port, in("ax") value, options(nomem, nostack));
    }
}

impl Port<u32> {
    /// Reads a double word from the port.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the port is
    /// valid and that reading from it is safe.
    pub unsafe fn read(&self) -> u32 {
        let mut value: u32;
        core::arch::asm!("in eax, dx", out("eax") value, in("dx") self.port, options(nomem, nostack));
        value
    }

    /// Writes a double word to the port.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the port is
    /// valid and that writing to it is safe.
    pub unsafe fn write(&self, value: u32) {
        core::arch::asm!("out dx, eax", in("dx") self.port, in("eax") value, options(nomem, nostack));
    }
} 