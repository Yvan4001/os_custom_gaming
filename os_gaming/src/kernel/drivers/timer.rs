use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::instructions::port::Port;
use x86_64::registers::model_specific::Msr;
use x86_64::structures::idt::InterruptStackFrame;
use spin::Mutex;
use lazy_static::lazy_static;

// Constants
pub const PIT_FREQUENCY: u32 = 1193182; // Base frequency of the PIT (Hz)
pub const PIT_COMMAND_PORT: u16 = 0x43;
pub const PIT_CHANNEL0_PORT: u16 = 0x40;
pub const DEFAULT_TICK_RATE: u16 = 1000; // Default tick rate (Hz)

// Timer sources
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimerSource {
    PIT,
    HPET,
    TSC,
    APIC,
}

// Global timer state
lazy_static! {
    /// The number of timer ticks since system boot
    static ref TICKS: AtomicU64 = AtomicU64::new(0);
    
    /// The calibrated CPU frequency in MHz
    static ref CPU_MHZ: AtomicU64 = AtomicU64::new(0);
    
    /// Timer manager instance
    static ref TIMER_MANAGER: Mutex<TimerManager> = Mutex::new(TimerManager::new());
}

/// Represents the system timer manager
pub struct TimerManager {
    primary_source: TimerSource,
    initialized: bool,
    tick_rate: u16,
    calibrated: bool,
    tsc_multiplier: f64,  // Multiplier to convert TSC ticks to nanoseconds
    hpet_address: Option<u64>,
    hpet_period: u64,
    supports_invariant_tsc: bool,
}

impl TimerManager {
    /// Create a new timer manager instance
    pub fn new() -> Self {
        Self {
            primary_source: TimerSource::PIT, // Default source
            initialized: false,
            tick_rate: DEFAULT_TICK_RATE,
            calibrated: false,
            tsc_multiplier: 0.0,
            hpet_address: None,
            hpet_period: 0,
            supports_invariant_tsc: false,
        }
    }
    
    /// Initialize the timer subsystem
    pub fn init(&mut self) -> Result<(), &'static str> {
        if self.initialized {
            return Ok(());
        }
        
        // Detect available timer sources
        self.detect_timer_capabilities();
        
        // Initialize the PIT (always available)
        self.init_pit(self.tick_rate)?;
        
        // Calibrate the TSC against the PIT
        self.calibrate_tsc();
        
        // If HPET is available, initialize it
        if let Some(addr) = self.hpet_address {
            if let Err(e) = self.init_hpet(addr) {
                #[cfg(feature = "std")]
                log::warn!("HPET initialization failed: {}", e);
                // Continue with PIT as fallback
            } else {
                // Switch to HPET as primary source if successfully initialized
                self.primary_source = TimerSource::HPET;
            }
        }
        
        // If we have a reliable TSC, prefer it for high-precision timing
        if self.supports_invariant_tsc && self.calibrated {
            self.primary_source = TimerSource::TSC;
            #[cfg(feature = "std")]
            log::info!("Using invariant TSC as primary timer source");
        }
        
        self.initialized = true;
        
        #[cfg(feature = "std")]
        log::info!("Timer subsystem initialized: {:?} @ {} Hz", 
                  self.primary_source, self.tick_rate);
        
