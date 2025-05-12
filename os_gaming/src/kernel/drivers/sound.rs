#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};
use core::arch::asm;
use core::ops;
use spin::Mutex;
#[cfg(feature = "std")]
use std::vec::Vec;
use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;
#[macro_use]
use lazy_static::lazy_static;
use micromath::F32Ext;


use crate::kernel::interrupts;

/// Supported sample rates
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SampleRate {
    Hz8000 = 8000,
    Hz11025 = 11025,
    Hz16000 = 16000,
    Hz22050 = 22050,
    Hz32000 = 32000,
    Hz44100 = 44100,
    Hz48000 = 48000,
}

// Constants for hardware ports
const PC_SPEAKER_PORT: u16 = 0x61;
const PIT_COMMAND_PORT: u16 = 0x43;
const PIT_CHANNEL2_PORT: u16 = 0x42;

// Sound Blaster ports (will be detected dynamically)
const SB16_DEFAULT_PORT: u16 = 0x220;
const SB16_DEFAULT_IRQ: u8 = 5;
const SB16_DEFAULT_DMA: u8 = 1;
const SB16_DEFAULT_DMA16: u8 = 5;

/// Sound hardware types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SoundHardwareType {
    None,
    PcSpeaker,
    SoundBlaster16,
    HdAudio,
    Virtualized,
}

/// Represents the sound hardware state
pub struct SoundDriver {
    initialized: AtomicBool,
    volume: u8,
    samples_buffer: Vec<i16>,
    hardware_type: SoundHardwareType,
    sb_base_port: u16,
    sb_irq: u8,
    sb_dma: u8,
    sb_dma16: u8,

    // HD Audio specific fields
    hda_mmio_base: *const u8,
    hda_irq: u8,
    hda_stream_count: u32,
    hda_output_stream: u32,
    hda_input_stream: u32,
}

// These traits must be implemented manually because raw pointers aren't Send or Sync by default.
// This is safe because we carefully control access to the pointers with proper synchronization.
unsafe impl Send for SoundDriver {}
unsafe impl Sync for SoundDriver {}

impl SoundHardwareType {
    pub fn get_name(&self) -> &str {
        match self {
            SoundHardwareType::None => "None",
            SoundHardwareType::PcSpeaker => "PC Speaker",
            SoundHardwareType::SoundBlaster16 => "Sound Blaster 16",
            SoundHardwareType::HdAudio => "HD Audio",
            SoundHardwareType::Virtualized => "Virtualized",
        }
    }

    pub fn get_device_type(&self) -> &str {
        match self {
            SoundHardwareType::None => "None",
            SoundHardwareType::PcSpeaker => "PcSpeaker",
            SoundHardwareType::SoundBlaster16 => "SoundBlaster16",
            SoundHardwareType::HdAudio => "HdAudio",
            SoundHardwareType::Virtualized => "Virtualized",
        }
    }
}

impl SoundDriver {
    /// Create a new sound driver instance
    pub fn new() -> Self {
        SoundDriver {
            initialized: AtomicBool::new(false),
            volume: 75, // Default volume (0-100)
            samples_buffer: Vec::new(),
            hardware_type: SoundHardwareType::None,
            sb_base_port: SB16_DEFAULT_PORT,
            sb_irq: SB16_DEFAULT_IRQ,
            sb_dma: SB16_DEFAULT_DMA,
            sb_dma16: SB16_DEFAULT_DMA16,

            // HD Audio defaults
            hda_mmio_base: core::ptr::null(),
            hda_irq: 0,
            hda_stream_count: 0,
            hda_output_stream: 0,
            hda_input_stream: 0,
        }
    }

