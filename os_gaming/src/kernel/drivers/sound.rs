#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use x86_64::instructions::port::Port;
#[cfg(feature = "std")]
use std::vec::Vec;
use core::{sync::atomic::{AtomicBool, Ordering}};

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
                    log::info!("Detected Sound Blaster version {}.{} at port 0x{:X}",
                              version >> 8, version & 0xFF, port);
                    
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
            },
            SoundHardwareType::PcSpeaker => {
                // PC Speaker doesn't have volume control
                // We could simulate by adjusting duty cycle, but that's complex
            },
            _ => {},
        }
    }

    /// Get the current volume
    pub fn get_volume(&self) -> u8 {
        self.volume
    }

    /// Play a sound sample
    pub fn play_sample(&mut self, sample_data: &[i16], sample_rate: SampleRate) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Sound driver not initialized");
        }
        
        // Store the sample in our buffer
        self.samples_buffer.clear();
        self.samples_buffer.extend_from_slice(sample_data);
        
        match self.hardware_type {
            SoundHardwareType::SoundBlaster16 => {
                self.play_sample_sb16(sample_data, sample_rate)
            },
            SoundHardwareType::PcSpeaker => {
                // PC Speaker can't play samples, so we'll just make a beep
                // with a frequency approximating the average sample
                self.play_beep_with_speaker(440, 100)
            },
            _ => Err("Unsupported sound hardware"),
        }
    }

    /// Play a sample on SB16
    fn play_sample_sb16(&self, sample_data: &[i16], sample_rate: SampleRate) -> Result<(), &'static str> {
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
        log::info!("Simulating playback of {} samples at {} Hz", 
                  sample_data.len(), sample_rate as u16);
        
        Ok(())
    }

    /// Play a beep using the PC Speaker
    pub fn play_beep_with_speaker(&self, frequency: u16, duration_ms: u32) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Sound driver not initialized");
        }
        
        // Set up the PIT to generate a square wave at the specified frequency
        let divisor: u16 = 1193180 / frequency;
        
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
            },
            SoundHardwareType::PcSpeaker => {
                let _ = self.pc_speaker_off();
            },
            _ => {},
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
            },
            SoundHardwareType::PcSpeaker => {
                self.play_beep_with_speaker(frequency, duration_ms)
            },
            _ => Err("Unsupported sound hardware"),
        }
    }

    /// Simple delay function
    fn delay(&self, microseconds: u32) {
        // In a real driver, this would be a proper delay function
        // For now, we'll just do a simple busy wait
        
        // This is a very rough approximation and not accurate!
        for _ in 0..microseconds * 10 {
            core::hint::spin_loop();
        }
    }

    /// Get the detected hardware type
    pub fn get_hardware_type(&self) -> SoundHardwareType {
        self.hardware_type
    }
    
    /// Generate a test tone (sine wave) of the specified frequency and duration
    fn generate_test_tone(&self, frequency: u16, duration_ms: u32, sample_rate: SampleRate) -> Vec<i16> {
        let sample_rate_hz = sample_rate as u32;
        let num_samples = (sample_rate_hz * duration_ms) / 1000;
        let mut samples = Vec::with_capacity(num_samples as usize);
        
        // Constants for the sine wave calculation
        let amplitude: f32 = 0x7FFF as f32 * (self.volume as f32 / 100.0);
        let period: f32 = sample_rate_hz as f32 / frequency as f32;
        
        for i in 0..num_samples {
            let angle = (i as f32 / period) * 2.0 * core::f32::consts::PI;
            let sample = (angle.sin() * amplitude) as i16;
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