        Ok(())
    }
    
    /// Detect available timer sources and capabilities
    fn detect_timer_capabilities(&mut self) {
        // Check for invariant TSC support
        let cpuid = raw_cpuid::CpuId::new();
        if let Some(advanced_pm) = cpuid.get_advanced_power_mgmt_info() {
            self.supports_invariant_tsc = advanced_pm.has_invariant_tsc();
        }
        
        // HPET detection typically done through ACPI tables
        // For simplicity in this implementation, we'll use fixed values if available
        #[cfg(feature = "std")]
        {
            // Fixed HPET memory address - for real implementation, get from ACPI
            self.hpet_address = Some(0xFED00000);
            self.hpet_period = 10; // 10 nanoseconds period (100 MHz)
        }
    }
    
    /// Initialize the Programmable Interval Timer
    fn init_pit(&mut self, frequency: u16) -> Result<(), &'static str> {
        let divisor: u16 = (PIT_FREQUENCY as u32 / frequency as u32) as u16;
        
        if divisor < 1 {
            return Err("PIT frequency too high");
        }
        
        // Set up PIT channel 0 in rate generator mode (mode 2)
        // with the specified divisor
        unsafe {
            let mut command_port = Port::new(PIT_COMMAND_PORT);
            let mut data_port = Port::new(PIT_CHANNEL0_PORT);
            
            // Command: channel 0, access mode: lobyte/hibyte, mode 2
            command_port.write(0x34u8);
            
            // Set divisor
            data_port.write((divisor & 0xFF) as u8);         // Low byte
            data_port.write(((divisor >> 8) & 0xFF) as u8);  // High byte
        }
        
        // Store actual tick rate
        self.tick_rate = (PIT_FREQUENCY as u32 / divisor as u32) as u16;
        
        Ok(())
    }
    
    /// Initialize the High Precision Event Timer
    fn init_hpet(&mut self, base_address: u64) -> Result<(), &'static str> {
        // HPET registers are memory-mapped
        // This is a simplified implementation
        // In a real driver, you would properly map the memory and access registers
        
        #[cfg(feature = "std")]
        log::info!("HPET initialization at address 0x{:X}", base_address);
        
        // Simulation of HPET initialization for std mode
        #[cfg(feature = "std")]
        {
            // Read capabilities register to check if HPET is present
            // In a real implementation, you would access memory-mapped registers
            
            // Enable HPET
            // For a real implementation:
            // 1. Set the main counter to 0
            // 2. Enable the main counter by setting bit 0 of the configuration register
            
            // Store the period for later use
            self.hpet_period = 10; // Assume 10ns period (100 MHz) for now
            
            return Ok(());
        }
        
        // In no_std mode, we would need to use memory-mapped I/O
        #[cfg(not(feature = "std"))]
        {
            // Implementation would involve volatile memory access to MMIO region
            // Something like:
            // let hpet = unsafe { &mut *(base_address as *mut HpetRegisters) };
            // hpet.config.write(hpet.config.read() | 1);  // Enable bit
            
            return Err("HPET not implemented in no_std mode yet");
        }
    }
    
    /// Calibrate TSC using PIT
    fn calibrate_tsc(&mut self) {
        // We'll measure TSC frequency against the known PIT frequency
        let calibration_ms = 50; // Calibrate over 50ms for better accuracy
        let ticks_to_wait = (self.tick_rate as u64 * calibration_ms) / 1000;
        
        // Start timing
        let start_tick = TICKS.load(Ordering::SeqCst);
        let start_tsc = self.read_tsc();
        
        // Wait for the specified number of ticks
        while TICKS.load(Ordering::SeqCst) - start_tick < ticks_to_wait {
            // Busy-wait
            core::hint::spin_loop();
        }
        
        // End timing
        let end_tsc = self.read_tsc();
        let tsc_delta = end_tsc - start_tsc;
        
        // Calculate TSC frequency
        let tsc_freq = tsc_delta * 1000 / calibration_ms;
        
        // Store CPU frequency in MHz
        let cpu_mhz = tsc_freq / 1_000_000;
        CPU_MHZ.store(cpu_mhz, Ordering::SeqCst);
        
        // Calculate ns multiplier
        self.tsc_multiplier = 1_000_000_000.0 / tsc_freq as f64;
        
        self.calibrated = true;
        
        #[cfg(feature = "std")]
        log::info!("TSC calibrated: CPU frequency = {} MHz", cpu_mhz);
    }
    
    /// Read the TSC value
    #[inline]
    fn read_tsc(&self) -> u64 {
        unsafe { core::arch::x86_64::_rdtsc() }
    }
    
    /// Get the current tick count
    pub fn get_ticks(&self) -> u64 {
        TICKS.load(Ordering::SeqCst)
    }
    
    /// Get the time since boot in milliseconds
    pub fn uptime_ms(&self) -> u64 {
        let ticks = self.get_ticks();
        (ticks * 1000) / self.tick_rate as u64
    }
    
    /// Get the time since boot in microseconds (high precision)
    pub fn uptime_us(&self) -> u64 {
        match self.primary_source {
            TimerSource::TSC => {
                if self.calibrated {
                    let tsc = self.read_tsc();
                    (tsc as f64 * self.tsc_multiplier / 1000.0) as u64
                } else {
                    let ticks = self.get_ticks();
                    (ticks * 1_000_000) / self.tick_rate as u64
                }
            },
            TimerSource::HPET => {
                // In a real driver, read the HPET counter
                // For now, fall back to PIT-based timing
                let ticks = self.get_ticks();
                (ticks * 1_000_000) / self.tick_rate as u64
            },
            _ => {
                // Use PIT ticks with microsecond conversion
                let ticks = self.get_ticks();
                (ticks * 1_000_000) / self.tick_rate as u64
            }
        }
    }
    
    /// Sleep for the specified number of milliseconds
    pub fn sleep(&self, ms: u64) {
        let target_tick = self.get_ticks() + (ms * self.tick_rate as u64) / 1000;
        
        #[cfg(feature = "std")]
        {
            // In std mode, use std::thread::sleep for better power efficiency
            std::thread::sleep(std::time::Duration::from_millis(ms));
            return;
        }
        
        // Busy-wait until we reach the target tick
        while self.get_ticks() < target_tick {
            core::hint::spin_loop();
        }
    }
    
    /// Get a high-precision timestamp in nanoseconds
    /// Useful for game frame timing and performance measurements
    pub fn timestamp_ns(&self) -> u64 {
        match self.primary_source {
            TimerSource::TSC => {
                if self.calibrated {
                    let tsc = self.read_tsc();
                    (tsc as f64 * self.tsc_multiplier) as u64
                } else {
                    let ticks = self.get_ticks();
                    (ticks * 1_000_000_000) / self.tick_rate as u64
                }
            },
            TimerSource::HPET => {
                // In a real driver, read the HPET counter and convert using period
                // For now, fall back to PIT-based timing
                let ticks = self.get_ticks();
                (ticks * 1_000_000_000) / self.tick_rate as u64
            },
            _ => {
                // Use PIT ticks with nanosecond conversion
                let ticks = self.get_ticks();
                (ticks * 1_000_000_000) / self.tick_rate as u64
            }
        }
    }
    
    /// Get time difference in microseconds (for frame timing)
    pub fn time_diff_us(&self, start: u64, end: u64) -> u64 {
        end.saturating_sub(start)
    }
    
    /// Calculate frames per second based on frame time
    pub fn calculate_fps(&self, frame_time_us: u64) -> f64 {
        if frame_time_us == 0 {
            return 0.0;
        }
        1_000_000.0 / frame_time_us as f64
    }
}
/// Timer interrupt handler
pub extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Increment tick counter
    TICKS.fetch_add(1, Ordering::SeqCst);
    
    // End Of Interrupt signal
    #[cfg(not(feature = "std"))]
    unsafe {
        // Send EOI to PIC
        crate::kernel::interrupts::end_of_interrupt();
    }
}