    /// Initialize the sound hardware
    pub fn initialize(&mut self) -> Result<(), &'static str> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Try to detect and initialize sound hardware in priority order
        if self.detect_sound_blaster().is_ok() {
            self.hardware_type = SoundHardwareType::SoundBlaster16;
        } else {
            // Fallback to PC Speaker (always available)
            self.initialize_pc_speaker()?;
            self.hardware_type = SoundHardwareType::PcSpeaker;
        }

        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Initialize the PC Speaker
    fn initialize_pc_speaker(&mut self) -> Result<(), &'static str> {
        // PC Speaker doesn't require complex initialization
        // Just make sure it's initially off
        self.pc_speaker_off()?;
        Ok(())
    }

    pub fn shutdown(&mut self) {
        self.initialized.store(false, Ordering::SeqCst);
    }

    /// Detect Sound Blaster hardware
    fn detect_sound_blaster(&mut self) -> Result<(), &'static str> {
        // Try to detect Sound Blaster at common I/O ports
        let possible_ports = [0x220, 0x240, 0x260, 0x280];

        for &port in &possible_ports {
            if self.reset_dsp(port).is_ok() {
                self.sb_base_port = port;

                // Get DSP version
                if let Ok(version) = self.get_dsp_version() {
                    // Found a Sound Blaster!
                    #[cfg(feature = "std")]
                    log::info!(
                        "Detected Sound Blaster version {}.{} at port 0x{:X}",
                        version >> 8,
                        version & 0xFF,
                        port
                    );

                    // For SB16, detect IRQ and DMA channels
                    if version >= 0x0400 {
                        // SB16 or higher
                        self.detect_sb16_settings();
                    }

                    return Ok(());
                }
            }
        }

        Err("No Sound Blaster compatible device detected")
    }

    /// Reset the Sound Blaster DSP
    fn reset_dsp(&self, port: u16) -> Result<(), &'static str> {
        let reset_port = port + 0x6;
        let read_data_port = port + 0xA;
        let read_status_port = port + 0xE;

        // Safety: Direct port I/O requires unsafe
        unsafe {
            // 1. Write 1 to the reset port
            Port::new(reset_port).write(1u8);

            // 2. Delay at least 3 microseconds
            self.delay(10);

            // 3. Write 0 to the reset port
            Port::new(reset_port).write(0u8);

            // 4. Wait for bit 7 of read-status to be set and read 0xAA from read-data
            let timeout = 100; // Arbitrary timeout value
            for _ in 0..timeout {
                let status: u8 = Port::new(read_status_port).read();
                if (status & 0x80) != 0 {
                    let value: u8 = Port::new(read_data_port).read();
                    if value == 0xAA {
                        return Ok(());
                    }
                }
                self.delay(10);
            }
        }

        Err("DSP reset failed")
    }

    /// Write a command to the DSP
    fn write_dsp(&self, command: u8) -> Result<(), &'static str> {
        let write_port = self.sb_base_port + 0xC;
        let write_status_port = self.sb_base_port + 0xC;

        // Safety: Direct port I/O requires unsafe
        unsafe {
            // Wait until the DSP is ready to receive a command
            let timeout = 100;
            for _ in 0..timeout {
                let status: u8 = Port::new(write_status_port).read();
                if (status & 0x80) == 0 {
                    // Write the command
                    Port::new(write_port).write(command);
                    return Ok(());
                }
                self.delay(1);
            }
        }

        Err("DSP write timeout")
    }

    /// Queue audio for playback with callback for continuous streaming
    pub fn queue_audio(
        &mut self,
        initial_buffer: &[i16],
        sample_rate: SampleRate,
        buffer_callback: Option<fn(&mut SoundDriver) -> Option<Vec<i16>>>,
    ) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Sound driver not initialized");
        }

        let mut buffers = AUDIO_BUFFERS.lock();

        // Setup audio buffer system
        buffers.setup(8192, sample_rate); // 8K samples buffer size
        buffers.callback = buffer_callback;

        // Queue initial buffer
        buffers.queue_buffer(initial_buffer);
        buffers.playing = true;

        // Start playback based on hardware type
        match self.hardware_type {
            SoundHardwareType::SoundBlaster16 => {
                if let Some(buffer) = buffers.get_active_buffer() {
                    setup_sb16_dma(self, buffer, sample_rate);
                    Ok(())
                } else {
                    Err("Failed to get active buffer")
                }
            }
            SoundHardwareType::PcSpeaker => {
                // PC Speaker can't do sample playback properly
                Err("PC Speaker doesn't support sample streaming")
            }
            _ => Err("Hardware doesn't support sample streaming"),
        }
    }

    /// Public method to stop audio playback
    pub fn stop_playback(&mut self) -> Result<(), &'static str> {
        let mut buffers = AUDIO_BUFFERS.lock();

        if !buffers.playing {
            return Ok(());
        }

        buffers.playing = false;

        match self.hardware_type {
            SoundHardwareType::SoundBlaster16 => {
                stop_sb16_playback(self);
                Ok(())
            }
            SoundHardwareType::PcSpeaker => self.pc_speaker_off(),
            _ => Ok(()),
        }
    }

    /// Check if audio is currently playing
    pub fn is_playing(&self) -> bool {
        AUDIO_BUFFERS.lock().playing
    }

    /// Read a byte from the DSP
    fn read_dsp(&self) -> Result<u8, &'static str> {
        let read_data_port = self.sb_base_port + 0xA;
        let read_status_port = self.sb_base_port + 0xE;

        // Safety: Direct port I/O requires unsafe
        unsafe {
            // Wait until data is available
            let timeout = 100;
            for _ in 0..timeout {
                let status: u8 = Port::new(read_status_port).read();
                if (status & 0x80) != 0 {
                    // Read the data
                    return Ok(Port::new(read_data_port).read());
                }
                self.delay(1);
            }
        }

        Err("DSP read timeout")
    }

    /// Get the DSP version
    fn get_dsp_version(&self) -> Result<u16, &'static str> {
        // Ask for version
        self.write_dsp(0xE1)?;

        // Read major and minor version
        let major = self.read_dsp()?;
        let minor = self.read_dsp()?;

        Ok(((major as u16) << 8) | (minor as u16))
    }

    /// Detect SB16 IRQ and DMA settings
    fn detect_sb16_settings(&mut self) {
        // Read the mixer register to detect IRQ and DMA settings
        // This is a simplified implementation
        let mixer_addr_port = self.sb_base_port + 0x4;
        let mixer_data_port = self.sb_base_port + 0x5;

        unsafe {
            // Read IRQ configuration (mixer register 0x80)
            Port::new(mixer_addr_port).write(0x80u8);
            let irq_config: u8 = Port::new(mixer_data_port).read();

            self.sb_irq = match irq_config & 0x0F {
                0x01 => 2,
                0x02 => 5,
                0x04 => 7,
                0x08 => 10,
                _ => self.sb_irq, // Keep default if unknown
            };

            // Read DMA configuration (mixer register 0x81)
            Port::new(mixer_addr_port).write(0x81u8);
            let dma_config: u8 = Port::new(mixer_data_port).read();

            self.sb_dma = match dma_config & 0x0F {
                0x01 => 0,
                0x02 => 1,
                0x08 => 3,
                _ => self.sb_dma, // Keep default if unknown
            };

            // Read 16-bit DMA configuration (mixer register 0x82)
            Port::new(mixer_addr_port).write(0x82u8);
            let dma16_config: u8 = Port::new(mixer_data_port).read();

            self.sb_dma16 = match dma16_config & 0x0F {
                0x01 => 5,
                0x02 => 6,
                0x08 => 7,
                _ => self.sb_dma16, // Keep default if unknown
            };
        }
    }

    /// Set the SB16 master volume
    fn set_sb16_volume(&self, volume: u8) -> Result<(), &'static str> {
        let mixer_addr_port = self.sb_base_port + 0x4;
        let mixer_data_port = self.sb_base_port + 0x5;

        // Map 0-100 to 0-255 for the SB16 volume
        let sb_volume = ((volume as u16) * 255 / 100) as u8;

        unsafe {
            // Set master volume left (register 0x22)
            Port::new(mixer_addr_port).write(0x22u8);
            Port::new(mixer_data_port).write(sb_volume);

            // Set master volume right (register 0x23)
            Port::new(mixer_addr_port).write(0x23u8);
            Port::new(mixer_data_port).write(sb_volume);
        }

        Ok(())
    }

    /// Set the volume level (0-100)
    pub fn set_volume(&mut self, volume: u8) {
        let vol = volume.min(100);
        self.volume = vol;

        if !self.initialized.load(Ordering::SeqCst) {
            return;
        }

        match self.hardware_type {
            SoundHardwareType::SoundBlaster16 => {
                let _ = self.set_sb16_volume(vol);
            }
            SoundHardwareType::PcSpeaker => {
                // PC Speaker doesn't have volume control
                // We could simulate by adjusting duty cycle, but that's complex
            }
            _ => {}
        }
    }

    /// Get the current volume
    pub fn get_volume(&self) -> u8 {
        self.volume
    }

    pub fn get_output_devices(&self) -> Vec<SoundHardwareType> {
        let mut devices = Vec::new();

        if self.initialized.load(Ordering::SeqCst) {
            devices.push(self.hardware_type);
        }

        devices
    }

    pub fn get_name(&self) -> &str {
        match self.hardware_type {
            SoundHardwareType::SoundBlaster16 => "Sound Blaster 16",
            SoundHardwareType::PcSpeaker => "PC Speaker",
            SoundHardwareType::HdAudio => "HD Audio",
            _ => "Unknown",
        }
    }

    /// Play a sound sample
    pub fn play_sample(
        &mut self,
        sample_data: &[i16],
        sample_rate: SampleRate,
    ) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Sound driver not initialized");
        }

        // Store the sample in our buffer
        self.samples_buffer.clear();
        self.samples_buffer.extend_from_slice(sample_data);

        match self.hardware_type {
            SoundHardwareType::SoundBlaster16 => self.play_sample_sb16(sample_data, sample_rate),
            SoundHardwareType::PcSpeaker => {
                // PC Speaker can't play samples, so we'll just make a beep
                // with a frequency approximating the average sample
                self.play_beep_with_speaker(440, 100)
            }
            _ => Err("Unsupported sound hardware"),
        }
    }

    /// Play a sample on SB16
    fn play_sample_sb16(
        &self,
        sample_data: &[i16],
        sample_rate: SampleRate,
    ) -> Result<(), &'static str> {
        // This is a simplified implementation that would need DMA setup
        // for a real driver

        // 1. Set the sample rate
        let rate = sample_rate as u16;
        self.write_dsp(0x41)?; // Set sample rate command
        self.write_dsp(((rate >> 8) & 0xFF) as u8)?; // High byte
        self.write_dsp((rate & 0xFF) as u8)?; // Low byte

        // 2. Set the sample size and mode
        // Single-cycle 16-bit signed PCM stereo
        self.write_dsp(0xB6)?;

        // 3. Set the length (minus 1)
        let length = sample_data.len() as u16 - 1;
        self.write_dsp((length & 0xFF) as u8)?; // Low byte
        self.write_dsp(((length >> 8) & 0xFF) as u8)?; // High byte

        // 4. In a real driver, we would:
        //    - Set up DMA transfer for the sample data
        //    - Configure IRQ for completion notification
        //    - Copy data to the DMA buffer

        // For this implementation, we'll simulate it
        #[cfg(feature = "std")]
        log::info!(
            "Simulating playback of {} samples at {} Hz",
            sample_data.len(),
            sample_rate as u16
        );

        Ok(())
    }

    /// Play a beep using the PC Speaker
    pub fn play_beep_with_speaker(
        &self,
        frequency: u16,
        duration_ms: u32,
    ) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Sound driver not initialized");
        }

        // Set up the PIT to generate a square wave at the specified frequency
        let divisor: u16 = 65535 / frequency;

        // Safety: Direct port I/O requires unsafe
        unsafe {
            // Set up the PIT channel 2 in mode 3 (square wave generator)
            Port::new(PIT_COMMAND_PORT).write(0xB6u8); // 10110110 in binary

            // Set the divisor
            Port::new(PIT_CHANNEL2_PORT).write((divisor & 0xFF) as u8); // Low byte
            Port::new(PIT_CHANNEL2_PORT).write(((divisor >> 8) & 0xFF) as u8); // High byte

            // Turn on the PC speaker
            let mut speaker_state: u8 = Port::new(PC_SPEAKER_PORT).read();
            speaker_state |= 0x03; // Set bits 0 and 1 (gate and data)
            Port::new(PC_SPEAKER_PORT).write(speaker_state);
        }

        // Sleep for the specified duration, if available
        #[cfg(feature = "std")]
        {
            std::thread::sleep(std::time::Duration::from_millis(duration_ms as u64));
            self.pc_speaker_off()?;
        }

        // In no_std mode, we would need a different approach for timing
        #[cfg(not(feature = "std"))]
        {
            // Delay for duration_ms milliseconds using a busy-wait
            for _ in 0..duration_ms {
                self.delay(1000); // Delay 1ms
            }
            self.pc_speaker_off()?;
        }

        Ok(())
    }

    /// Turn off the PC speaker
    fn pc_speaker_off(&self) -> Result<(), &'static str> {
        // Safety: Direct port I/O requires unsafe
        unsafe {
            let mut speaker_state: u8 = Port::new(PC_SPEAKER_PORT).read();
            speaker_state &= 0xFC; // Clear bits 0 and 1
            Port::new(PC_SPEAKER_PORT).write(speaker_state);
        }
        Ok(())
    }

    /// Stop current playback
    pub fn stop(&mut self) {
        if !self.initialized.load(Ordering::SeqCst) {
            return;
        }

        match self.hardware_type {
            SoundHardwareType::SoundBlaster16 => {
                // Send command to stop DSP playback
                let _ = self.write_dsp(0xD0);
            }
            SoundHardwareType::PcSpeaker => {
                let _ = self.pc_speaker_off();
            }
            _ => {}
        }

        self.samples_buffer.clear();
    }

    /// Play a simple beep
    pub fn beep(&mut self, frequency: u16, duration_ms: u32) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Sound driver not initialized");
        }

        match self.hardware_type {
            SoundHardwareType::SoundBlaster16 => {
                // For SB16, we'll generate a simple sine wave and play it as a sample
                let sample_rate = SampleRate::Hz16000;
                let samples = self.generate_test_tone(frequency, duration_ms, sample_rate);
                self.play_sample(&samples, sample_rate)
            }
            SoundHardwareType::PcSpeaker => self.play_beep_with_speaker(frequency, duration_ms),
            _ => Err("Unsupported sound hardware"),
        }
    }

    /// High-precision delay function
    fn delay(&self, microseconds: u32) {
        #[cfg(feature = "std")]
        {
            // In std mode, we can use thread sleep
            std::thread::sleep(std::time::Duration::from_micros(microseconds as u64));
            return;
        }

        #[cfg(not(feature = "std"))]
        {
            // In kernel mode, we have multiple options:

            // Option 1: Use the TSC (Time Stamp Counter) for precise timing if available
            if let Some(tsc_frequency) = self.get_tsc_frequency() {
                let start = self.read_tsc();
                let ticks_to_wait = (tsc_frequency as u64 * microseconds as u64) / 1_000_000;
                let end = start + ticks_to_wait;

                while self.read_tsc() < end {
                    core::hint::spin_loop();
                }

                return;
            }

            // Option 2: Use PIT for timing
            // Calculate iterations based on CPU speed (rough approximation)
            let calibrated_loops_per_us = if let Some(loops) = self.get_calibrated_loops() {
                loops
            } else {
                // Default fallback if not calibrated
                10 // This is a very rough approximation!
            };

            for _ in 0..microseconds * calibrated_loops_per_us {
                core::hint::spin_loop();
            }
        }
    }

    #[cfg(not(feature = "std"))]
    fn get_tsc_frequency(&self) -> Option<u64> {
        // We would normally retrieve this from CPUID or calibrate at startup
        // For now, return None to use the PIT method
        None
    }

    #[cfg(not(feature = "std"))]
    fn read_tsc(&self) -> u64 {
        // Read the time stamp counter
        unsafe {
            let low: u32;
            let high: u32;

            core::arch::asm!(
                "rdtsc",
                out("eax") low,
                out("edx") high,
                options(nomem, nostack)
            );

            ((high as u64) << 32) | (low as u64)
        }
    }

    #[cfg(not(feature = "std"))]
    fn get_calibrated_loops(&self) -> Option<u32> {
        // This would be calibrated during initialization
        // by measuring how many loops occur during a known time interval
        static LOOPS_PER_US: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

        let loops = LOOPS_PER_US.load(core::sync::atomic::Ordering::Relaxed);
        if loops > 0 {
            Some(loops)
        } else {
            None
        }
    }

    pub fn hda_irq(&self) {
        // Constants for HD Audio registers
        const HDA_REG_GSTS: u32 = 0x04; // Global Status Register
        const HDA_REG_RIRBSTS: u32 = 0x05; // Response Interrupt Status
        const HDA_REG_INTSTS: u32 = 0x34; // Interrupt Status Register
        const HDA_REG_SSYNC: u32 = 0x38; // Stream Synchronization

        const HDA_INTSTS_GIS: u32 = 0x01; // Global Interrupt Status
        const HDA_INTSTS_CIS: u32 = 0x02; // Controller Interrupt Status
        const HDA_INTSTS_SIS: u32 = 0x04; // Stream Interrupt Status

        const HDA_STREAM_REG_BASE: u32 = 0x80; // Base address of stream registers
        const HDA_STREAM_REG_STRIDE: u32 = 0x20; // Stride between streams

        // Safety: this accesses memory-mapped hardware registers
        unsafe {
            // 1. Read the interrupt status register to determine cause
            let hda_base = self.hda_mmio_base;
            if hda_base.is_null() {
                return; // No HD Audio controller initialized
            }

            let int_status = self.read_hda_reg(hda_base, HDA_REG_INTSTS);

            // 2. Process global interrupts if any
            if (int_status & HDA_INTSTS_GIS) != 0 {
                // Read global status register
                let global_status = self.read_hda_reg(hda_base, HDA_REG_GSTS);

                // Handle state change interrupts
                if global_status != 0 {
                    #[cfg(feature = "std")]
                    log::debug!("HD Audio state change: 0x{:X}", global_status);

                    // Clear the state change status
                    self.write_hda_reg(hda_base, HDA_REG_GSTS, global_status);
                }
            }

            // 3. Process response interrupts (RIRB)
            if (int_status & HDA_INTSTS_CIS) != 0 {
                let rirb_status = self.read_hda_reg(hda_base, HDA_REG_RIRBSTS);

                if rirb_status != 0 {
                    // Process any responses from codecs
                    self.process_hda_codec_responses();

                    // Clear RIRB interrupt status
                    self.write_hda_reg(hda_base, HDA_REG_RIRBSTS, rirb_status);
                }
            }

            // 4. Process stream interrupts
            if (int_status & HDA_INTSTS_SIS) != 0 {
                // Determine which streams have pending interrupts
                for stream_idx in 0..self.hda_stream_count {
                    let stream_base = hda_base.add(
                        (HDA_STREAM_REG_BASE + stream_idx as u32 * HDA_STREAM_REG_STRIDE) as usize,
                    );

                    // Process this stream if it has an interrupt pending
                    if self.process_hda_stream_interrupt(stream_base, stream_idx) {
                        // Notify audio system of buffer completion
                        self.notify_buffer_completion(stream_idx);
                    }
                }

                // Synchronize streams if needed
                self.write_hda_reg(hda_base, HDA_REG_SSYNC, 0x01);
            }
        }

        // 5. Send EOI to acknowledge the interrupt
        unsafe {
            crate::kernel::interrupts::irq::end_of_interrupt(self.hda_irq + 32);
        }
    }

    /// Read from an HD Audio controller register
    unsafe fn read_hda_reg(&self, base: *const u8, reg: u32) -> u32 {
        let reg_addr = base.add(reg as usize) as *const u32;
        core::ptr::read_volatile(reg_addr)
    }

    /// Write to an HD Audio controller register
    unsafe fn write_hda_reg(&self, base: *const u8, reg: u32, value: u32) {
        let reg_addr = base.add(reg as usize) as *mut u32;
        core::ptr::write_volatile(reg_addr, value);
    }

    /// Process responses from HDA codecs
    fn process_hda_codec_responses(&self) {
        // Constants for response buffer
        const HDA_RIRB_BASE: u32 = 0x50;
        const HDA_RIRB_RP: u32 = (HDA_RIRB_BASE + 0x04);
        const HDA_RIRB_WP: u32 = (HDA_RIRB_BASE + 0x08);

        unsafe {
            if self.hda_mmio_base.is_null() {
                return;
            }

            // Get read and write pointers
            let rp = self.read_hda_reg(self.hda_mmio_base, HDA_RIRB_RP);
            let wp = self.read_hda_reg(self.hda_mmio_base, HDA_RIRB_WP);

            if rp == wp {
                return; // No responses to process
            }

            // Process all available responses
            // In a real implementation, we would:
            // 1. Read the response entries from the RIRB
            // 2. Update our codec state based on responses
            // 3. Complete any pending commands

            // Update read pointer after processing
            self.write_hda_reg(self.hda_mmio_base, HDA_RIRB_RP, wp);
        }
    }

    /// Process a stream interrupt and return true if a buffer completed
    fn process_hda_stream_interrupt(&self, stream_base: *const u8, stream_idx: u32) -> bool {
        // Stream register offsets
        const SD_REG_STS: u32 = 0x03; // Status Register
        const SD_REG_CTL: u32 = 0x00; // Control Register
        const SD_REG_LPIB: u32 = 0x04; // Link Position in Buffer

        // Status register bits
        const SD_STS_BCIS: u32 = 0x04; // Buffer Completion Interrupt Status
        const SD_STS_FIFOE: u32 = 0x08; // FIFO Error

        unsafe {
            // Read status register
            let status = self.read_hda_reg(stream_base as *const u8, SD_REG_STS);

            // Check if buffer completion interrupt occurred
            let buffer_completed = (status & SD_STS_BCIS) != 0;

            // Check for FIFO errors
            if (status & SD_STS_FIFOE) != 0 {
                #[cfg(feature = "std")]
                log::error!("HD Audio Stream {} FIFO error", stream_idx);

                // Reset the stream
                let ctl = self.read_hda_reg(stream_base as *const u8, SD_REG_CTL);
                self.write_hda_reg(stream_base as *const u8, SD_REG_CTL, ctl & !0x02); // Clear run bit
                self.delay(100); // Short delay
                self.write_hda_reg(stream_base as *const u8, SD_REG_CTL, ctl); // Restore run bit
            }

            // Clear status bits by writing them back
            if status != 0 {
                self.write_hda_reg(stream_base as *const u8, SD_REG_STS, status);
            }

            buffer_completed
        }
    }

    /// Notify the audio system that a buffer has completed
    fn notify_buffer_completion(&self, stream_idx: u32) {
        // In a real implementation, this would:
        // 1. Get the next buffer from the audio queue
        // 2. Update the buffer descriptor list (BDL)
        // 3. Continue playback or notify higher layers

        // For this example, we'll update AUDIO_BUFFERS if we're using stream 0
        if stream_idx == 0 {
            let mut buffers = AUDIO_BUFFERS.lock();

            if !buffers.playing {
                return;
            }

            // Handle buffer switch similar to SB16 implementation
            let sample_rate = buffers.sample_rate;
            if let Some(next_buffer) = buffers.switch_buffer() {
                // Update the HD Audio buffer for the next playback
                self.setup_hda_buffer(next_buffer, sample_rate);
            } else if let Some(callback) = buffers.callback {
                let mut driver = SOUND_DRIVER.lock();

                if let Some(new_data) = callback(&mut driver) {
                    buffers.queue_buffer(&new_data);

                    if let Some(buffer) = buffers.get_active_buffer() {
                        self.setup_hda_buffer(buffer, sample_rate);
                    }
                } else {
                    buffers.playing = false;
                    self.stop_hda_playback();
                }
            } else {
                buffers.playing = false;
                self.stop_hda_playback();
            }
        }
    }

    /// Setup HD Audio buffer for playback
    fn setup_hda_buffer(&self, buffer: &[i16], sample_rate: SampleRate) {
        // In a real implementation, this would:
        // 1. Copy the buffer data to a DMA-accessible buffer
        // 2. Update the buffer descriptor list (BDL)
        // 3. Configure the stream for the correct sample rate and format
        // 4. Start or continue playback

        #[cfg(feature = "std")]
        log::trace!(
            "HD Audio buffer setup: {} samples at {} Hz",
            buffer.len(),
            sample_rate as u32
        );
    }

    /// Stop HD Audio playback
    fn stop_hda_playback(&self) {
        // In a real implementation, this would:
        // 1. Stop the stream (clear run bit)
        // 2. Reset the stream if necessary
        // 3. Cleanup any resources

        #[cfg(feature = "std")]
        log::trace!("HD Audio playback stopped");
    }

    /// Get the detected hardware type
    pub fn get_hardware_type(&self) -> SoundHardwareType {
        self.hardware_type
    }

    /// Generate a test tone (sine wave) of the specified frequency and duration
    fn generate_test_tone(
    &self,
    frequency: u16,
    duration_ms: u32,
    sample_rate: SampleRate,
    ) -> Vec<i16> {
    let sample_rate_hz = sample_rate as u32;
    // Prevent overflow by using u64 for intermediate calculation
    let num_samples = ((sample_rate_hz as u64 * duration_ms as u64) / 1000) as usize;
    let mut samples = Vec::with_capacity(num_samples);

    let amplitude: f32 = 0x7FFF as f32 * (self.volume.min(100) as f32 / 100.0);
    let period: f32 = sample_rate_hz as f32 / frequency as f32;

    for i in 0..num_samples {
    let angle = ((i as f32) / period) * 2.0 * core::f32::consts::PI;
    // Ensure we don't exceed i16 bounds
    let sample = (angle.sin() * amplitude).clamp(-32768.0, 32767.0) as i16;
    samples.push(sample);
    }
    samples
    }

}

