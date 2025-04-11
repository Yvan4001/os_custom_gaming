extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, Ordering};
use x86_64::instructions::port::Port;

// Common Ethernet types
pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const ETHERTYPE_ARP: u16 = 0x0806;
pub const ETHERTYPE_IPV6: u16 = 0x86DD;

/// Network card types that we can support
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetworkCardType {
    Unknown,
    IntelE1000,
    RealtekRTL8139,
    BroadcomBCM5751,
    VirtIO,
}

/// Represents a network interface
pub struct NetworkInterface {
    name: String,
    mac_address: [u8; 6],
    ip_address: Option<[u8; 4]>,
    active: AtomicBool,
    mtu: u16,
    card_type: NetworkCardType,
    io_base: u16,        // Base I/O port for port-mapped devices
    mem_base: u64,       // Base memory address for memory-mapped devices
    irq: u8,             // IRQ number
    driver: NetworkDriverType,
}

/// Type of network driver implementation to use
enum NetworkDriverType {
    None,
    E1000(E1000Driver),
    RTL8139(RTL8139Driver),
}

/// Trait that all network card drivers must implement
pub trait NetworkDriver {
    fn init(&mut self) -> Result<(), &'static str>;
    fn send(&mut self, data: &[u8]) -> Result<(), &'static str>;
    fn receive(&mut self) -> Option<Vec<u8>>;
    fn get_mac_address(&self) -> [u8; 6];
    fn reset(&mut self) -> Result<(), &'static str>;
}

/// Ethernet frame structure
pub struct EthernetFrame {
    pub destination: [u8; 6],
    pub source: [u8; 6],
    pub ethertype: u16,
    pub payload: Vec<u8>,
}

impl EthernetFrame {
    pub fn new(dest: [u8; 6], src: [u8; 6], ethertype: u16, payload: Vec<u8>) -> Self {
        Self {
            destination: dest,
            source: src,
            ethertype,
            payload,
        }
    }
    
    /// Convert frame to byte array for transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(14 + self.payload.len());
        
        // Add destination MAC
        bytes.extend_from_slice(&self.destination);
        
        // Add source MAC
        bytes.extend_from_slice(&self.source);
        
        // Add EtherType (in big-endian)
        bytes.push((self.ethertype >> 8) as u8);
        bytes.push((self.ethertype & 0xFF) as u8);
        
        // Add payload
        bytes.extend_from_slice(&self.payload);
        
        bytes
    }
    
    /// Parse a byte slice into an Ethernet frame
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 14 {
            return None; // Frame too small
        }
        
        let mut destination = [0u8; 6];
        let mut source = [0u8; 6];
        
        destination.copy_from_slice(&bytes[0..6]);
        source.copy_from_slice(&bytes[6..12]);
        
        let ethertype = ((bytes[12] as u16) << 8) | (bytes[13] as u16);
        let payload = bytes[14..].to_vec();
        
        Some(Self {
            destination,
            source,
            ethertype,
            payload,
        })
    }
}

impl NetworkInterface {
    /// Create a new network interface
    pub fn new(name: String, mac_address: [u8; 6], card_type: NetworkCardType) -> Self {
        NetworkInterface {
            name,
            mac_address,
            ip_address: None,
            active: AtomicBool::new(false),
            mtu: 1500, // Default MTU
            card_type,
            io_base: 0,
            mem_base: 0,
            irq: 0,
            driver: NetworkDriverType::None,
        }
    }

    /// Set the IP address for this interface
    pub fn set_ip_address(&mut self, ip: [u8; 4]) {
        self.ip_address = Some(ip);
    }
    
    /// Get the MAC address
    pub fn get_mac_address(&self) -> [u8; 6] {
        self.mac_address
    }
    
    /// Get the IP address if set
    pub fn get_ip_address(&self) -> Option<[u8; 4]> {
        self.ip_address
    }
    
