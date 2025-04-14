use super::drivers::{self, hdmi::GamingRequirements};
use crate::Config;
use lazy_static::lazy_static;
use spin::Mutex;

/// Boot status tracking
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BootStatus {
    NotStarted,
    CPUInitializing,
    MemoryInitializing,
    DisplayInitializing,
    StorageInitializing,
    InputInitializing,
    NetworkInitializing,
    SoundInitializing,
    FilesystemInitializing,
    PowerInitializing,
    BootCompleted,
    Failed(u32), // Error code
}

/// Boot configuration parameters
pub struct BootConfig {
    /// Memory map information
    pub memory_map: Option<&'static [u8]>,
    /// Command line arguments
    pub cmdline: Option<&'static str>,
    /// Initial display width
    pub display_width: Option<u32>,
    /// Initial display height
    pub display_height: Option<u32>,
    /// Initial display refresh rate
    pub refresh_rate: Option<u32>,
}

impl Default for BootConfig {
    fn default() -> Self {
        Self {
            memory_map: None,
            cmdline: None,
            display_width: Some(1920),
            display_height: Some(1080),
            refresh_rate: Some(60),
        }
    }
}

lazy_static! {
    /// Global boot status that can be checked from anywhere
    static ref BOOT_STATUS: Mutex<BootStatus> = Mutex::new(BootStatus::NotStarted);
}

/// Get current boot status
pub fn get_boot_status() -> BootStatus {
    *BOOT_STATUS.lock()
}

/// Set boot status
fn set_boot_status(status: BootStatus) {
    #[cfg(feature = "std")]
    match status {
        BootStatus::NotStarted => log::info!("Boot process starting"),
        BootStatus::CPUInitializing => log::info!("Initializing CPU"),
        BootStatus::MemoryInitializing => log::info!("Initializing memory subsystem"),
        BootStatus::DisplayInitializing => log::info!("Initializing display subsystem"),
        BootStatus::StorageInitializing => log::info!("Initializing storage subsystem"),
        BootStatus::InputInitializing => log::info!("Initializing input devices"),
        BootStatus::NetworkInitializing => log::info!("Initializing network subsystem"),
        BootStatus::SoundInitializing => log::info!("Initializing sound subsystem"),
        BootStatus::FilesystemInitializing => log::info!("Initializing filesystem"),
        BootStatus::PowerInitializing => log::info!("Initializing power management"),
        BootStatus::BootCompleted => log::info!("Boot process completed successfully"),
        BootStatus::Failed(code) => log::error!("Boot process failed with error code: {}", code),
    }
    
    *BOOT_STATUS.lock() = status;
}

/// Initialize the kernel and set up required subsystems
pub fn init(config: &Config) -> Result<(), &'static str> {
    let boot_config = BootConfig::default();
    return internal_init(boot_config);
}

/// Internal initialization function that works with BootConfig
pub fn internal_init(config: BootConfig) -> Result<(), &'static str> {
    set_boot_status(BootStatus::NotStarted);
    
    // 1. CPU Initialization and feature detection
    set_boot_status(BootStatus::CPUInitializing);
    cpu_init()?;
    
    // 2. Memory management initialization
    set_boot_status(BootStatus::MemoryInitializing);
    memory_init(config.memory_map)?;
    
    // 3. Display/HDMI initialization
    set_boot_status(BootStatus::DisplayInitializing);
    display_init(&config)?;
    
    // 4. Storage subsystem initialization
    set_boot_status(BootStatus::StorageInitializing);
    storage_init()?;
    
    // 5. Input devices initialization
    set_boot_status(BootStatus::InputInitializing);
    input_init()?;
    
    // 6. Network subsystem initialization
    set_boot_status(BootStatus::NetworkInitializing);
    network_init()?;
    
    // 7. Sound subsystem initialization
    set_boot_status(BootStatus::SoundInitializing);
    sound_init()?;
    
    // 8. Filesystem initialization
    set_boot_status(BootStatus::FilesystemInitializing);
    filesystem_init()?;
    
    // 9. Power management initialization
    set_boot_status(BootStatus::PowerInitializing);
    power_init()?;
    
    // Boot complete
    set_boot_status(BootStatus::BootCompleted);
    
    #[cfg(feature = "std")]
    log::info!("OS Gaming boot sequence completed successfully");
    
    #[cfg(not(feature = "std"))]
    println!("OS Gaming boot sequence completed successfully");
    
    Ok(())
}