pub fn init() -> Result<SoundDriver, &'static str> {
    let mut sound_driver = SoundDriver::new();
    sound_driver.initialize()?;
    Ok(sound_driver)
}


struct AudioBuffers {
    buffer_a: Vec<i16>,
    buffer_b: Vec<i16>,
    active_buffer: AudioBufferActive,
    playing: bool,
    sample_rate: SampleRate,
    buffer_size: usize,
    position: usize,
    callback: Option<fn(&mut SoundDriver) -> Option<Vec<i16>>>,
}

enum AudioBufferActive {
    None,
    BufferA,
    BufferB,
}

lazy_static! {
    static ref SOUND_DRIVER: Mutex<SoundDriver> = Mutex::new(SoundDriver::new());
    static ref AUDIO_BUFFERS: Mutex<AudioBuffers> = Mutex::new(AudioBuffers::new());
}

impl AudioBuffers {
    fn new() -> Self {
        AudioBuffers {
            buffer_a: Vec::new(),
            buffer_b: Vec::new(),
            active_buffer: AudioBufferActive::None,
            playing: false,
            sample_rate: SampleRate::Hz22050,
            buffer_size: 4096,
            position: 0,
            callback: None,
        }
    }

    fn setup(&mut self, size: usize, rate: SampleRate) {
        self.buffer_a = Vec::with_capacity(size);
        self.buffer_b = Vec::with_capacity(size);
        self.buffer_size = size;
        self.sample_rate = rate;
        self.position = 0;
        self.active_buffer = AudioBufferActive::None;
        self.playing = false;
    }