    /// Get interface name
    pub fn get_name(&self) -> &str {
        &self.name
    }
    
    /// Get MTU
    pub fn get_mtu(&self) -> u16 {
        self.mtu
    }
    
    /// Set up the driver with specific hardware parameters
    pub fn setup_hardware(&mut self, io_base: u16, mem_base: u64, irq: u8) -> Result<(), &'static str> {
        self.io_base = io_base;
        self.mem_base = mem_base;
        self.irq = irq;
        
        // Create and initialize the appropriate driver
        match self.card_type {
            NetworkCardType::IntelE1000 => {
                let mut driver = E1000Driver::new(mem_base, irq, self.mac_address);
                driver.init()?;
                self.driver = NetworkDriverType::E1000(driver);
            },
            NetworkCardType::RealtekRTL8139 => {
                let mut driver = RTL8139Driver::new(io_base, irq);
                driver.init()?;
                // Get the MAC from the hardware
                self.mac_address = driver.get_mac_address();
                self.driver = NetworkDriverType::RTL8139(driver);
            },
            _ => return Err("Unsupported network card type"),
        }
        
        Ok(())
    }

    /// Activate the network interface
    pub fn activate(&mut self) -> Result<(), &'static str> {
        match &mut self.driver {
            NetworkDriverType::None => return Err("No driver initialized"),
            NetworkDriverType::E1000(driver) => driver.init()?,
            NetworkDriverType::RTL8139(driver) => driver.init()?,
        }
        
        self.active.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Deactivate the network interface
    pub fn deactivate(&mut self) {
        self.active.store(false, Ordering::SeqCst);
    }

    /// Send a packet through this interface
    pub fn send_packet(&mut self, data: &[u8]) -> Result<(), &'static str> {
        if !self.active.load(Ordering::SeqCst) {
            return Err("Network interface is not active");
        }

        match &mut self.driver {
            NetworkDriverType::None => Err("No driver initialized"),
            NetworkDriverType::E1000(driver) => driver.send(data),
            NetworkDriverType::RTL8139(driver) => driver.send(data),
        }
    }
    
    /// Send an Ethernet frame
    pub fn send_frame(&mut self, frame: &EthernetFrame) -> Result<(), &'static str> {
        if !self.active.load(Ordering::SeqCst) {
            return Err("Network interface is not active");
        }
        
        let bytes = frame.to_bytes();
        self.send_packet(&bytes)
    }

    /// Receive a packet from this interface
    pub fn receive_packet(&mut self) -> Option<Vec<u8>> {
        if !self.active.load(Ordering::SeqCst) {
            return None;
        }

        match &mut self.driver {
            NetworkDriverType::None => None,
            NetworkDriverType::E1000(driver) => driver.receive(),
            NetworkDriverType::RTL8139(driver) => driver.receive(),
        }
    }
    
    /// Receive an Ethernet frame
    pub fn receive_frame(&mut self) -> Option<EthernetFrame> {
        if let Some(data) = self.receive_packet() {
            return EthernetFrame::from_bytes(&data);
        }
        None
    }
    
    /// Reset the network card
    pub fn reset(&mut self) -> Result<(), &'static str> {
        match &mut self.driver {
            NetworkDriverType::None => Err("No driver initialized"),
            NetworkDriverType::E1000(driver) => driver.reset(),
            NetworkDriverType::RTL8139(driver) => driver.reset(),
        }
    }
}

/// Intel E1000 network driver implementation
struct E1000Driver {
    mem_base: u64,
    irq: u8,
    mac_address: [u8; 6],
    rx_ring: Vec<u8>,
    tx_ring: Vec<u8>,
}

impl E1000Driver {
    fn new(mem_base: u64, irq: u8, mac: [u8; 6]) -> Self {
        Self {
            mem_base,
            irq,
            mac_address: mac,
            rx_ring: Vec::with_capacity(16 * 2048), // 16 descriptors of 2048 bytes each
            tx_ring: Vec::with_capacity(16 * 2048),
        }
    }
    
