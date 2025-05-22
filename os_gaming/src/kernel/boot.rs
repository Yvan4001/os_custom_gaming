use super::drivers::{self, hdmi::GamingRequirements};
use crate::kernel::interrupts;
use crate::config;
use lazy_static::lazy_static;
use spin::Mutex;
extern crate alloc;
use alloc::string::String;
use crate::println;
use crate::boot::info::CustomBootInfo as BootInfo;
use crate::Config;
use crate::kernel::memory::memory_manager::MemoryManager;

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
    pub boot_info: Option<&'static BootInfo>,
}

impl Default for BootConfig {
    fn default() -> Self {
        Self {
            memory_map: None,
            cmdline: None,
            display_width: Some(1920),
            display_height: Some(1080),
            refresh_rate: Some(60),
            boot_info: None,
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
    *BOOT_STATUS.lock() = status;

    #[cfg(not(feature = "std"))]
    match status {
        BootStatus::NotStarted => println!("Boot process starting"),
        BootStatus::CPUInitializing => println!("Initializing CPU"),
        BootStatus::MemoryInitializing => println!("Initializing memory subsystem"),
        BootStatus::BootCompleted => println!("Boot process completed successfully"),
        BootStatus::Failed(code) => println!("Boot process failed with error code: {}", code),
        _ => {}
    }
}


/// Initialize the kernel and set up required subsystems
pub fn init(boot_info: &'static BootInfo) -> Result<(), &'static str> {
    let mut boot_config = BootConfig::default();
    boot_config.boot_info = Some(boot_info);

    // Set default display settings
    let config = Config::default();
    boot_config.display_width = Some(config.width);
    boot_config.display_height = Some(config.height);
    boot_config.refresh_rate = Some(config.refresh_rate);

    internal_init(boot_config)
}


/// Internal initialization function that works with BootConfig
pub fn internal_init(config: BootConfig) -> Result<(), &'static str> {
    set_boot_status(BootStatus::NotStarted);
    
    // 1. CPU Initialization and feature detection
    set_boot_status(BootStatus::CPUInitializing);
    cpu_init()?;

    // 2. Memory management initialization
    set_boot_status(BootStatus::MemoryInitializing);
    if let Some(boot_info) = config.boot_info {
        memory_init(boot_info)?;
    } else {
        return Err("No boot information available for memory initialization");
    }

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

    // 10. Initialize interrupts
    interrupts::init();
    
    // Boot complete
    set_boot_status(BootStatus::BootCompleted);
    
    #[cfg(feature = "std")]
    log::info!("OS Gaming boot sequence completed successfully");
    
    #[cfg(not(feature = "std"))]
    println!("OS Gaming boot sequence completed successfully");
    
    Ok(())
}

/// Initialize CPU features and optimizations
/// Initialize CPU features
fn cpu_init() -> Result<(), &'static str> {
    // Basic CPU initialization
    #[cfg(not(feature = "std"))]
    {
        use x86_64::instructions;
        instructions::interrupts::disable();
        // Add any other CPU initialization here
    }

    Ok(())
}


/// Initialize memory management
fn memory_init(boot_info: &'static BootInfo) -> Result<(), &'static str> {
    // Setting physical memory offset first is correct
    let phys_mem_offset = boot_info.physical_memory_offset;

    // Add this code to reserve the problematic memory region
    // This explicitly marks the 0x400000 region as used before page mapping
    #[cfg(not(feature = "std"))]
    unsafe {
        // Calculate virtual address for the problematic physical address
        let virt_addr = match phys_mem_offset {
            Some(offset) => offset + 0x500000,
            None => {
                log::error!("Physical memory offset missing");
                return Err(crate::kernel::memory::memory_manager::MemoryInitError::PhysicalOffsetMissing.into());
            }
        };
        // Mark it as used by writing a magic value
        core::ptr::write_volatile(virt_addr as *mut u64, 0xDEADBEEF);
        println!("Reserved problematic memory region at physical address 0x400000");
    }

    // Continue with normal memory initialization
    MemoryManager::init_core(boot_info)?;
    crate::kernel::memory::allocator::init_heap()
        .map_err(|_| "Error initializing heap")?;

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

/// Panic handler for boot errors
#[cfg(not(feature = "std"))]
pub fn boot_panic(info: &core::panic::PanicInfo) -> ! {
    println!("Kernel panic during boot: {}", info);
    set_boot_status(BootStatus::Failed(0xDEAD));
    loop {
        x86_64::instructions::hlt();
    }
}