    fn queue_buffer(&mut self, data: &[i16]) -> bool {
        match self.active_buffer {
            AudioBufferActive::None => {
                // First buffer - fill buffer A
                self.buffer_a.clear();
                self.buffer_a.extend_from_slice(data);
                self.active_buffer = AudioBufferActive::BufferA;
                true
            }
            AudioBufferActive::BufferA => {
                // Buffer A active, fill buffer B
                self.buffer_b.clear();
                self.buffer_b.extend_from_slice(data);
                true
            }
            AudioBufferActive::BufferB => {
                // Buffer B active, fill buffer A
                self.buffer_a.clear();
                self.buffer_a.extend_from_slice(data);
                true
            }
        }
    }

    fn get_active_buffer(&self) -> Option<&[i16]> {
        match self.active_buffer {
            AudioBufferActive::None => None,
            AudioBufferActive::BufferA => Some(&self.buffer_a),
            AudioBufferActive::BufferB => Some(&self.buffer_b),
        }
    }

    fn switch_buffer(&mut self) -> Option<&[i16]> {
        self.position = 0;

        match self.active_buffer {
            AudioBufferActive::None => None,
            AudioBufferActive::BufferA => {
                // Switch to buffer B if it has data
                if !self.buffer_b.is_empty() {
                    self.active_buffer = AudioBufferActive::BufferB;
                    Some(&self.buffer_b)
                } else {
                    // No data in buffer B
                    None
                }
            }
            AudioBufferActive::BufferB => {
                // Switch to buffer A if it has data
                if !self.buffer_a.is_empty() {
                    self.active_buffer = AudioBufferActive::BufferA;
                    Some(&self.buffer_a)
                } else {
                    // No data in buffer A
                    None
                }
            }
        }
    }
}