    // Memory-mapped register access
    fn read_reg(&self, reg: u32) -> u32 {
        unsafe {
            let ptr = (self.mem_base as *mut u32).add((reg / 4) as usize);
            ptr.read_volatile()
        }
    }
    
    fn write_reg(&self, reg: u32, value: u32) {
        unsafe {
            let ptr = (self.mem_base as *mut u32).add((reg / 4) as usize);
            ptr.write_volatile(value);
        }
    }
}

impl NetworkDriver for E1000Driver {
    fn init(&mut self) -> Result<(), &'static str> {
        // Memory-mapped I/O addresses for E1000 registers
        const REG_CTRL: u32 = 0x0000; // Control Register
        const REG_STATUS: u32 = 0x0008; // Status Register
        const REG_EEPROM: u32 = 0x0014; // EEPROM Control Register
        const REG_CTRL_EXT: u32 = 0x0018; // Extended Control Register
        
        // Reset the device
        let mut ctrl = self.read_reg(REG_CTRL);
        ctrl |= 0x04000000; // Set RST bit
        self.write_reg(REG_CTRL, ctrl);
        
        // Wait for reset to complete
        while (self.read_reg(REG_CTRL) & 0x04000000) != 0 {
            // Delay
            for _ in 0..1000 {
                core::hint::spin_loop();
            }
        }
        
        // Wait for auto-negotation to complete
        let mut status;
        let mut timeout = 1000;
        loop {
            status = self.read_reg(REG_STATUS);
            if (status & 0x80) != 0 {
                break; // Link up
            }
            
            timeout -= 1;
            if timeout == 0 {
                return Err("E1000: Link timeout");
            }
            
            // Delay
            for _ in 0..1000 {
                core::hint::spin_loop();
            }
        }
        
        // Basic initialization complete
        #[cfg(feature = "std")]
        log::info!("E1000 initialized, link is up");
        
        Ok(())
    }
    
    fn send(&mut self, data: &[u8]) -> Result<(), &'static str> {
        // In a real driver, you would:
        // 1. Check if a TX descriptor is available
        // 2. Copy the data to the TX ring buffer
        // 3. Update the descriptor and tail register
        
        // This is a simplified placeholder
        #[cfg(feature = "std")]
        log::debug!("E1000: Would send {} bytes", data.len());
        
        Ok(())
    }
    
    fn receive(&mut self) -> Option<Vec<u8>> {
        // In a real driver, you would:
        // 1. Check if an RX descriptor has data
        // 2. Copy the data from the RX ring buffer
        // 3. Update the descriptor and tail register
        
        // This is a simplified placeholder
        None
    }
    
    fn get_mac_address(&self) -> [u8; 6] {
        self.mac_address
    }
    
    fn reset(&mut self) -> Result<(), &'static str> {
        self.init()
    }
}

/// Realtek RTL8139 network driver implementation
struct RTL8139Driver {
    io_base: u16,
    irq: u8,
    rx_buffer: Vec<u8>,
    rx_buffer_offset: usize,
}

impl RTL8139Driver {
    fn new(io_base: u16, irq: u8) -> Self {
        Self {
            io_base,
            irq,
            rx_buffer: Vec::with_capacity(8192), // 8K receive buffer
            rx_buffer_offset: 0,
        }
    }
    
    // I/O port register access
    fn read_reg8(&self, reg: u16) -> u8 {
        unsafe {
            let mut port = Port::new(self.io_base + reg);
            port.read()
        }
    }
    
    fn write_reg8(&self, reg: u16, value: u8) {
        unsafe {
            let mut port = Port::new(self.io_base + reg);
            port.write(value);
        }
    }
    
    fn read_reg16(&self, reg: u16) -> u16 {
        unsafe {
            let mut port = Port::new(self.io_base + reg);
            port.read()
        }
    }
    
