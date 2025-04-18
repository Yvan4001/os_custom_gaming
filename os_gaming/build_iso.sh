#!/bin/bash
set -e

# For the kernel (no_std) build
echo "Building kernel..."
# Ensure we're using nightly
RUSTFLAGS="-C link-arg=-nostartfiles" cargo +nightly clean

# Build with std disabled and alloc enabled
RUSTFLAGS="-C link-arg=-nostartfiles -C link-arg=-nodefaultlibs -C link-arg=-nostdlib" \
cargo +nightly build -Zbuild-std=core,compiler_builtins,alloc --target x86_64-unknown-none

# Directory setup
BUILD_DIR="build"
ISO_DIR="$BUILD_DIR/iso"
BOOT_DIR="$ISO_DIR/boot"
GRUB_DIR="$BOOT_DIR/grub"

# Make sure directories exist
mkdir -p $ISO_DIR $BOOT_DIR $GRUB_DIR

# Copy the kernel binary
cp target/x86_64-unknown-none/release/os_gaming $BOOT_DIR/kernel.bin

# Create GRUB config file
cat > $GRUB_DIR/grub.cfg << EOF
set timeout=3
set default=0

menuentry "OS Gaming" {
    multiboot /boot/kernel.bin
    boot
}
EOF

# Create the ISO
grub-mkrescue -o os_gaming.iso $ISO_DIR

echo "ISO created: os_gaming.iso"