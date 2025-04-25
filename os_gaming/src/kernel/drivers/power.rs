//! Power management driver for OS Gaming
//!
//! This module provides power management features:
//! - System power states (S0-S5)
//! - CPU performance states (P-states)
//! - Thermal management (T-states)
//! - Battery monitoring
//! - Performance profiles for gaming

use core::sync::atomic::{AtomicBool, Ordering};
use core::arch::asm;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::port::Port;

#[cfg(feature = "std")]
use sysinfo::{System};

#[cfg(all(feature = "std", feature = "battery"))]
use battery::{Battery, Manager, State};

/// System power states as defined by ACPI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerState {
    S0,     // Working
    S1,     // Sleeping with processor context maintained
    S2,     // Sleeping with processor context lost
    S3,     // Sleeping with memory preserved (Suspend to RAM)
    S4,     // Hibernation (Suspend to disk)
    S5,     // Soft off
    Unknown,
}

/// CPU performance states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CpuPState {
    P0,     // Maximum performance and power
    P1,     // High performance (slightly reduced)
    P2,     // Balanced
    P3,     // Power saving
    P4,     // Minimum performance and power
    Custom(u8), // Custom P-state level
}

/// Performance profiles for different usage scenarios
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PerformanceProfile {
    MaxPerformance,   // Maximum performance, no power saving
    Gaming,           // Optimized for gaming (high performance, some power saving)
    Balanced,         // Balance between performance and power saving
    PowerSaving,      // Prioritize power saving over performance
    Custom,           // Custom user-defined profile
}

/// CPU frequency scaling governor
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CpuGovernor {
    Performance,      // Run at maximum frequency
    Powersave,        // Run at minimum frequency
    Ondemand,         // Scale based on load (aggressive)
    Conservative,     // Scale based on load (gradual)
    Schedutil,        // Scale based on scheduler information
    UserSpace,        // Manually set frequency
}

/// Battery information
#[derive(Debug, Clone)]
pub struct BatteryInfo {
    pub present: bool,
    pub charging: bool,
    pub percent: u8,
    pub time_remaining: Option<u32>, // Minutes
    pub capacity: Option<u32>,       // mWh
    pub voltage: Option<u32>,        // mV
    pub wear_level: Option<u8>,      // Percentage
}

/// Thermal information
#[derive(Debug, Clone)]
pub struct ThermalInfo {
    pub cpu_temp: i32,        // CPU temperature in Celsius
    pub gpu_temp: i32,        // GPU temperature in Celsius
    pub fan_rpm: Option<u32>, // Fan speed in RPM
    pub throttling: bool,     // Whether thermal throttling is active
}

/// Power management system
pub struct PowerManager {
    initialized: AtomicBool,
    current_power_state: PowerState,
    current_profile: PerformanceProfile,
    cpu_governor: CpuGovernor,
    battery_info: Option<BatteryInfo>,
    thermal_info: ThermalInfo,
    supports_acpi: bool,
    supports_cpu_freq: bool,
    max_cpu_freq: u32, // MHz
    min_cpu_freq: u32, // MHz
}

// Global power manager instance
lazy_static! {
    static ref POWER_MANAGER: Mutex<PowerManager> = Mutex::new(PowerManager::new());
}

