#![no_std]

use core::marker::PhantomData;

/// Represents a PIC interrupt controller.
#[derive(Debug, Clone, Copy)]
pub struct Pic {
    port: u16,
}

impl Pic {
    /// Creates a new PIC controller.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the given port
    /// is valid and not used by any other part of the program.
    pub const unsafe fn new(port: u16) -> Pic {
        Pic { port }
    }

    /// Reads a byte from the PIC.
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

    /// Writes a byte to the PIC.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the port is
    /// valid and that writing to it is safe.
    pub unsafe fn write(&self, value: u8) {
        core::arch::asm!("out dx, al", in("dx") self.port, in("al") value, options(nomem, nostack));
    }
}

/// Represents a chained pair of PIC controllers.
#[derive(Debug, Clone, Copy)]
pub struct ChainedPics {
    primary: Pic,
    secondary: Pic,
    offset: u8,
}

impl ChainedPics {
    /// Creates a new chained pair of PIC controllers.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the given ports
    /// are valid and not used by any other part of the program.
    pub const unsafe fn new(offset1: u8, offset2: u8) -> ChainedPics {
        ChainedPics {
            primary: Pic::new(0x20),
            secondary: Pic::new(0xA0),
            offset: offset1,
        }
    }

    /// Initializes the PIC controllers.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs low-level hardware operations.
    pub unsafe fn initialize(&mut self) {
        // Save the current masks
        let primary_mask = self.primary.read();
        let secondary_mask = self.secondary.read();

        // Start the initialization sequence
        self.primary.write(0x11);
        self.secondary.write(0x11);

        // Set the vector offsets
        self.primary.write(self.offset);
        self.secondary.write(self.offset + 8);

        // Tell the PICs how they are connected
        self.primary.write(0x04);
        self.secondary.write(0x02);

        // Set the operating mode
        self.primary.write(0x01);
        self.secondary.write(0x01);

        // Restore the masks
        self.primary.write(primary_mask);
        self.secondary.write(secondary_mask);
    }

    /// Disables the PIC controllers.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs low-level hardware operations.
    pub unsafe fn disable(&mut self) {
        self.primary.write(0xFF);
        self.secondary.write(0xFF);
    }
}

/// Represents an IRQ number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Irq {
    number: u8,
}

impl Irq {
    /// Creates a new IRQ number.
    pub const fn new(number: u8) -> Irq {
        Irq { number }
    }

    /// Returns the IRQ number.
    pub const fn as_u8(self) -> u8 {
        self.number
    }

    /// Returns the IRQ number as a usize.
    pub const fn as_usize(self) -> usize {
        self.number as usize
    }
}

/// Represents an interrupt vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InterruptVector {
    number: u8,
}

impl InterruptVector {
    /// Creates a new interrupt vector.
    pub const fn new(number: u8) -> InterruptVector {
        InterruptVector { number }
    }

    /// Returns the interrupt vector number.
    pub const fn as_u8(self) -> u8 {
        self.number
    }

    /// Returns the interrupt vector number as a usize.
    pub const fn as_usize(self) -> usize {
        self.number as usize
    }
} 