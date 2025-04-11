//! CPU subsystem for OS Gaming
//! 
//! This module provides CPU detection, feature checking, power management,
//! and performance monitoring capabilities optimized for gaming.

// Declare submodules
pub mod identification;
pub mod features;
pub mod power;
pub mod performance;

// Re-export commonly used items for easier access
pub use identification::{get_cpu_info, CpuInfo};
pub use features::{CpuFeature, has_feature, has_required_features};
pub use power::{set_performance_mode, set_balanced_mode, set_power_saving_mode};
pub use performance::{start_monitoring, stop_monitoring, read_performance_data};

use crate::kernel::interrupts;

/// Initialize CPU subsystem
pub fn init() -> Result<(), &'static str> {
    // Identify CPU and its capabilities
    let cpu_info = identification::detect_cpu();
    
    #[cfg(feature = "std")]
    log::info!("CPU detected: {} {}", cpu_info.vendor_id, cpu_info.brand_string);
    
    // Check for required CPU features
    if !features::has_required_features() {
        return Err("CPU does not support required features for OS Gaming");
    }
    
    // Initialize power management
    power::init()?;
    
    // Initialize performance monitoring
    performance::init()?;
    
    Ok(())
}

/// Configure gaming-optimized CPU settings
pub fn configure_gaming_mode() -> Result<(), &'static str> {
    // Disable power saving features for maximum performance
    power::set_performance_mode()?;
    
    // Configure CPU caches for gaming workloads
    features::optimize_cache_for_gaming();
    
    // Start performance monitoring
    performance::start_monitoring();
    
    #[cfg(feature = "std")]
    log::info!("CPU configured for gaming performance");
    
    Ok(())
}

/// Return current CPU status information
pub fn get_status() -> CpuStatus {
    let perf_data = performance::read_performance_data();
    let current_freq = power::get_current_frequency();
    
    CpuStatus {
        utilization: calculate_utilization(&perf_data),
        frequency: current_freq,
        temperature: None,  // Would need temperature monitoring
        perf_data,
    }
}

/// CPU status structure for monitoring and display
#[derive(Debug)]
pub struct CpuStatus {
    pub utilization: f64,          // CPU utilization percentage
    pub frequency: Option<u64>,    // Current frequency in Hz
    pub temperature: Option<f32>,  // CPU temperature in Celsius
    pub perf_data: performance::PerfData,  // Performance counters
}

/// Calculate CPU utilization from performance data
fn calculate_utilization(perf_data: &performance::PerfData) -> f64 {
    // This is a simplified calculation - real implementation would
    // compare active vs idle cycles
    if perf_data.cycles == 0 {
        return 0.0;
    }
    
    let ipc = performance::calculate_ipc(perf_data);
    // Scale IPC to a percentage (assuming max efficiency ~3.0 IPC)
    (ipc / 3.0).min(1.0) * 100.0
}