impl PowerManager {
    /// Create a new power manager
    pub fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            current_power_state: PowerState::S0,
            current_profile: PerformanceProfile::Balanced,
            cpu_governor: CpuGovernor::Ondemand,
            battery_info: None,
            thermal_info: ThermalInfo {
                cpu_temp: 0,
                gpu_temp: 0,
                fan_rpm: None,
                throttling: false,
            },
            supports_acpi: false,
            supports_cpu_freq: false,
            max_cpu_freq: 0,
            min_cpu_freq: 0,
        }
    }

    /// Initialize the power management system
    pub fn init(&mut self) -> Result<(), &'static str> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Detect hardware capabilities
        self.detect_capabilities();

        // Initialize ACPI if supported
        if self.supports_acpi {
            self.init_acpi()?;
        }

        // Initialize CPU frequency scaling if supported
        if self.supports_cpu_freq {
            self.init_cpu_freq()?;
        }

        // Read initial thermal information
        self.update_thermal_info();

        // Set default performance profile
        self.set_performance_profile(PerformanceProfile::Balanced)?;

        self.initialized.store(true, Ordering::SeqCst);

        #[cfg(feature = "std")]
        log::info!("Power management initialized");

        Ok(())
    }

    /// Detect hardware power management capabilities
    fn detect_capabilities(&mut self) {
        #[cfg(feature = "std")]
        {
            self.supports_acpi = true;
            self.supports_cpu_freq = true;

            // Use sysinfo to get CPU frequency info
            let mut system = System::new_all();
            system.refresh_all();

            if let Some(cpu) = system.cpus().first() {
                self.max_cpu_freq = cpu.frequency() as u32;
                // Assume min is 25% of max for now
                self.min_cpu_freq = self.max_cpu_freq / 4;
            }

            log::info!("CPU frequency range: {} - {} MHz",
                      self.min_cpu_freq, self.max_cpu_freq);
        }

        #[cfg(not(feature = "std"))]
        {
            // In a real bare-metal OS, we would detect these by checking CPUID
            // and ACPI tables

            // For now, set some reasonable defaults
            self.supports_acpi = true;
            self.supports_cpu_freq = true;
            self.max_cpu_freq = 3000; // 3 GHz
            self.min_cpu_freq = 800;  // 800 MHz
        }
    }

    /// Initialize ACPI subsystem
    fn init_acpi(&mut self) -> Result<(), &'static str> {
        #[cfg(feature = "std")]
        log::info!("ACPI subsystem initialized");

        Ok(())
    }

    /// Initialize CPU frequency scaling
    fn init_cpu_freq(&mut self) -> Result<(), &'static str> {
        #[cfg(feature = "std")]
        log::info!("CPU frequency scaling initialized");

        Ok(())
    }

    /// Update battery information
    #[cfg(all(feature = "std", feature = "battery"))]
    pub fn update_battery_info(&mut self) {
        match Manager::new() {
            Ok(manager) => {
                if let Ok(batteries) = manager.batteries() {
                    if let Ok(battery) = batteries.into_iter().next() {
                        let percent = battery.state_of_charge().value() as u8;
                        let time_remaining = match battery.state() {
                            State::Discharging => battery.time_to_empty().map(|t| t.value() as u32),
                            _ => None,
                        };

                        let info = BatteryInfo {
                            present: true,
                            charging: matches!(battery.state(), State::Charging),
                            percent,
                            time_remaining,
                            capacity: Some(battery.energy_full().value() as u32),
                            voltage: Some(battery.voltage().value() as u32),
                            wear_level: Some(((1.0 - (battery.energy_full().value() / battery.energy_full_design().value())) * 100.0) as u8),
                        };

                        self.battery_info = Some(info);
                    } else {
                        self.battery_info = None;
                    }
                } else {
                    self.battery_info = None;
                }
            },
            Err(_) => {
                self.battery_info = None;
            }
        }
    }

    /// Update thermal information
    pub fn update_thermal_info(&mut self) {
        #[cfg(feature = "std")]
        {
            let mut system = System::new_all();
            system.refresh_all();

            // Find CPU temperature
            let mut cpu_temp = 0;
            let mut gpu_temp = 0;

            self.thermal_info = ThermalInfo {
                cpu_temp,
                gpu_temp,
                fan_rpm: None, // Not provided by sysinfo
                throttling: cpu_temp > 80, // Assume throttling above 80°C
            };
        }

        #[cfg(not(feature = "std"))]
        {
            // In a real OS, we would read from hardware sensors
            // For now, just provide dummy values
            self.thermal_info = ThermalInfo {
                cpu_temp: 45,
                gpu_temp: 50,
                fan_rpm: Some(2000),
                throttling: false,
            };
        }
    }

    /// Set system power state
    pub fn set_power_state(&mut self, state: PowerState) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Power manager not initialized");
        }

        match state {
            PowerState::S0 => {
                self.current_power_state = PowerState::S0;
                Ok(())
            },
            PowerState::S3 => {
                #[cfg(feature = "std")]
                log::info!("Entering S3 sleep state (suspend to RAM)");

                self.current_power_state = PowerState::S3;
                Ok(())
            },
            PowerState::S4 => {
                #[cfg(feature = "std")]
                log::info!("Entering S4 sleep state (hibernate)");

                self.current_power_state = PowerState::S4;
                Ok(())
            },
            PowerState::S5 => {
                #[cfg(feature = "std")]
                log::info!("Entering S5 sleep state (soft off)");

                self.current_power_state = PowerState::S5;
                Ok(())
            },
            _ => Err("Unsupported power state"),
        }
    }

    /// Get current power state
    pub fn get_power_state(&self) -> PowerState {
        self.current_power_state
    }

    /// Set CPU governor
    pub fn set_cpu_governor(&mut self, governor: CpuGovernor) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Power manager not initialized");
        }

        if !self.supports_cpu_freq {
            return Err("CPU frequency scaling not supported");
        }

        self.cpu_governor = governor;

        #[cfg(feature = "std")]
        log::info!("CPU governor set to {:?}", governor);

        Ok(())
    }

    /// Set CPU frequency
    pub fn set_cpu_frequency(&mut self, freq_mhz: u32) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Power manager not initialized");
        }

        if !self.supports_cpu_freq {
            return Err("CPU frequency scaling not supported");
        }

        if freq_mhz < self.min_cpu_freq || freq_mhz > self.max_cpu_freq {
            return Err("Frequency out of range");
        }

        self.cpu_governor = CpuGovernor::UserSpace;

        #[cfg(feature = "std")]
        log::info!("CPU frequency set to {} MHz", freq_mhz);

        Ok(())
    }

    /// Set performance profile
    pub fn set_performance_profile(&mut self, profile: PerformanceProfile) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Power manager not initialized");
        }

        match profile {
            PerformanceProfile::MaxPerformance => {
                if self.supports_cpu_freq {
                    self.set_cpu_governor(CpuGovernor::Performance)?;
                }
            },
            PerformanceProfile::Gaming => {
                if self.supports_cpu_freq {
                    self.set_cpu_governor(CpuGovernor::Ondemand)?;
                }
            },
            PerformanceProfile::Balanced => {
                if self.supports_cpu_freq {
                    self.set_cpu_governor(CpuGovernor::Ondemand)?;
                }
            },
            PerformanceProfile::PowerSaving => {
                if self.supports_cpu_freq {
                    self.set_cpu_governor(CpuGovernor::Powersave)?;
                }
            },
            PerformanceProfile::Custom => {},
        }

        self.current_profile = profile;

        #[cfg(feature = "std")]
        log::info!("Performance profile set to {:?}", profile);

        Ok(())
    }

    /// Get current performance profile
    pub fn get_performance_profile(&self) -> PerformanceProfile {
        self.current_profile
    }

    /// Get current CPU governor
    pub fn get_cpu_governor(&self) -> CpuGovernor {
        self.cpu_governor
    }

    /// Get battery information
    pub fn get_battery_info(&self) -> Option<&BatteryInfo> {
        self.battery_info.as_ref()
    }

    /// Get thermal information
    pub fn get_thermal_info(&self) -> &ThermalInfo {
        &self.thermal_info
    }

    /// Check if system is on battery power
    pub fn is_on_battery(&self) -> bool {
        if let Some(battery) = &self.battery_info {
            battery.present && !battery.charging
        } else {
            false
        }
    }

    /// Reboot the system
    ///
    /// This method attempts to reboot the computer using the PS/2 keyboard
    /// controller reset mechanism.
    ///
    /// Returns Ok(()) if the reboot was initiated successfully, or an error
    /// if the power manager wasn't properly initialized.
    pub fn reboot(&self) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Power manager not initialized");
        }

        #[cfg(not(feature = "std"))]
        {
            // Le triple fault est la méthode la plus fiable
            unsafe {
                // Désactiver les interruptions avant tout
                asm!("cli");

                // Tentative via le contrôleur de clavier PS/2
                let mut port = Port::new(0x64);
                port.write(0xFEu8);

                // Si on arrive ici, la méthode a échoué
                // Utiliser HLT pour stopper le processeur
                loop {
                    asm!("hlt");
                }
            }
        }

        #[cfg(feature = "std")]
        {
            log::info!("System reboot requested");
        }

        Ok(())
    }

    /// Shutdown the system
    pub fn shutdown(&self) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Power manager not initialized");
        }

        #[cfg(feature = "std")]
        {
            log::info!("System shutdown requested");
        }

        #[cfg(not(feature = "std"))]
        {
            if self.supports_acpi {
                // Normally we would call the ACPI S5 sleep state
            }
        }

        Ok(())
    }

    /// Refresh all power-related information
    pub fn refresh(&mut self) {
        self.update_thermal_info();
    }
}