/// Initialize the timer subsystem
pub fn init() -> Result<(), &'static str> {
    let mut manager = TIMER_MANAGER.lock();
    manager.init()
}

/// Sleep for the specified number of milliseconds
pub fn sleep(ms: u64) {
    let manager = TIMER_MANAGER.lock();
    manager.sleep(ms);
}

/// Start the system timer with game loop and task scheduling capabilities
pub fn start_system_timer() {
    // Make sure the timer system is initialized
    if let Err(e) = init() {
        #[cfg(feature = "log")]
        log::error!("Failed to initialize timer: {}", e);
        return;
    }

    // Start PIT timer for system-wide timing
    #[cfg(not(feature = "std"))]
    {
        // Ensure timer interrupts are enabled
        let mut manager = TIMER_MANAGER.lock();
        
        // Configure a higher precision timer rate for games (e.g., 1000Hz)
        if !manager.initialized {
            if let Err(_) = manager.init_pit(1000) {
                #[cfg(feature = "log")]
                log::error!("Failed to configure PIT for high precision timing");
            }
            manager.initialized = true;
        }
        
        // Register the timer interrupt handler with the IDT
        unsafe {
            crate::kernel::interrupts::register_timer_handler(timer_interrupt_handler);
        }
        
        // Enable timer interrupts
        unsafe {
            crate::kernel::interrupts::enable_timer_interrupt();
        }
        
        #[cfg(feature = "log")]
        log::info!("System timer started with {} Hz tick rate", manager.tick_rate);
        
        // Set up the task scheduler for periodic tasks
        init_task_scheduler();
    }
    
    #[cfg(feature = "std")]
    {
        // In std environment, start a background thread for timer simulation
        std::thread::spawn(|| {
            let tick_duration = std::time::Duration::from_millis(1);
            loop {
                // Increment the system tick counter
                TICKS.fetch_add(1, Ordering::SeqCst);
                
                // Process any scheduled tasks
                process_scheduled_tasks();
                
                // Sleep to simulate the timer interrupt
                std::thread::sleep(tick_duration);
            }
        });
        
        #[cfg(feature = "log")]
        log::info!("System timer started in simulation mode");
    }
    
    // Initialize the performance counter for high-precision game timing
    init_performance_counter();
    
    #[cfg(feature = "log")]
    log::info!("Game loop timing system initialized");
}