    fn write_reg16(&self, reg: u16, value: u16) {
        unsafe {
            let mut port = Port::new(self.io_base + reg);
            port.write(value);
        }
    }
    
    fn read_reg32(&self, reg: u16) -> u32 {
        unsafe {
            let mut port = Port::new(self.io_base + reg);
            port.read()
        }
    }
    
    fn write_reg32(&self, reg: u16, value: u32) {
        unsafe {
            let mut port = Port::new(self.io_base + reg);
            port.write(value);
        }
    }
}

impl NetworkDriver for RTL8139Driver {
    fn init(&mut self) -> Result<(), &'static str> {
        // RTL8139 registers
        const REG_CONFIG1: u16 = 0x52;
        const REG_COMMAND: u16 = 0x37;
        const REG_RX_BUF: u16 = 0x30;
        const REG_IMR: u16 = 0x3C; // Interrupt Mask Register
        const REG_ISR: u16 = 0x3E; // Interrupt Status Register
        
        // Power on the device (wake it up)
        self.write_reg8(REG_CONFIG1, 0x00);
        
        // Software reset
        self.write_reg8(REG_COMMAND, 0x10);
        
        // Wait for reset to complete
        while (self.read_reg8(REG_COMMAND) & 0x10) != 0 {
            // Delay
            for _ in 0..1000 {
                core::hint::spin_loop();
            }
        }
        
        // Allocate receive buffer
        self.rx_buffer = vec![0; 8192];
        self.rx_buffer_offset = 0;
        
        // Set up receive buffer address
        let buffer_addr = self.rx_buffer.as_ptr() as u32;
        self.write_reg32(REG_RX_BUF, buffer_addr);
        
        // Enable receiver and transmitter
        self.write_reg8(REG_COMMAND, 0x0C);
        
        // Set interrupt mask
        self.write_reg16(REG_IMR, 0x0005); // ROK + TOK
        
        // Read MAC address from EEPROM (in a real driver)
        
        #[cfg(feature = "std")]
        log::info!("RTL8139 initialized");
        
        Ok(())
    }
    
    fn send(&mut self, data: &[u8]) -> Result<(), &'static str> {
        // In a real driver, you would:
        // 1. Copy the data to a transmit buffer
        // 2. Send the transmit command
        // 3. Wait for completion or setup interrupt
        
        // This is a simplified placeholder
        #[cfg(feature = "std")]
        log::debug!("RTL8139: Would send {} bytes", data.len());
        
        Ok(())
    }
    
    fn receive(&mut self) -> Option<Vec<u8>> {
        // In a real driver, you would:
        // 1. Check if a packet is available
        // 2. Extract the packet from the receive buffer
        // 3. Update buffer pointers
        
        // This is a simplified placeholder
        None
    }
    
    fn get_mac_address(&self) -> [u8; 6] {
        // Read MAC address from the device registers
        let mut mac = [0u8; 6];
        for i in 0..6 {
            mac[i] = self.read_reg8(i as u16);
        }
        mac
    }
    
    fn reset(&mut self) -> Result<(), &'static str> {
        self.init()
    }
}

/// Network driver initialization - detect and setup all available network interfaces
pub fn initialize() -> Result<Vec<NetworkInterface>, &'static str> {
    let mut interfaces = Vec::new();
    
    // In a real OS, you would enumerate PCI devices to find network cards
    #[cfg(feature = "std")]
    {
        // Simulate finding a network card for testing in std mode
        let mut intel_if = NetworkInterface::new(
            "eth0".to_string(),
            [0x00, 0x1B, 0x21, 0x12, 0x34, 0x56], // Sample MAC
            NetworkCardType::IntelE1000
        );
        
        // In a real driver, you would get these from PCI configuration
        intel_if.setup_hardware(0, 0xFEBC0000, 11)?;
        interfaces.push(intel_if);
        
        #[cfg(feature = "std")]
        log::info!("Found and initialized 1 network interface");
    }
    
    // If no interfaces were found, create a dummy/loopback interface
    if interfaces.is_empty() {
        let loopback = NetworkInterface::new(
            "lo".to_string(),
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            NetworkCardType::Unknown
        );
        interfaces.push(loopback);
    }
    
    Ok(interfaces)
}

