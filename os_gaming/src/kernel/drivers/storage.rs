extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, Ordering};

/// Types of storage devices
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageDeviceType {
    Unknown,
    Ata,
    Nvme,
    Usb,
    Scsi,
    VirtIO,
}

/// Represents a storage device in the system
pub struct StorageDevice {
    name: String,
    device_type: StorageDeviceType,
    sector_size: u32,
    sector_count: u64,
    initialized: AtomicBool,
    read_only: bool,
}

pub struct Partition {
    device_name: String,
    start_sector: u64,
    sector_count: u64,
    partition_type: u8,
    bootable: bool,
}

/// Represents the storage subsystem
pub struct StorageManager {
    devices: Vec<StorageDevice>,
    default_device: Option<usize>,
}

impl StorageDevice {
    /// Create a new storage device
    pub fn new(name: String, device_type: StorageDeviceType, sector_size: u32, sector_count: u64, read_only: bool) -> Self {
        Self {
            name,
            device_type,
            sector_size,
            sector_count,
            initialized: AtomicBool::new(false),
            read_only,
        }
    }
    
    /// Initialize the storage device
    pub fn initialize(&self) -> Result<(), &'static str> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }
        
        // Device-specific initialization would go here
        // This would typically involve setting up DMA regions,
        // initializing the controller, etc.
        
        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }
    
    /// Read sectors from the device
    pub fn read_sectors(&self, start_sector: u64, count: u32, buffer: &mut [u8]) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Storage device not initialized");
        }
        
        if start_sector + count as u64 > self.sector_count {
            return Err("Read operation exceeds device bounds");
        }
        
        if buffer.len() < (count as usize * self.sector_size as usize) {
            return Err("Buffer too small for requested sectors");
        }
        
        // Device-specific read operation would go here
        // For now, we just fill the buffer with a pattern for demonstration
        #[cfg(feature = "std")]
        {
            for i in 0..buffer.len() {
                buffer[i] = (i % 256) as u8;
            }
        }
        
        Ok(())
    }
    
    /// Write sectors to the device
    pub fn write_sectors(&self, start_sector: u64, count: u32, buffer: &[u8]) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("Storage device not initialized");
        }
        
        if self.read_only {
            return Err("Cannot write to read-only device");
        }
        
        if start_sector + count as u64 > self.sector_count {
            return Err("Write operation exceeds device bounds");
        }
        
        if buffer.len() < (count as usize * self.sector_size as usize) {
            return Err("Buffer too small for requested sectors");
        }
        
        // Device-specific write operation would go here
        
        Ok(())
    }
    
    /// Get device name
    pub fn get_name(&self) -> &str {
        &self.name
    }
    
    /// Get device type
    pub fn get_device_type(&self) -> StorageDeviceType {
        self.device_type
    }
    
    /// Get total size in bytes
    pub fn get_size_bytes(&self) -> u64 {
        self.sector_count * self.sector_size as u64
    }
    
    /// Get sector size
    pub fn get_sector_size(&self) -> u32 {
        self.sector_size
    }
    
    /// Check if device is read-only
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }
}

impl Partition {
    /// Create a new partition
    pub fn new(device_name: String, start_sector: u64, sector_count: u64, partition_type: u8, bootable: bool) -> Self {
        Self {
            device_name,
            start_sector,
            sector_count,
            partition_type,
            bootable,
        }
    }