/// Get uptime in milliseconds
pub fn uptime_ms() -> u64 {
    let manager = TIMER_MANAGER.lock();
    manager.uptime_ms()
}

/// Get a high-precision timestamp in nanoseconds
pub fn timestamp_ns() -> u64 {
    let manager = TIMER_MANAGER.lock();
    manager.timestamp_ns()
}

/// Get the CPU frequency in MHz
pub fn get_cpu_mhz() -> u64 {
    CPU_MHZ.load(Ordering::SeqCst)
}

pub fn tick() {
    // This function is called by the timer interrupt handler
    // It can be used to update game state or perform periodic tasks
}

/// Measure execution time of a function in microseconds
pub fn measure_execution_time<F, R>(f: F) -> (R, u64)
where
    F: FnOnce() -> R,
{
    let manager = TIMER_MANAGER.lock();
    let start = manager.timestamp_ns();
    let result = f();
    let end = manager.timestamp_ns();
    let elapsed_us = (end - start) / 1000;
    
    (result, elapsed_us)
}

/// A simple stopwatch for timing operations
pub struct Stopwatch {
    start_time: u64,
    elapsed_us: u64,
    running: bool,
}

impl Stopwatch {
    /// Create a new stopwatch (not started)
    pub fn new() -> Self {
        Self {
            start_time: 0,
            elapsed_us: 0,
            running: false,
        }
    }
    
    /// Start the stopwatch
    pub fn start(&mut self) {
        if !self.running {
            self.start_time = timestamp_ns();
            self.running = true;
        }
    }
    
    /// Stop the stopwatch
    pub fn stop(&mut self) -> u64 {
        if self.running {
            let now = timestamp_ns();
            self.elapsed_us = (now - self.start_time) / 1000;
            self.running = false;
        }
        self.elapsed_us
    }
    
    /// Reset the stopwatch
    pub fn reset(&mut self) {
        self.elapsed_us = 0;
        if self.running {
            self.start_time = timestamp_ns();
        }
    }
    
    /// Get elapsed time in microseconds
    pub fn elapsed_us(&self) -> u64 {
        if self.running {
            let now = timestamp_ns();
            (now - self.start_time) / 1000
        } else {
            self.elapsed_us
        }
    }
    
    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed_us() / 1000
    }
}

/// Frame timing for games
pub struct FrameTimer {
    last_frame_time: u64,
    frame_count: u64,
    fps: f64,
    frame_times: [u64; 60], // Store last 60 frame times for averaging
    frame_time_index: usize,
    one_second_timer: u64,
}