/// Network Manager to handle multiple interfaces
pub struct NetworkManager {
    interfaces: Vec<NetworkInterface>,
    default_interface: Option<usize>,
}

impl NetworkManager {
    /// Create a new network manager
    pub fn new() -> Result<Self, &'static str> {
        let interfaces = initialize()?;
        let default_interface = if !interfaces.is_empty() { Some(0) } else { None };
        
        Ok(Self {
            interfaces,
            default_interface,
        })
    }
    
    /// Get a list of all interfaces
    pub fn get_interfaces(&self) -> &[NetworkInterface] {
        &self.interfaces
    }
    
    /// Get a mutable reference to an interface by name
    pub fn get_interface_mut(&mut self, name: &str) -> Option<&mut NetworkInterface> {
        self.interfaces.iter_mut().find(|iface| iface.get_name() == name)
    }
    
    /// Get a reference to an interface by name
    pub fn get_interface(&self, name: &str) -> Option<&NetworkInterface> {
        self.interfaces.iter().find(|iface| iface.get_name() == name)
    }
    
    /// Get the default interface
    pub fn get_default_interface(&self) -> Option<&NetworkInterface> {
        self.default_interface.map(|idx| &self.interfaces[idx])
    }
    
    /// Set the default interface by name
    pub fn set_default_interface(&mut self, name: &str) -> Result<(), &'static str> {
        let idx = self.interfaces.iter().position(|iface| iface.get_name() == name)
            .ok_or("Interface not found")?;
        
        self.default_interface = Some(idx);
        Ok(())
    }
    
    /// Send a packet using the default interface
    pub fn send_packet(&mut self, data: &[u8]) -> Result<(), &'static str> {
        if let Some(idx) = self.default_interface {
            self.interfaces[idx].send_packet(data)
        } else {
            Err("No default interface set")
        }
    }
    
    /// ARP request to get MAC address for an IP
    pub fn arp_request(&mut self, target_ip: [u8; 4]) -> Result<[u8; 6], &'static str> {
        let idx = self.default_interface.ok_or("No default interface set")?;
        let interface = &mut self.interfaces[idx];
        
        // Create ARP request packet
        let mut arp_packet = Vec::with_capacity(28);
        
        // Hardware type: Ethernet (1)
        arp_packet.push(0x00);
        arp_packet.push(0x01);
        
        // Protocol type: IPv4 (0x0800)
        arp_packet.push(0x08);
        arp_packet.push(0x00);
        
        // Hardware address length: 6
        arp_packet.push(6);
        
        // Protocol address length: 4
        arp_packet.push(4);
        
        // Operation: Request (1)
        arp_packet.push(0x00);
        arp_packet.push(0x01);
        
        // Sender MAC address
        arp_packet.extend_from_slice(&interface.get_mac_address());
        
        // Sender IP address
        if let Some(ip) = interface.get_ip_address() {
            arp_packet.extend_from_slice(&ip);
        } else {
            return Err("Interface has no IP address");
        }
        
        // Target MAC address (zeros for query)
        arp_packet.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        
        // Target IP address
        arp_packet.extend_from_slice(&target_ip);
        
        // Create Ethernet frame
        let frame = EthernetFrame::new(
            [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // Broadcast
            interface.get_mac_address(),
            ETHERTYPE_ARP,
            arp_packet
        );
        
        // Send the frame
        interface.send_frame(&frame)?;
        
        // In a real implementation, you would now wait for a response
        // This is a placeholder
        Err("ARP response handling not implemented")
    }
}

/// Initialize the network subsystem
pub fn init() -> Result<NetworkManager, &'static str> {
    NetworkManager::new()
}