pub fn get_sound_driver() -> impl ops::DerefMut<Target = SoundDriver> {
    SOUND_DRIVER.lock()
}

/// Handle sound hardware interrupt
pub fn handle_interrupt() {
    // Determine which hardware generated the interrupt
    let mut driver = SOUND_DRIVER.lock();

    match driver.hardware_type {
        SoundHardwareType::SoundBlaster16 => handle_sb16_interrupt(&mut driver),
        SoundHardwareType::HdAudio => handle_hda_interrupt(&mut driver),
        _ => {
            // Other hardware types might not use interrupts
            #[cfg(feature = "std")]
            log::warn!("Received interrupt for non-interrupt-driven sound hardware");
        }
    }
}

/// Handle Sound Blaster 16 interrupt
fn handle_sb16_interrupt(driver: &mut SoundDriver) {
    // 1. Acknowledge the interrupt
    // Read SB16 interrupt status register to clear the interrupt
    let status_port = driver.sb_base_port + 0xE;
    unsafe {
        let _status: u8 = Port::new(status_port).read();
    }

    // 2. Get audio buffers
    let mut buffers = AUDIO_BUFFERS.lock();

    if !buffers.playing {
        return;
    }

    // 3. Switch to next buffer if current one is complete
    let sample_rate = buffers.sample_rate;
    if let Some(next_buffer) = buffers.switch_buffer() {
        // Start DMA for the next buffer
        setup_sb16_dma(driver, next_buffer, sample_rate);
    } else {
        // No more data, check if we have a callback for more audio
        if let Some(callback) = buffers.callback {
            if let Some(new_data) = callback(driver) {
                // Got more audio data, queue it
                buffers.queue_buffer(&new_data);

                // And start playing it
                if let Some(buffer) = buffers.get_active_buffer() {
                    setup_sb16_dma(driver, buffer, sample_rate);
                }
            } else {
                // No more audio from callback, stop playback
                buffers.playing = false;
                stop_sb16_playback(driver);
            }
        } else {
            // No callback and no more buffers - we're done
            buffers.playing = false;
            stop_sb16_playback(driver);
        }
    }

    // 4. Send EOI to the PIC to allow more interrupts
    unsafe {
        // Use the IRQ number from the driver
        crate::kernel::interrupts::irq::end_of_interrupt(driver.sb_irq + 32);
    }
}