impl FrameTimer {
    /// Create a new frame timer
    pub fn new() -> Self {
        Self {
            last_frame_time: timestamp_ns(),
            frame_count: 0,
            fps: 0.0,
            frame_times: [0; 60],
            frame_time_index: 0,
            one_second_timer: timestamp_ns(),
        }
    }
    
    /// Begin a new frame
    pub fn begin_frame(&mut self) {
        self.last_frame_time = timestamp_ns();
    }
    
    /// End current frame and calculate timing
    pub fn end_frame(&mut self) -> FrameTiming {
        let now = timestamp_ns();
        let frame_time_ns = now - self.last_frame_time;
        let frame_time_us = frame_time_ns / 1000;
        
        // Store frame time in circular buffer
        self.frame_times[self.frame_time_index] = frame_time_us;
        self.frame_time_index = (self.frame_time_index + 1) % 60;
        
        // Update frame count
        self.frame_count += 1;
        
        // Calculate FPS once per second
        if now - self.one_second_timer >= 1_000_000_000 {
            // Calculate average FPS
            let frames_this_second = self.frame_count;
            self.fps = frames_this_second as f64;
            
            // Reset counters
            self.frame_count = 0;
            self.one_second_timer = now;
        }
        
        // Calculate average frame time over last 60 frames
        let mut sum = 0;
        let mut count = 0;
        for &time in &self.frame_times {
            if time > 0 {
                sum += time;
                count += 1;
            }
        }
        
        let avg_frame_time = if count > 0 { sum / count } else { frame_time_us };
        
        FrameTiming {
            frame_time_us,
            fps: self.fps,
            avg_frame_time_us: avg_frame_time,
        }
    }
    
    /// Get the current FPS
    pub fn get_fps(&self) -> f64 {
        self.fps
    }
}

/// Frame timing information
#[derive(Debug, Clone, Copy)]
pub struct FrameTiming {
    pub frame_time_us: u64,   // Time taken for last frame (microseconds)
    pub fps: f64,             // Frames per second
    pub avg_frame_time_us: u64, // Average frame time over last 60 frames
}

/// Process scheduled tasks at their appropriate intervals
#[inline]
fn process_scheduled_tasks() {
    // Access the global task list (protected by mutex)
    if let Some(mut tasks) = SCHEDULED_TASKS.try_lock() {
        let current_tick = TICKS.load(Ordering::SeqCst);
        
        // Process all due tasks
        for task in tasks.iter_mut() {
            if task.enabled && current_tick >= task.next_execution {
                // Execute the task callback
                (task.callback)();
                
                // Schedule next execution
                if task.interval > 0 {
                    task.next_execution = current_tick + task.interval;
                } else {
                    // One-shot task, disable it
                    task.enabled = false;
                }
            }
        }
    }
}

/// Initialize the task scheduler for periodic events
fn init_task_scheduler() {
    lazy_static! {
        /// List of scheduled tasks to run at specific intervals
        static ref SCHEDULED_TASKS: Mutex<Vec<ScheduledTask>> = Mutex::new(Vec::with_capacity(32));
    }
    
    // Initialize with some system tasks
    let mut tasks = SCHEDULED_TASKS.lock();
    
    // Add task for updating system stats every second
    tasks.push(ScheduledTask {
        name: "system_stats_update",
        interval: DEFAULT_TICK_RATE as u64, // 1 second
        next_execution: DEFAULT_TICK_RATE as u64, // Start after 1 second
        callback: || {
            update_system_stats();
        },
        enabled: true,
    });
    
    // Add task for polling input devices at 100Hz
    tasks.push(ScheduledTask {
        name: "input_polling",
        interval: DEFAULT_TICK_RATE as u64 / 100, // 10ms (100Hz)
        next_execution: DEFAULT_TICK_RATE as u64 / 100, // Start after 10ms
        callback: || {
            poll_input_devices();
        },
        enabled: true,
    });
}

