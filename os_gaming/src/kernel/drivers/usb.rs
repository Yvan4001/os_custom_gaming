extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

/// USB controller types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsbControllerType {
    Uhci,   // Universal Host Controller Interface (USB 1.1)
    Ohci,   // Open Host Controller Interface (USB 1.1)
    Ehci,   // Enhanced Host Controller Interface (USB 2.0)
    Xhci,   // Extensible Host Controller Interface (USB 3.0+)
}

/// USB device classes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsbDeviceClass {
    Unknown = 0x00,
    Audio = 0x01,
    Cdc = 0x02,
    Hid = 0x03,
    Physical = 0x05,
    Image = 0x06,
    Printer = 0x07,
    MassStorage = 0x08,
    Hub = 0x09,
    CdcData = 0x0A,
    SmartCard = 0x0B,
    ContentSecurity = 0x0D,
    Video = 0x0E,
    PersonalHealthcare = 0x0F,
    AudioVideo = 0x10,
    Billboard = 0x11,
    TypeC = 0x12,
    Diagnostic = 0xDC,
    Wireless = 0xE0,
    Miscellaneous = 0xEF,
    VendorSpecific = 0xFF,
}

/// USB device information
pub struct UsbDevice {
    vendor_id: u16,
    product_id: u16,
    class: UsbDeviceClass,
    subclass: u8,
    protocol: u8,
    manufacturer: String,
    product: String,
    serial_number: String,
    interfaces: Vec<UsbInterface>,
    port: u8,
    address: u8,
    speed: UsbSpeed,
    connected: AtomicBool,
}

/// USB device speed
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsbSpeed {
    Low,    // 1.5 Mbit/s
    Full,   // 12 Mbit/s
    High,   // 480 Mbit/s
    Super,  // 5 Gbit/s
    SuperPlus, // 10 Gbit/s
}

/// USB interface information
pub struct UsbInterface {
    interface_number: u8,
    alt_setting: u8,
    class: UsbDeviceClass,
    subclass: u8,
    protocol: u8,
    endpoints: Vec<UsbEndpoint>,
}

/// USB endpoint information
pub struct UsbEndpoint {
    address: u8,
    attributes: u8,
    max_packet_size: u16,
    interval: u8,
}

/// USB controller information
pub struct UsbController {
    controller_type: UsbControllerType,
    base_address: u64,
    irq: u8,
    port_count: u8,
    initialized: AtomicBool,
    devices: Vec<UsbDevice>,
}

/// USB subsystem manager
pub struct UsbManager {
    controllers: Vec<UsbController>,
    devices: Vec<UsbDevice>,
}

impl UsbDevice {
    /// Create a new USB device
    pub fn new(
        vendor_id: u16, 
        product_id: u16, 
        class: UsbDeviceClass, 
        subclass: u8, 
        protocol: u8,
        port: u8,
        address: u8,
        speed: UsbSpeed
    ) -> Self {
        Self {
            vendor_id,
            product_id,
            class,
            subclass,
            protocol,
            manufacturer: String::new(),
            product: String::new(),
            serial_number: String::new(),
            interfaces: Vec::new(),
            port,
            address,
            speed,
            connected: AtomicBool::new(true),
        }
    }
    
    /// Get device identifier string
    pub fn get_id_string(&self) -> String {
        format!("{:04x}:{:04x}", self.vendor_id, self.product_id)
    }
    
    /// Get device class string
    pub fn get_class_string(&self) -> &str {
        match self.class {
            UsbDeviceClass::Unknown => "Unknown",
            UsbDeviceClass::Audio => "Audio",
            UsbDeviceClass::Cdc => "CDC",
            UsbDeviceClass::Hid => "HID",
            UsbDeviceClass::Physical => "Physical",
            UsbDeviceClass::Image => "Image",
            UsbDeviceClass::Printer => "Printer",
            UsbDeviceClass::MassStorage => "Mass Storage",
            UsbDeviceClass::Hub => "Hub",
            UsbDeviceClass::CdcData => "CDC Data",
            UsbDeviceClass::SmartCard => "Smart Card",
            UsbDeviceClass::ContentSecurity => "Content Security",
            UsbDeviceClass::Video => "Video",
            UsbDeviceClass::PersonalHealthcare => "Personal Healthcare",
            UsbDeviceClass::AudioVideo => "Audio/Video",
            UsbDeviceClass::Billboard => "Billboard",
            UsbDeviceClass::TypeC => "USB Type-C",
            UsbDeviceClass::Diagnostic => "Diagnostic",
            UsbDeviceClass::Wireless => "Wireless",
            UsbDeviceClass::Miscellaneous => "Miscellaneous",
            UsbDeviceClass::VendorSpecific => "Vendor Specific",
        }
    }
    