/// Handle HD Audio interrupt
fn handle_hda_interrupt(driver: &mut SoundDriver) {
    // HD Audio uses a different interrupt model with memory-mapped registers
    // This would need to be implemented based on your HD Audio controller specifics

    #[cfg(feature = "std")]
    log::trace!("HD Audio interrupt received");

    // Send EOI
    unsafe {
        crate::kernel::interrupts::irq::end_of_interrupt(driver.hda_irq + 32);
    }
}

/// Set up the DMA controller for SB16 playback
fn setup_sb16_dma(driver: &SoundDriver, buffer: &[i16], sample_rate: SampleRate) {
    // This is a simplified version - a real implementation would:
    // 1. Set up the DMA controller registers for the appropriate channel
    // 2. Program the DMA transfer count
    // 3. Set up DMA page register
    // 4. Tell SB16 to start playback with appropriate commands

    // For this example, we'll just simulate the process

    // Set up the sample rate
    let rate = sample_rate as u16;
    let _ = driver.write_dsp(0x41); // Set sample rate command
    let _ = driver.write_dsp(((rate >> 8) & 0xFF) as u8); // High byte
    let _ = driver.write_dsp((rate & 0xFF) as u8); // Low byte

    // Configure for 16-bit audio playback
    let _ = driver.write_dsp(0xB6); // 16-bit DMA output, signed, mono

    // Set the length (bytes, minus 1)
    let length = (buffer.len() * 2) as u16 - 1; // *2 because each sample is 2 bytes
    let _ = driver.write_dsp((length & 0xFF) as u8); // Low byte
    let _ = driver.write_dsp(((length >> 8) & 0xFF) as u8); // High byte

    #[cfg(feature = "std")]
    log::trace!("SB16 DMA setup: {} samples at {} Hz", buffer.len(), rate);
}