/// Initialize high-precision performance counter for game timing
fn init_performance_counter() {
    // Set up high-precision timing using the best available timer source
    let mut manager = TIMER_MANAGER.lock();
    
    // Calibrate TSC if available for maximum precision
    if !manager.calibrated {
        manager.calibrate_tsc();
    }
    
    // Initialize game loop timing data
    GAME_TIMING.lock().last_frame_time = manager.timestamp_ns();
    
    #[cfg(feature = "log")]
    {
        let source_name = match manager.primary_source {
            TimerSource::TSC => "Time Stamp Counter",
            TimerSource::HPET => "High Precision Event Timer",
            TimerSource::PIT => "Programmable Interval Timer",
            TimerSource::APIC => "APIC Timer",
        };
        
        log::info!("Game performance counter initialized using {} at {} MHz", 
                 source_name, CPU_MHZ.load(Ordering::SeqCst));
    }
}

/// Update system statistics
fn update_system_stats() {
    #[cfg(feature = "log")]
    {
        let manager = TIMER_MANAGER.lock();
        let uptime_secs = manager.uptime_ms() / 1000;
        log::trace!("System uptime: {}s, CPU: {} MHz", uptime_secs, CPU_MHZ.load(Ordering::SeqCst));
    }
}

/// Poll input devices for game controllers, keyboard, etc.
fn poll_input_devices() {
    // Call into input driver to poll devices
    #[cfg(not(feature = "std"))]
    {
        // Check for new input events
        if let Some(input) = crate::kernel::drivers::input::poll_events() {
            // Process input events
            process_input_event(input);
        }
    }
}

/// Process a game input event
fn process_input_event(input: InputEvent) {
    // Call registered input handlers
    if let Some(handlers) = INPUT_HANDLERS.try_lock() {
        for handler in handlers.iter() {
            (handler)(input);
        }
    }
}

/// A scheduled task in the timer system
struct ScheduledTask {
    /// Name of the task
    name: &'static str,
    /// Interval in system ticks (0 for one-shot tasks)
    interval: u64,
    /// Next execution time in system ticks
    next_execution: u64,
    /// Function to call when the task is due
    callback: fn(),
    /// Whether the task is enabled
    enabled: bool,
}

/// Game timing information
struct GameTiming {
    /// Timestamp of the last frame
    last_frame_time: u64,
    /// Target frame rate (0 for unlimited)
    target_fps: u64,
    /// Minimum frame time in ns (based on target FPS)
    min_frame_time_ns: u64,
    /// Fixed time step for physics (ns)
    fixed_time_step_ns: u64,
    /// Accumulated time for fixed updates
    accumulated_time_ns: u64,
}

lazy_static! {
    /// Global game timing information
    static ref GAME_TIMING: Mutex<GameTiming> = Mutex::new(GameTiming {
        last_frame_time: 0,
        target_fps: 60,
        min_frame_time_ns: 16_666_667, // 60 FPS (1s / 60 = 16.67ms)
        fixed_time_step_ns: 16_666_667, // 60Hz physics updates
        accumulated_time_ns: 0,
    });
    
    /// Input event handlers
    static ref INPUT_HANDLERS: Mutex<Vec<fn(InputEvent)>> = Mutex::new(Vec::new());
    
    /// Scheduled tasks
    static ref SCHEDULED_TASKS: Mutex<Vec<ScheduledTask>> = Mutex::new(Vec::new());
}

/// Represents an input event
#[derive(Clone, Copy, Debug)]
pub struct InputEvent {
    // Input event fields would go here
    // For example:
    pub event_type: InputEventType,
    pub device_id: u8,
    pub data: [u32; 4],  // Generic data array for different event types
}

/// Types of input events
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEventType {
    KeyPress,
    KeyRelease,
    MouseMove,
    MouseButton,
    GamepadButton,
    GamepadAxis,
    Touch,
    Gesture,
}