    /// Get device product name
    pub fn get_product_name(&self) -> &str {
        if self.product.is_empty() {
            "Unknown Device"
        } else {
            &self.product
        }
    }
    
    /// Set device strings
    pub fn set_strings(&mut self, manufacturer: String, product: String, serial: String) {
        self.manufacturer = manufacturer;
        self.product = product;
        self.serial_number = serial;
    }
    
    /// Add an interface to the device
    pub fn add_interface(&mut self, interface: UsbInterface) {
        self.interfaces.push(interface);
    }
    
    /// Check if device is connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
    
    /// Disconnect device
    pub fn disconnect(&self) {
        self.connected.store(false, Ordering::SeqCst);
    }
}

impl UsbInterface {
    /// Create a new USB interface
    pub fn new(
        interface_number: u8,
        alt_setting: u8,
        class: UsbDeviceClass,
        subclass: u8,
        protocol: u8
    ) -> Self {
        Self {
            interface_number,
            alt_setting,
            class,
            subclass,
            protocol,
            endpoints: Vec::new(),
        }
    }
    
    /// Add an endpoint to the interface
    pub fn add_endpoint(&mut self, endpoint: UsbEndpoint) {
        self.endpoints.push(endpoint);
    }
}

impl UsbEndpoint {
    /// Create a new USB endpoint
    pub fn new(
        address: u8,
        attributes: u8,
        max_packet_size: u16,
        interval: u8
    ) -> Self {
        Self {
            address,
            attributes,
            max_packet_size,
            interval,
        }
    }
    
    /// Get endpoint direction (IN = true, OUT = false)
    pub fn is_in(&self) -> bool {
        (self.address & 0x80) != 0
    }
    
    /// Get endpoint number
    pub fn get_number(&self) -> u8 {
        self.address & 0x0F
    }
    
    /// Get endpoint type
    pub fn get_type(&self) -> u8 {
        (self.attributes & 0x03)
    }
}

impl UsbController {
    /// Create a new USB controller
    pub fn new(
        controller_type: UsbControllerType,
        base_address: u64,
        irq: u8,
        port_count: u8
    ) -> Self {
        Self {
            controller_type,
            base_address,
            irq,
            port_count,
            initialized: AtomicBool::new(false),
            devices: Vec::new(),
        }
    }
    
    /// Initialize the controller
    pub fn initialize(&mut self) -> Result<(), &'static str> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }
        
        // Controller-specific initialization would go here
        // This would involve setting up registers, enabling interrupts, etc.
        
        self.initialized.store(true, Ordering::SeqCst);
        
        #[cfg(feature = "std")]
        log::info!("USB {:?} controller initialized", self.controller_type);
        