    pub fn get_start_sector(&self) -> u64 {
        self.start_sector
    }
    pub fn get_sector_count(&self) -> u64 {
        self.sector_count
    }
    pub fn get_partition_type(&self) -> u8 {
        self.partition_type
    }
    pub fn is_bootable(&self) -> bool {
        self.bootable
    }
    pub fn get_device_name(&self) -> &str {
        &self.device_name
    }
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            default_device: None,
        }
    }

    pub fn scan_partitions(&mut self, device_name: &str) -> Result<Vec<Partition>, &'static str> {
        let device = self.get_device(device_name)
            .ok_or("Device not found")?;
        
        // Read MBR (first sector)
        let mut mbr_buffer = vec![0u8; device.get_sector_size() as usize];
        device.read_sectors(0, 1, &mut mbr_buffer)?;
        
        // Check for valid MBR signature (last two bytes should be 0x55, 0xAA)
        if mbr_buffer[510] != 0x55 || mbr_buffer[511] != 0xAA {
            return Err("Invalid MBR signature");
        }
        
        // Parse partition table (starts at offset 446)
        let mut partitions = Vec::new();
        for i in 0..4 {
            let offset = 446 + i * 16;
            
            // Check if partition entry is used
            let partition_type = mbr_buffer[offset + 4];
            if partition_type == 0 {
                continue; // Empty partition entry
            }
            
            // Extract partition information
            let bootable = mbr_buffer[offset] == 0x80;
            let start_sector = u32::from_le_bytes([
                mbr_buffer[offset + 8],
                mbr_buffer[offset + 9],
                mbr_buffer[offset + 10],
                mbr_buffer[offset + 11],
            ]) as u64;
            
            let sector_count = u32::from_le_bytes([
                mbr_buffer[offset + 12],
                mbr_buffer[offset + 13],
                mbr_buffer[offset + 14],
                mbr_buffer[offset + 15],
            ]) as u64;
            
            partitions.push(Partition {
                device_name: device_name.to_string(),
                start_sector,
                sector_count,
                partition_type,
                bootable,
            });
        }
        
        Ok(partitions)
    }
    
    // Add method to read from a partition
    pub fn read_partition(&self, partition: &Partition, relative_sector: u64, 
                        count: u32, buffer: &mut [u8]) -> Result<(), &'static str> {
        let device = self.get_device(&partition.device_name)
            .ok_or("Device not found")?;
        
        if relative_sector + count as u64 > partition.sector_count {
            return Err("Read exceeds partition bounds");
        }
        
        // Convert relative sector to absolute sector
        let absolute_sector = partition.start_sector + relative_sector;
        
        // Perform the read
        device.read_sectors(absolute_sector, count, buffer)
    }
    
    // Add method to write to a partition
    pub fn write_partition(&self, partition: &Partition, relative_sector: u64, 
                         count: u32, buffer: &[u8]) -> Result<(), &'static str> {
        let device = self.get_device(&partition.device_name)
            .ok_or("Device not found")?;
        
        if relative_sector + count as u64 > partition.sector_count {
            return Err("Write exceeds partition bounds");
        }
        
        // Convert relative sector to absolute sector
        let absolute_sector = partition.start_sector + relative_sector;
        
        // Perform the write
        device.write_sectors(absolute_sector, count, buffer)
    }
    
    /// Add a storage device to the manager
    pub fn add_device(&mut self, device: StorageDevice) -> Result<(), &'static str> {
        // Initialize the device
        device.initialize()?;
        
        // Add to our list
        self.devices.push(device);
        
        // If this is the first device, make it the default
        if self.default_device.is_none() && !self.devices.is_empty() {
            self.default_device = Some(0);
        }
        
        Ok(())
    }
    
    /// Get a device by name
    pub fn get_device(&self, name: &str) -> Option<&StorageDevice> {
        self.devices.iter().find(|dev| dev.get_name() == name)
    }
    
    /// Get a mutable reference to a device by name
    pub fn get_device_mut(&mut self, name: &str) -> Option<&mut StorageDevice> {
        self.devices.iter_mut().find(|dev| dev.get_name() == name)
    }
    
    /// Get all devices
    pub fn get_devices(&self) -> &[StorageDevice] {
        &self.devices
    }
    
    /// Get the default device
    pub fn get_default_device(&self) -> Option<&StorageDevice> {
        self.default_device.map(|idx| &self.devices[idx])
    }
    
    /// Set the default device by name
    pub fn set_default_device(&mut self, name: &str) -> Result<(), &'static str> {
        let idx = self.devices.iter().position(|dev| dev.get_name() == name)
            .ok_or("Device not found")?;
        
        self.default_device = Some(idx);
        Ok(())
    }
}

/// Initialize storage subsystem
pub fn init() -> Result<StorageManager, &'static str> {
    let mut manager = StorageManager::new();
    
    // Detect storage devices
    // In a real OS, this would involve scanning PCI bus, SATA controllers, etc.
    
    #[cfg(feature = "std")]
    {
        // For testing in std mode, create some virtual devices
        let primary_disk = StorageDevice::new(
            "sda".to_string(),
            StorageDeviceType::Ata,
            512,
            2_000_000, // ~1GB
            false
        );
        
        manager.add_device(primary_disk)?;
        
        // Add a second device
        let secondary_disk = StorageDevice::new(
            "sdb".to_string(),
            StorageDeviceType::Nvme,
            4096,
            4_000_000, // ~16GB with 4K sectors
            false
        );
        
        manager.add_device(secondary_disk)?;
        
        log::info!("Detected {} storage devices", manager.get_devices().len());
    }
    
    Ok(manager)
}