/// Schedule a one-time task to run after the specified delay
pub fn schedule_task(name: &'static str, delay_ms: u64, callback: fn()) -> Result<(), &'static str> {
    let ticks_delay = (delay_ms * DEFAULT_TICK_RATE as u64) / 1000;
    let current_tick = TICKS.load(Ordering::SeqCst);
    
    if let Some(mut tasks) = SCHEDULED_TASKS.try_lock() {
        tasks.push(ScheduledTask {
            name,
            interval: 0, // One-time task
            next_execution: current_tick + ticks_delay,
            callback,
            enabled: true,
        });
        Ok(())
    } else {
        Err("Failed to acquire task list lock")
    }
}

/// Schedule a periodic task to run at the specified interval
pub fn schedule_periodic_task(name: &'static str, interval_ms: u64, callback: fn()) -> Result<(), &'static str> {
    let ticks_interval = (interval_ms * DEFAULT_TICK_RATE as u64) / 1000;
    let current_tick = TICKS.load(Ordering::SeqCst);
    
    if let Some(mut tasks) = SCHEDULED_TASKS.try_lock() {
        tasks.push(ScheduledTask {
            name,
            interval: ticks_interval,
            next_execution: current_tick + ticks_interval,
            callback,
            enabled: true,
        });
        Ok(())
    } else {
        Err("Failed to acquire task list lock")
    }
}

/// Register an input handler function
pub fn register_input_handler(handler: fn(InputEvent)) {
    if let Some(mut handlers) = INPUT_HANDLERS.try_lock() {
        handlers.push(handler);
    }
}

/// Set target frame rate for the game loop
pub fn set_target_fps(fps: u64) {
    if fps == 0 {
        // Unlimited FPS
        GAME_TIMING.lock().target_fps = 0;
        GAME_TIMING.lock().min_frame_time_ns = 0;
    } else {
        GAME_TIMING.lock().target_fps = fps;
        GAME_TIMING.lock().min_frame_time_ns = 1_000_000_000 / fps;
    }
}

/// Wait for next frame (respecting target FPS)
pub fn wait_for_next_frame() -> (u64, f64) {
    let mut timing = GAME_TIMING.lock();
    let now = timestamp_ns();
    let elapsed = now - timing.last_frame_time;
    
    // If we have a target frame rate, wait until it's time for the next frame
    if timing.target_fps > 0 && elapsed < timing.min_frame_time_ns {
        let wait_time = timing.min_frame_time_ns - elapsed;
        
        // For very short waits, use spin loop for precision
        if wait_time < 1_000_000 { // Less than 1ms
            let end_time = now + wait_time;
            while timestamp_ns() < end_time {
                core::hint::spin_loop();
            }
        } else {
            // For longer waits, use sleep
            sleep(wait_time / 1_000_000);
        }
    }
    
    // Get final frame time
    let final_time = timestamp_ns();
    let frame_time = final_time - timing.last_frame_time;
    timing.last_frame_time = final_time;
    
    // Calculate FPS
    let fps = if frame_time > 0 {
        1_000_000_000.0 / frame_time as f64
    } else {
        0.0
    };
    
    (frame_time / 1_000, fps) // Return frame time in microseconds and FPS
}

/// Run fixed time step updates (for physics, etc.)
pub fn run_fixed_updates(update_fn: fn(f64)) -> usize {
    let mut timing = GAME_TIMING.lock();
    let now = timestamp_ns();
    let frame_time = now - timing.last_frame_time;
    
    // Add elapsed time to accumulator
    timing.accumulated_time_ns += frame_time;
    
    // Run as many fixed updates as needed
    let mut updates_run = 0;
    while timing.accumulated_time_ns >= timing.fixed_time_step_ns {
        // Run the update with delta time in seconds
        update_fn(timing.fixed_time_step_ns as f64 / 1_000_000_000.0);
        
        // Subtract fixed step from accumulator
        timing.accumulated_time_ns -= timing.fixed_time_step_ns;
        updates_run += 1;
    }
    
    // Return number of updates run
    updates_run
}

pub fn get_system_time_ms() -> u64 {
    #[cfg(feature = "std")]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
    
    #[cfg(not(feature = "std"))]
    {
        // Use system timer or PIT/TSC for timestamp
        crate::kernel::drivers::timer::get_timestamp_ms()
    }
}

pub fn get_timestamp_ms() -> u64 {
    let manager = TIMER_MANAGER.lock();
    manager.uptime_ms()
}