extern crate alloc;
use alloc::string::String;
use crate::alloc::string::ToString;
use alloc::vec::Vec;
use alloc::vec;
use core::sync::atomic::{AtomicBool, Ordering};
use x86_64::instructions::port::Port;

// Common Ethernet types
pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const ETHERTYPE_ARP: u16 = 0x0806;
pub const ETHERTYPE_IPV6: u16 = 0x86DD;

//Common Wireless types
pub const WIRELESS_IPV4: u16 = 0x0800;
pub const WIRELESS_ARP: u16 = 0x0806;
pub const WIRELESS_IPV6: u16 = 0x86DD;

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
    io_base: u16,  // Base I/O port for port-mapped devices
    mem_base: u64, // Base memory address for memory-mapped devices
    irq: u8,       // IRQ number
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

pub struct WirelessFrame {
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

impl WirelessFrame {
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
    pub fn setup_hardware(
        &mut self,
        io_base: u16,
        mem_base: u64,
        irq: u8,
    ) -> Result<(), &'static str> {
        self.io_base = io_base;
        self.mem_base = mem_base;
        self.irq = irq;

        // Create and initialize the appropriate driver
        match self.card_type {
            NetworkCardType::IntelE1000 => {
                let mut driver = E1000Driver::new(mem_base, irq, self.mac_address);
                driver.init()?;
                self.driver = NetworkDriverType::E1000(driver);
            }
            NetworkCardType::RealtekRTL8139 => {
                let mut driver = RTL8139Driver::new(io_base, irq);
                driver.init()?;
                // Get the MAC from the hardware
                self.mac_address = driver.get_mac_address();
                self.driver = NetworkDriverType::RTL8139(driver);
            }
            NetworkCardType::BroadcomBCM5751 => {
                let mut driver = RTL8139Driver::new(io_base, irq);
                driver.init()?;
                self.driver = NetworkDriverType::RTL8139(driver);
                return Err("Broadcom BCM5751 driver not implemented");
            }
            NetworkCardType::VirtIO => {
                // Placeholder for VirtIO driver
                let mut driver = RTL8139Driver::new(io_base, irq);
                driver.init()?;
                self.driver = NetworkDriverType::RTL8139(driver);
                return Err("VirtIO driver not implemented");
            }
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
    pub fn desactivate(&mut self) {
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

/// Transmit descriptor for E1000 driver
#[repr(C, packed)]
struct TxDescriptor {
    buffer_addr: u64, // Physical address of the transmit buffer
    length: u16,      // Length of data to transmit
    cso: u8,          // Checksum offset
    cmd: u8,          // Command field
    status: u8,       // Status field
    css: u8,          // Checksum start
    special: u16,     // Special field
}

/// Receive descriptor for E1000 driver
#[repr(C, packed)]
struct RxDescriptor {
    buffer_addr: u64, // Physical address of the receive buffer
    length: u16,      // Length of received data
    checksum: u16,    // Packet checksum
    status: u8,       // Status field
    errors: u8,       // Errors field
    special: u16,     // Special field
}

/// Intel E1000 network driver implementation
struct E1000Driver {
    mem_base: u64,
    irq: u8,
    mac_address: [u8; 6],
    tx_descs: Vec<TxDescriptor>,
    rx_descs: Vec<RxDescriptor>,
    tx_buffers: Vec<Vec<u8>>,
    rx_buffers: Vec<Vec<u8>>,
    tx_idx: usize,
    rx_idx: usize,
}

impl E1000Driver {
    const REG_TDT: u32 = 0x3818; // Transmit Descriptor Tail
    const REG_RDT: u32 = 0x2818; // Receive Descriptor Tail

    fn new(mem_base: u64, irq: u8, mac_address: [u8; 6]) -> Self {
        Self {
            mem_base,
            irq,
            mac_address,
            tx_descs: Vec::new(),
            rx_descs: Vec::new(),
            tx_buffers: Vec::new(),
            rx_buffers: Vec::new(),
            tx_idx: 0,
            rx_idx: 0,
        }
    }

    // Read a 32-bit register from the memory-mapped I/O space
    fn read_reg(&self, reg: u32) -> u32 {
        unsafe {
            let addr = self.mem_base as *const u32;
            core::ptr::read_volatile(addr.add((reg / 4) as usize))
        }
    }

    // Write a 32-bit value to a register in the memory-mapped I/O space
    fn write_reg(&self, reg: u32, value: u32) {
        unsafe {
            let addr = self.mem_base as *mut u32;
            core::ptr::write_volatile(addr.add((reg / 4) as usize), value);
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
        // Check if data exceeds MTU
        if data.len() > 1500 {
            return Err("Packet exceeds MTU");
        }

        // Get the current TX descriptor
        let tx_tail = self.read_reg(Self::REG_TDT) as usize % self.tx_descs.len();

        // Get length of tx_descs once to avoid borrowing issues later
        let tx_descs_len = self.tx_descs.len();

        // Check if descriptor is available (DD bit in status)
        if (self.tx_descs[tx_tail].status & 0x01) == 0 && self.tx_descs[tx_tail].cmd != 0 {
            // Descriptor is busy
            return Err("No transmit descriptors available");
        }

        // Copy data to the transmit buffer
        let tx_buffer = &mut self.tx_buffers[tx_tail];
        tx_buffer[..data.len()].copy_from_slice(data);

        // Update descriptor
        self.tx_descs[tx_tail].length = data.len() as u16;

        // Set command flags
        // EOP - End of Packet
        // IFCS - Insert FCS (CRC)
        // RS - Report Status
        self.tx_descs[tx_tail].cmd = 0x0B; // EOP | IFCS | RS

        // Clear status
        self.tx_descs[tx_tail].status = 0;

        // Update tail pointer to signal ready for transmit
        let new_tail = (tx_tail + 1) % tx_descs_len;
        self.write_reg(Self::REG_TDT, new_tail as u32);

        // For debugging in std mode
        #[cfg(feature = "std")]
        log::debug!("E1000: Transmitting {} bytes", data.len());

        // Optionally wait for transmit completion (for synchronous operation)
        // Note: In a real driver, you'd typically use interrupts instead
        // Optionally wait for transmit completion (for synchronous operation)
        // Note: In a real driver, you'd typically use interrupts instead
        let mut timeout = 10000;
        while (self.tx_descs[tx_tail].status & 0x01) == 0 && timeout > 0 {
            core::hint::spin_loop();
            timeout -= 1;
        }
        #[cfg(feature = "std")]
        log::warn!("E1000: Transmit timeout");

        Ok(())
    }

    fn receive(&mut self) -> Option<Vec<u8>> {
        // Check the current RX descriptor
        let rx_idx = self.rx_idx;
        let desc = &mut self.rx_descs[rx_idx];

        // Check if descriptor has data (DD bit in status)
        if (desc.status & 0x01) == 0 {
            // No packet available
            return None;
        }

        // Check for errors
        if (desc.errors & 0x1F) != 0 {
            // Packet has errors
            // Clear status and return descriptor to hardware
            desc.status = 0;

            // Update tail pointer
            let new_tail = (rx_idx + 1) % self.rx_descs.len();
            self.rx_idx = new_tail;
            self.write_reg(Self::REG_RDT, new_tail as u32);

            return None;
        }

        // Get the packet length
        let packet_len = desc.length as usize;

        // Create a new buffer and copy the data
        let mut packet = vec![0u8; packet_len];
        packet.copy_from_slice(&self.rx_buffers[rx_idx][..packet_len]);

        // Clear status and return descriptor to hardware
        desc.status = 0;

        // Update tail pointer
        let new_tail = (rx_idx + 1) % self.rx_descs.len();
        self.rx_idx = new_tail;
        self.write_reg(Self::REG_RDT, new_tail as u32);

        #[cfg(feature = "std")]
        log::debug!("E1000: Received packet of {} bytes", packet_len);

        Some(packet)
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
    tx_idx: u8, // Track the current transmit descriptor (0-3)
}

impl RTL8139Driver {
    const REG_COMMAND: u16 = 0x37; // Command register
    const REG_CAPR: u16 = 0x38; // Current Address of Packet Read
    const REG_CBR: u16 = 0x3A; // Current Buffer Address

    // Transmit status registers (4 separate transmit buffers)
    const REG_TSD0: u16 = 0x10; // Transmit Status of Descriptor 0
    const REG_TSD1: u16 = 0x14; // Transmit Status of Descriptor 1
    const REG_TSD2: u16 = 0x18; // Transmit Status of Descriptor 2
    const REG_TSD3: u16 = 0x1C; // Transmit Status of Descriptor 3

    // Transmit start address registers
    const REG_TSAD0: u16 = 0x20; // Transmit Start Address of Descriptor 0
    const REG_TSAD1: u16 = 0x24; // Transmit Start Address of Descriptor 1
    const REG_TSAD2: u16 = 0x28; // Transmit Start Address of Descriptor 2
    const REG_TSAD3: u16 = 0x2C; // Transmit Start Address of Descriptor 3

    // Receive status and configuration
    const REG_ROK: u16 = 0x3E; // Receive OK bit in ISR
    const REG_RCR: u16 = 0x44; // Receive Configuration Register

    // Other useful constants
    const TSD_TOK: u32 = 0x00008000; // Transmit OK status
    const TSD_OWN: u32 = 0x00002000; // Owner bit (0 = ready for TX)
    const RX_OK: u16 = 0x0001; // Receive OK flag
    const TX_OK: u16 = 0x0004; // Transmit OK flag
    const RX_BUF_SIZE: usize = 8192; // Receive buffer size (8K)

    fn new(io_base: u16, irq: u8) -> Self {
        Self {
            io_base,
            irq,
            rx_buffer: Vec::with_capacity(8192), // 8K receive buffer
            rx_buffer_offset: 0,
            tx_idx: 0, // Initialize transmit descriptor index to 0
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
        // Check packet size - RTL8139 has limit of 1792 bytes per packet
        if data.len() > 1792 {
            return Err("Packet too large for RTL8139");
        }

        // Get the current transmit descriptor
        let tx_idx = self.tx_idx as usize;

        // Get the register addresses for this descriptor
        let tsd_reg = match tx_idx {
            0 => Self::REG_TSD0,
            1 => Self::REG_TSD1,
            2 => Self::REG_TSD2,
            3 => Self::REG_TSD3,
            _ => unreachable!(),
        };

        let tsad_reg = match tx_idx {
            0 => Self::REG_TSAD0,
            1 => Self::REG_TSAD1,
            2 => Self::REG_TSAD2,
            3 => Self::REG_TSAD3,
            _ => unreachable!(),
        };

        // Check if this descriptor is available (not owned by the NIC)
        let status = self.read_reg32(tsd_reg);
        if (status & Self::TSD_OWN) != 0 {
            return Err("Transmit descriptor busy");
        }

        // Create a transmit buffer
        let mut tx_buffer = Vec::with_capacity(data.len());
        tx_buffer.extend_from_slice(data);

        // Get physical address of the buffer
        // In a real OS, you'd use DMA-capable memory and get the physical address
        let buffer_addr = tx_buffer.as_ptr() as u32;

        // Write the buffer address to the descriptor
        self.write_reg32(tsad_reg, buffer_addr);

        // Write the status (packet length and OWN bit) to start transmission
        // Early TX threshold is set to the whole packet
        let tx_cmd = ((data.len() as u32) & 0x1FFF) | 0x00800000; // Set OWN, EOR bits
        self.write_reg32(tsd_reg, tx_cmd);

        // Move to the next descriptor for the next packet
        self.tx_idx = (self.tx_idx + 1) % 4;

        // In a real driver, you would either:
        // 1. Wait for the TOK (Transmit OK) interrupt, or
        // 2. Poll for completion if synchronous operation is needed

        // For this example, we'll do a simple poll with timeout
        let mut timeout = 1000;
        while timeout > 0 {
            let status = self.read_reg32(tsd_reg);
            if (status & Self::TSD_TOK) != 0 {
                // Transmission successful
                #[cfg(feature = "std")]
                log::debug!("RTL8139: Sent {} bytes successfully", data.len());

                return Ok(());
            }

            // Small delay
            for _ in 0..100 {
                core::hint::spin_loop();
            }

            timeout -= 1;
        }

        // Timeout occurred
        #[cfg(feature = "std")]
        log::warn!("RTL8139: Transmit timeout");

        // We'll still return Ok since the packet was queued
        Ok(())
    }

    fn receive(&mut self) -> Option<Vec<u8>> {
        // Check if there's a packet in the buffer
        // The Command Register bit 0 is BUF_EMPTY which is set when the buffer is empty
        if (self.read_reg8(Self::REG_COMMAND) & 0x01) != 0 {
            return None; // No packet available
        }

        // Get the current read pointer
        let capr = self.read_reg16(Self::REG_CAPR);
        let cbr = self.read_reg16(Self::REG_CBR);

        // Calculate how much data is available in the buffer
        let buffer_offset = self.rx_buffer_offset;

        // Read the packet header
        // RTL8139 packet format: status (2 bytes) + length (2 bytes) + packet data
        let status = (self.rx_buffer[buffer_offset] as u16)
            | ((self.rx_buffer[buffer_offset + 1] as u16) << 8);

        let length = (self.rx_buffer[buffer_offset + 2] as u16)
            | ((self.rx_buffer[buffer_offset + 3] as u16) << 8);

        // Validate packet
        if (status & Self::RX_OK) == 0 {
            // Packet has errors, skip it
            // Update buffer offset (4 bytes header + packet length, aligned to 4 bytes)
            let packet_size = ((length + 4 + 3) & !3) as usize; // Align to 4 bytes
            self.rx_buffer_offset = (buffer_offset + packet_size) % self.rx_buffer.len();

            // Update the CAPR register to tell the NIC we've processed this data
            // The CAPR should be set to the offset - 0x10 (to account for the WRAP bit)
            self.write_reg16(
                Self::REG_CAPR,
                ((self.rx_buffer_offset - 0x10) as u16) & 0xFFFF,
            );

            return None;
        }

        // Extract the packet data (skip the 4-byte header)
        let data_start = buffer_offset + 4;
        let data_end = data_start + length as usize;

        // Create a new buffer for the packet
        let mut packet = Vec::with_capacity(length as usize);

        // Copy the packet data, handling wrap-around if needed
        if data_end <= self.rx_buffer.len() {
            // No wrap-around
            packet.extend_from_slice(&self.rx_buffer[data_start..data_end]);
        } else {
            // Packet wraps around the buffer end
            let first_part_len = self.rx_buffer.len() - data_start;
            packet.extend_from_slice(&self.rx_buffer[data_start..]);
            packet.extend_from_slice(&self.rx_buffer[0..(data_end - self.rx_buffer.len())]);
        }

        // Update buffer offset (4 bytes header + packet length, aligned to 4 bytes)
        let packet_size = ((length + 4 + 3) & !3) as usize; // Align to 4 bytes
        self.rx_buffer_offset = (buffer_offset + packet_size) % self.rx_buffer.len();

        // Update the CAPR register to tell the NIC we've processed this data
        // The CAPR should be set to the offset - 0x10 (to account for the WRAP bit)
        self.write_reg16(
            Self::REG_CAPR,
            ((self.rx_buffer_offset - 0x10) as u16) & 0xFFFF,
        );

        #[cfg(feature = "std")]
        log::debug!("RTL8139: Received packet of {} bytes", length);

        Some(packet)
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
            NetworkCardType::IntelE1000,
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
            NetworkCardType::Unknown,
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
        let default_interface = if !interfaces.is_empty() {
            Some(0)
        } else {
            None
        };

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
        self.interfaces
            .iter_mut()
            .find(|iface| iface.get_name() == name)
    }

    /// Get a reference to an interface by name
    pub fn get_interface(&self, name: &str) -> Option<&NetworkInterface> {
        self.interfaces
            .iter()
            .find(|iface| iface.get_name() == name)
    }

    /// Get the default interface
    pub fn get_default_interface(&self) -> Option<&NetworkInterface> {
        self.default_interface.map(|idx| &self.interfaces[idx])
    }

    /// Set the default interface by name
    pub fn set_default_interface(&mut self, name: &str) -> Result<(), &'static str> {
        let idx = self
            .interfaces
            .iter()
            .position(|iface| iface.get_name() == name)
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
            arp_packet,
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

pub fn handle_gaming_interrupt() {
    // Handle gaming interrupts here
    // This is a placeholder for actual interrupt handling
    #[cfg(feature = "std")]
    log::info!("Gaming interrupt handled");
}

pub fn shutdown() {
    // Shutdown the network subsystem
    #[cfg(feature = "std")]
    log::info!("Network subsystem shutting down");
    
    // Create a network manager instance first
    if let Ok(mut network_manager) = NetworkManager::new() {
        // First collect all interface names to avoid borrowing issues
        let interface_names: Vec<String> = network_manager
            .get_interfaces()
            .iter()
            .map(|iface| iface.get_name().to_string())
            .collect();
            
        // Then deactivate all interfaces using the collected names
        for name in interface_names {
            if let Some(interface) = network_manager.get_interface_mut(&name) {
                interface.desactivate();
            }
        }
    }
}