/// Initialize CPU features and optimizations
fn cpu_init() -> Result<(), &'static str> {
    // CPU feature detection
    #[cfg(feature = "std")]
    {
        // Use raw-cpuid crate to detect CPU features
        let cpuid = raw_cpuid::CpuId::new();
        
        if let Some(vendor_info) = cpuid.get_vendor_info() {
            log::info!("CPU Vendor: {}", vendor_info.as_str());
        }
        
        if let Some(processor_brand) = cpuid.get_processor_brand_string() {
            log::info!("CPU Model: {}", processor_brand.as_str());
        }
        
        // Check for important features for gaming
        if let Some(feature_info) = cpuid.get_feature_info() {
            let has_avx2 = cpuid.get_extended_feature_info()
                .map_or(false, |info| info.has_avx2());
                
            log::info!("CPU Features: SSE: {}, SSE2: {}, SSE3: {}, SSE4.1: {}, SSE4.2: {}, AVX: {}, AVX2: {}",
                feature_info.has_sse(),
                feature_info.has_sse2(),
                feature_info.has_sse3(),
                feature_info.has_sse41(),
                feature_info.has_sse42(),
                feature_info.has_avx(),
                has_avx2);
        }
        
        // Log physical cores and threads
        log::info!("CPU has {} physical cores, {} logical processors", 
            num_cpus::get_physical(),
            num_cpus::get());
    }
    
    #[cfg(not(feature = "std"))]
    {
        // In bare metal mode, we can directly read CPU registers
        use x86_64::registers::model_specific::Msr;
        
        // Initialize CPU model-specific registers for optimal gaming performance
        unsafe {
            // This would be expanded with actual optimizations for gaming
            // Example: setting power management MSRs for high performance
        }
    }
    
    Ok(())
}

/// Initialize memory management
fn memory_init(memory_map: Option<&'static [u8]>) -> Result<(), &'static str> {
    #[cfg(feature = "std")]
    {
        // In std mode, just log system memory info
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        
        let total_memory_mb = sys.total_memory() / 1024;
        let used_memory_mb = sys.used_memory() / 1024;
        let free_memory_mb = total_memory_mb - used_memory_mb;
        
        log::info!("Memory: {}/{} MB free ({} MB used)", 
            free_memory_mb, total_memory_mb, used_memory_mb);
    }
    
    #[cfg(not(feature = "std"))]
    {
        // In OS mode, we would initialize:
        // 1. Physical memory manager
        // 2. Virtual memory mappings
        // 3. Kernel heap allocator
        
        if let Some(map_data) = memory_map {
            // Parse the memory map provided by bootloader
            // (implementation would depend on your bootloader)
        }
        
        // Initialize memory management systems
        // (This is a placeholder for your actual memory initialization code)
    }
    
    Ok(())
}

/// Initialize display subsystem
fn display_init(config: &BootConfig) -> Result<(), &'static str> {
    // Initialize HDMI driver
    crate::kernel::drivers::hdmi::init()?;
    
    // Apply requested resolution if specified
    if let (Some(width), Some(height), Some(refresh)) = 
        (config.display_width, config.display_height, config.refresh_rate) {
        
        let resolution = drivers::hdmi::HdmiResolution {
            width,
            height,
            refresh_rate: refresh,
        };
        
        crate::kernel::drivers::hdmi::set_resolution(resolution)?;
    }
    
    // Detect graphics hardware
    crate::kernel::drivers::hdmi::detect_graphics_hardware()?;
    
    // Check if system meets minimum requirements for the OS
    let requirements = GamingRequirements {
        min_vram_mb: 512,       // Minimum 512MB VRAM
        requires_vulkan: false, // Vulkan not strictly required
        requires_opengl: true,  // OpenGL required
        requires_raytracing: false, // Ray tracing not required
        min_width: 1280,        // Minimum 720p resolution
        min_height: 720,
        min_refresh_rate: 60,   // Minimum 60Hz refresh rate
    };
    
    if !crate::kernel::drivers::hdmi::meets_gaming_requirements(&requirements) {
        return Err("System does not meet minimum graphics requirements");
    }
    
    // Log detected display information
    #[cfg(feature = "std")]
    if let Some(gpu) = crate::kernel::drivers::hdmi::get_primary_gpu() {
        log::info!("Primary GPU: {} {}", 
            match &gpu.vendor {
                drivers::hdmi::GpuVendor::Nvidia => "NVIDIA",
                drivers::hdmi::GpuVendor::AMD => "AMD",
                drivers::hdmi::GpuVendor::Intel => "Intel",
                drivers::hdmi::GpuVendor::VMware => "VMware",
                drivers::hdmi::GpuVendor::VirtualBox => "VirtualBox",
                drivers::hdmi::GpuVendor::Other(name) => name,
                drivers::hdmi::GpuVendor::Unknown => "Unknown",
            },
            gpu.model
        );
    }
    
    Ok(())
}

