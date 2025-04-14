pub mod pic {
    use pic8259::ChainedPics;
    use spin::Mutex;

    /// Primary PIC offset
    pub const PIC_1_OFFSET: u8 = 32;
    /// Secondary PIC offset
    pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

    /// The programmable interrupt controllers
    pub static PICS: Mutex<ChainedPics> =
        Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

    /// Initialize the PIC
    pub fn init() {
        unsafe {
            PICS.lock().initialize();
        }
    }

    /// Send end of interrupt signal
    pub unsafe fn end_of_interrupt(interrupt_id: u8) {
        PICS.lock().notify_end_of_interrupt(interrupt_id);
    }
}

/// Send end of interrupt signal to the appropriate controller
pub unsafe fn end_of_interrupt(interrupt_id: u8) {
    #[cfg(feature = "apic")]
    {
        if super::apic::is_enabled() {
            super::apic::end_of_interrupt();
            return;
        }
    }

    pic::end_of_interrupt(interrupt_id);
}