        Ok(())
    }
    
    /// Scan for devices on this controller
    pub fn scan_devices(&mut self) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("USB controller not initialized");
        }
        
        // In a real driver, this would enumerate devices on all ports
        // For now, we'll simulate device detection
        
        #[cfg(feature = "std")]
        {
            // For testing, pretend we found some devices
            if self.controller_type == UsbControllerType::Xhci {
                // Add a mouse
                let mut mouse = UsbDevice::new(
                    0x046D, 0xC52B, UsbDeviceClass::Hid, 1, 2, 1, 2, UsbSpeed::Full
                );
                mouse.set_strings(
                    "Logitech".to_string(),
                    "USB Optical Mouse".to_string(),
                    "12345678".to_string()
                );
                
                // Add interface and endpoints for the mouse
                let mut if_mouse = UsbInterface::new(0, 0, UsbDeviceClass::Hid, 1, 2);
                if_mouse.add_endpoint(UsbEndpoint::new(0x81, 0x03, 8, 10)); // IN interrupt
                mouse.add_interface(if_mouse);
                
                self.devices.push(mouse);
                
                // Add a keyboard
                let mut keyboard = UsbDevice::new(
                    0x045E, 0x00DB, UsbDeviceClass::Hid, 1, 1, 2, 3, UsbSpeed::Full
                );
                keyboard.set_strings(
                    "Microsoft".to_string(),
                    "Natural Keyboard".to_string(),
                    "87654321".to_string()
                );
                
                // Add interface and endpoints for the keyboard
                let mut if_keyboard = UsbInterface::new(0, 0, UsbDeviceClass::Hid, 1, 1);
                if_keyboard.add_endpoint(UsbEndpoint::new(0x81, 0x03, 8, 10)); // IN interrupt
                keyboard.add_interface(if_keyboard);
                
                self.devices.push(keyboard);
            }
        }
        
        Ok(())
    }
    
    /// Get all devices connected to this controller
    pub fn get_devices(&self) -> &[UsbDevice] {
        &self.devices
    }
}

impl UsbManager {
    /// Create a new USB manager
    pub fn new() -> Self {
        Self {
            controllers: Vec::new(),
            devices: Vec::new(),
        }
    }
    
    /// Add a controller to the manager
    pub fn add_controller(&mut self, mut controller: UsbController) -> Result<(), &'static str> {
        // Initialize the controller
        controller.initialize()?;
        
        // Scan for devices
        controller.scan_devices()?;
        
        // Add all devices from this controller to our list
        for device in controller.get_devices() {
            self.devices.push(device.clone());
        }
        
        // Add the controller
        self.controllers.push(controller);
        
        Ok(())
    }
    
    /// Get all USB devices
    pub fn get_devices(&self) -> &[UsbDevice] {
        &self.devices
    }
    
    /// Get devices of a specific class
    pub fn get_devices_by_class(&self, class: UsbDeviceClass) -> Vec<&UsbDevice> {
        self.devices.iter()
            .filter(|dev| dev.class == class)
            .collect()
    }
    
    /// Find device by vendor and product ID
    pub fn find_device(&self, vendor_id: u16, product_id: u16) -> Option<&UsbDevice> {
        self.devices.iter()
            .find(|dev| dev.vendor_id == vendor_id && dev.product_id == product_id)
    }
}

// Add Clone implementation for device structures
impl Clone for UsbDevice {
    fn clone(&self) -> Self {
        Self {
            vendor_id: self.vendor_id,
            product_id: self.product_id,
            class: self.class,
            subclass: self.subclass,
            protocol: self.protocol,
            manufacturer: self.manufacturer.clone(),
            product: self.product.clone(),
            serial_number: self.serial_number.clone(),
            interfaces: self.interfaces.clone(),
            port: self.port,
            address: self.address,
            speed: self.speed,
            connected: AtomicBool::new(self.connected.load(Ordering::SeqCst)),
        }
    }
}

impl Clone for UsbInterface {
    fn clone(&self) -> Self {
        Self {
            interface_number: self.interface_number,
            alt_setting: self.alt_setting,
            class: self.class,
            subclass: self.subclass,
            protocol: self.protocol,
            endpoints: self.endpoints.clone(),
        }
    }
}

impl Clone for UsbEndpoint {
    fn clone(&self) -> Self {
        Self {
            address: self.address,
            attributes: self.attributes,
            max_packet_size: self.max_packet_size,
            interval: self.interval,
        }
    }
}

/// Initialize the USB subsystem
pub fn init() -> Result<UsbManager, &'static str> {
    let mut manager = UsbManager::new();
    
    // Detect USB controllers
    // In a real OS, this would involve scanning the PCI bus
    
    #[cfg(feature = "std")]
    {
        // For testing, create virtual controllers
        let xhci = UsbController::new(
            UsbControllerType::Xhci,
            0xFED00000,
            11,
            4
        );
        
        manager.add_controller(xhci)?;
        
        log::info!("USB subsystem initialized with {} controllers and {} devices",
            manager.controllers.len(),
            manager.devices.len()
        );
    }
    
    Ok(manager)
}