/// Stop SB16 playback
fn stop_sb16_playback(driver: &SoundDriver) {
    let _ = driver.write_dsp(0xD5); // Stop 16-bit DMA mode

    #[cfg(feature = "std")]
    log::trace!("SB16 playback stopped");
}

pub fn beep(
    frequency: u16,
    duration_ms: u32,
) -> Result<(), &'static str> {
    let mut driver = SOUND_DRIVER.lock();
    driver.beep(frequency, duration_ms)
}

pub fn get_volume() -> u8 {
    let driver = SOUND_DRIVER.lock();
    driver.get_volume()
}
pub fn set_volume(volume: u8) {
    let mut driver = SOUND_DRIVER.lock();
    driver.set_volume(volume);
}

pub fn is_enabled() -> bool {
    let driver = SOUND_DRIVER.lock();
    driver.initialized.load(Ordering::SeqCst)
}

pub fn set_enabled(enabled: bool) {
    let mut driver = SOUND_DRIVER.lock();
    if enabled {
        driver.initialize().unwrap_or_default();
    } else {
        driver.shutdown();
    }
}
pub fn shutdown() {
    let mut driver = SOUND_DRIVER.lock();
    driver.stop_playback().unwrap_or_default();
    driver.initialized.store(false, Ordering::SeqCst);
}