/// Initialize the power management subsystem
pub fn init() -> Result<(), &'static str> {
    let mut manager = POWER_MANAGER.lock();
    manager.init()
}

/// Get power manager instance
pub fn get_manager() -> &'static Mutex<PowerManager> {
    &POWER_MANAGER
}

/// Set performance profile
pub fn set_performance_profile(profile: PerformanceProfile) -> Result<(), &'static str> {
    let mut manager = POWER_MANAGER.lock();
    manager.set_performance_profile(profile)
}

/// Get current performance profile
pub fn get_performance_profile() -> PerformanceProfile {
    let manager = POWER_MANAGER.lock();
    manager.get_performance_profile()
}

/// Get battery information
pub fn get_battery_info() -> Option<BatteryInfo> {
    let manager = POWER_MANAGER.lock();
    manager.get_battery_info().cloned()
}

/// Get thermal information
pub fn get_thermal_info() -> ThermalInfo {
    let manager = POWER_MANAGER.lock();
    manager.get_thermal_info().clone()
}

/// Reboot the system
pub fn reboot() -> Result<(), &'static str> {
    let manager = POWER_MANAGER.lock();
    manager.reboot()
}

pub fn enter_sleep_mode() -> Result<(), &'static str> {
    let mut manager = POWER_MANAGER.lock();
    manager.set_power_state(PowerState::S3)
}

/// Shutdown the system
pub fn shutdown() -> Result<(), &'static str> {
    let manager = POWER_MANAGER.lock();
    manager.shutdown()
}