/// Initialize storage subsystem
fn storage_init() -> Result<(), &'static str> {
    // Initialize storage subsystem
    let storage_manager = drivers::storage::init()?;
    
    // Scan for available storage devices
    #[cfg(feature = "std")]
    {
        log::info!("Scanning for storage devices...");
        
        for (idx, device) in storage_manager.get_devices().iter().enumerate() {
            log::info!("Storage device {}: {} ({:?}, {} bytes)", 
                idx,
                device.get_name(),
                device.get_device_type(),
                device.get_size_bytes());
        }
    }
    
    Ok(())
}

/// Initialize input devices
fn input_init() -> Result<(), &'static str> {
    // Initialize keyboard, mouse, and gamepad drivers
    drivers::keyboard::init();
    drivers::mouse::init();
    drivers::gamepad::init();
    
    #[cfg(feature = "std")]
    log::info!("Input devices initialized");
    
    Ok(())
}

/// Initialize network subsystem
fn network_init() -> Result<(), &'static str> {
    // Initialize network driver
    let net_manager = drivers::network::init()?;
    
    #[cfg(feature = "std")]
    {
        // Log network interfaces
        log::info!("Network interfaces initialized");
        
        for (idx, interface) in net_manager.get_interfaces().iter().enumerate() {
            log::info!("Network interface {}: {}, MAC: {:?}, IP: {:?}", 
                idx,
                interface.get_name(),
                interface.get_mac_address(),
                interface.get_ip_address());
        }
    }
    
    Ok(())
}

/// Initialize sound subsystem
fn sound_init() -> Result<(), &'static str> {
    // Initialize sound driver
    let sound_system = drivers::sound::init()?;
    
    #[cfg(feature = "std")]
    {
        log::info!("Sound system initialized");
        
        for (idx, device) in sound_system.get_output_devices().iter().enumerate() {
            log::info!("Sound device {}: {} ({:?})", 
                idx,
                device.get_name(),
                device.get_device_type());
        }
    }
    
    Ok(())
}

/// Initialize filesystem
fn filesystem_init() -> Result<(), &'static str> {
    // Get storage manager from the previously initialized storage subsystem
    // Reuse the storage manager instance created during storage_init()
    let storage_manager = drivers::storage::init()?;
    
    // Initialize filesystem with storage manager
    drivers::filesystem::init(&storage_manager)?;
    
    #[cfg(feature = "std")]
    {
        log::info!("Filesystem initialized");
        
        let fs_manager = drivers::filesystem::get_fs_manager();
        let fs_manager = fs_manager.lock();
        
        for (idx, fs) in fs_manager.get_filesystems().iter().enumerate() {
            log::info!("Filesystem {}: {} ({:?}) on {}, {}",
                idx,
                fs.get_name(),
                fs.get_type(),
                fs.get_device(),
                if fs.is_readonly() { "read-only" } else { "read-write" });
        }
    }
    
    Ok(())
}

/// Initialize power management
fn power_init() -> Result<(), &'static str> {
    // Initialize power management
    drivers::power::init()?;
    
    #[cfg(feature = "std")]
    log::info!("Power management initialized");
    
    Ok(())
}

/// Panic handler for kernel boot errors
#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("Kernel panic during boot: {}", info);
    
    // Set boot status to failed
    set_boot_status(BootStatus::Failed(0xDEAD));
    
    loop {
        // Halt CPU or wait for reset
        core::hint::spin_loop();
    }
}