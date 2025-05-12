# FluxGridOS

A custom operating system kernel optimized for gaming, written in Rust.

## Overview

OS Custom Gaming is a from-scratch operating system kernel designed specifically for gaming environments. Built with Rust's safety guarantees and zero-cost abstractions, this project aims to create a minimal, high-performance OS that dedicates maximum resources to gaming applications while providing necessary system services.

## Features

- **Bare-metal Rust kernel**: No underlying OS, direct hardware control
- **Memory optimized**: Custom memory management tuned for gaming workloads
- **GPU-focused drivers**: Direct hardware access to graphics cards (NVIDIA support)
- **Low-latency audio**: Custom sound subsystem for minimal audio latency
- **Configurable**: Binary configuration system for persistent settings
- **Gaming profiles**: Support for different optimization profiles

## Architecture
```bash
┌────────────────────────────────────────────────┐
│                  Applications                   │
└───────────────────────┬────────────────────────┘
│
┌───────────────────────┴────────────────────────┐
│                      GUI                        │
└───────────────────────┬────────────────────────┘
│
┌───────────────────────┴────────────────────────┐
│                  FluxGrid Kernel               │
├────────────────┬──────────────┬────────────────┤
│ Memory Manager │  Scheduler   │ Device Drivers  │
├────────────────┴──────────────┴────────────────┤
│              Hardware Abstraction Layer         │
└────────────────────────┬─────────────────────┬─┘
│                     │
┌────────────────────────┴─┐   ┌───────────────┴─┐
│     System Hardware      │   │       BIOS       │
└──────────────────────────┘   └─────────────────┘
```

## High-Level Architecture

### Kernel Structure

The OS Custom Gaming kernel follows a modular design with several key components:

- **Boot Process**
  - Uses a custom bootloader to initialize hardware
  - Sets up initial paging and memory structures
  - Transfers control to the Rust kernel entry point (`_start`)

- **Core Kernel**
  - Interrupt handling (IDT setup, ISRs)
  - CPU management (GDT setup, TSS)
  - Core kernel services

- **Memory Subsystem**
  - Physical memory management (`physical.rs`)
  - Virtual memory management (`virtual.rs`)
  - Heap allocation (`allocator.rs`)
  - Memory-mapped I/O support

- **Device Drivers**
  - GPU drivers (specialized NVIDIA support)
  - Input devices (keyboard, mouse)
  - Sound hardware
  - Storage controllers
  - VGA text mode for debugging
  - HDMI output

- **System Services**
  - Configuration management (`config/mod.rs`)
  - Power management
  - Virtual filesystem abstraction

- **GUI Layer**
  - Window management
  - Graphics rendering
  - Font handling
  - Theme support

### Memory Management

The memory management subsystem is composed of several specialized components:

- **Physical Memory Manager**
  - Tracks available physical memory
  - Handles frame allocation and deallocation
  - Maintains memory maps

- **Virtual Memory Manager**
  - Creates and manages page tables
  - Provides virtual address space allocation
  - Handles memory protection and permissions

- **Heap Allocator**
  - Implements the global allocator trait
  - Uses a linked list allocator for dynamic memory
  - Provides memory allocation services to the kernel

- **DMA Manager**
  - Handles direct memory access for devices
  - Ensures memory coherence

### Driver Subsystem

The driver architecture follows a layered approach:

```bash
┌─────────────────────────────────────────┐
│            Driver Interface              │
└───────────────┬─────────────────────────┘
│
┌───────────────┼─────────────────────────┐
│  ┌─────────┐  │  ┌─────────┐  ┌────────┐ │
│  │   GPU   │  │  │  Sound  │  │  Input │ │
│  └─────────┘  │  └─────────┘  └────────┘ │
│               │                          │
│  ┌─────────┐  │  ┌─────────┐  ┌────────┐ │
│  │  Video  │  │  │ Storage │  │  ACPI  │ │
│  └─────────┘  │  └─────────┘  └────────┘ │
└───────────────┴─────────────────────────┘
```

- **GPU Drivers**
  - Hardware-specific implementations (NVIDIA)
  - Memory management for VRAM
  - Command submission and synchronization

- **Sound System**
  - Audio buffer management
  - Waveform generation
  - Multiple channel support

- **Input Handling**
  - Keyboard and mouse drivers
  - Event processing
  - Gamepad support

### Configuration System

The configuration system uses bincode serialization to store system settings persistently:

- Serializes `SystemConfig` structures to binary format
- Handles loading/saving of configuration data
- Provides defaults when no configuration exists
- Supports gaming profiles with different optimization settings

## Building

### Prerequisites

- Rust toolchain (nightly)
- QEMU (for testing)
- x86_64 cross-compilation tools
- Cargo and build tools

### Build Steps

1. Clone the repository:

   ```sh
   git clone https://github.com/Yvan4001/os-custom-gaming.git
   cd os-custom-gaming
   ``` 

2. Build the kernel:

   ```sh
   cargo build --target x86_64-fluxGridOs.json
   ```

3. Create bootable disk image:

   ```sh
   ./build_iso.sh
   ```
   or
   ```sh
   ./build_iso.bat
   ```

## Current Status

### OS Custom Gaming is in active development. Current working components:

    Boot sequence
    Memory management
    VGA text output
    Keyboard input
    Basic GPU driver structure
    Configuration system
    Complete sound system
    Full GPU acceleration
    Game-optimized scheduler
    Storage subsystem

### Contributing

Contributions are welcome! Please check the issues list for tasks that need help, or suggest new features through the issue tracker.

    Fork the repository
    Create your feature branch (git checkout -b feature/amazing-feature)
    Commit your changes (git commit -m 'Add some amazing feature')
    Push to the branch (git push origin feature/amazing-feature)
    Open a Pull Request

## License

This project is distributed under a custom open source license that restricts distribution channels and commercial use. See the [LICENSE](LICENSE) file for details.
