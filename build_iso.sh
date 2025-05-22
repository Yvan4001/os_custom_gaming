#!/bin/bash
set -e

# Variable configuration
PROJECT_NAME="FluxGrid"
# KERNEL_ELF_NAME should match the name of your [[bin]] target in os_gaming/Cargo.toml
# or your package name if it's a lib with an entry_point.
# Assuming your binary is named "fluxgridOs" as per your Cargo.toml [[bin]] section.
KERNEL_TARGET_NAME="fluxgridOs"
TARGET_SPEC="x86_64-fluxgrid_os"
TARGET_JSON="$TARGET_SPEC.json" # Assumes this target spec JSON file exists
ASM_BOOTLOADER_SRC="bootloader/boot.asm" # Path to your assembly bootloader
ASM_BOOTLOADER_BIN="bootloader/boot.bin"   # Output for compiled assembly bootloader
ISO_NAME="$PROJECT_NAME.iso" # Define ISO name as a variable

echo "🔧 Configuring build environment..."
# Ensure NASM is installed for assembling the bootloader
if ! command -v nasm &> /dev/null
then
    echo "❌ NASM could not be found. Please install NASM."
    exit 1
fi

echo "🛠️ Assembling custom bootloader (${ASM_BOOTLOADER_SRC})..."
nasm "${ASM_BOOTLOADER_SRC}" -f bin -o "${ASM_BOOTLOADER_BIN}"
if [ $? -ne 0 ]; then
    echo "❌ Custom assembly bootloader compilation failed"
    exit 1
fi
echo "✅ Custom assembly bootloader compiled successfully: ${ASM_BOOTLOADER_BIN}"
echo "   You can test it with: qemu-system-x86_64 -fda ${ASM_BOOTLOADER_BIN}"

echo "🚀 Building Rust kernel ELF (${KERNEL_TARGET_NAME})..."
# The -Z build-std flags should be in your .cargo/config.toml for the target
# Ensure your os_gaming/Cargo.toml has a [[bin]] section with name = "fluxgridOs"
# or if your entry point is in lib.rs, that it's configured to produce an executable.
cargo +nightly build --target $TARGET_JSON # Add --release for release build

if [ $? -ne 0 ]; then
    echo "❌ Rust kernel compilation failed"
    exit 1
fi
echo "✅ Rust kernel compiled successfully"

# Define the path to the compiled kernel ELF
# This path assumes you are building in debug mode.
# If you use `cargo build --target $TARGET_JSON --release`, change "debug" to "release".
KERNEL_ELF_PATH="target/$TARGET_SPEC/debug/$KERNEL_TARGET_NAME"
if [ ! -f "$KERNEL_ELF_PATH" ]; then
    echo "❌ ERROR: Kernel ELF not found at $KERNEL_ELF_PATH"
    echo "   Check your Cargo.toml [[bin]] name and build mode (debug/release)."
    exit 1
fi

# Optional: Check kernel file size (a very small or zero size indicates a problem)
KERNEL_SIZE_BYTES=$(stat -c%s "$KERNEL_ELF_PATH")
if [ "$KERNEL_SIZE_BYTES" -lt "10000" ]; then # Arbitrary small size check (e.g., 10KB)
    echo "⚠️ WARNING: Kernel ELF file size is very small ($KERNEL_SIZE_BYTES bytes). This might indicate a build issue."
fi
echo "   Kernel ELF found: $KERNEL_ELF_PATH (Size: $KERNEL_SIZE_BYTES bytes)"


echo "📀 Creating ISO file with GRUB..."
# Create ISO directory structure
mkdir -p iso_root/boot/grub

# Copy Rust kernel ELF to ISO directory, using a consistent name
cp "$KERNEL_ELF_PATH" "iso_root/boot/${KERNEL_TARGET_NAME}.elf"

# Create GRUB configuration
cat > iso_root/boot/grub/grub.cfg << EOF
set timeout=3
set default=0

# Check if GRUB can see the file
if [ -e /boot/${KERNEL_TARGET_NAME}.elf ]; then
    echo "GRUB: Found /boot/${KERNEL_TARGET_NAME}.elf"
else
    echo "GRUB Error: Kernel file /boot/${KERNEL_TARGET_NAME}.elf not found by GRUB!"
    # Halt or loop here to make it obvious in GRUB if the file isn't seen
    # sleep 60
    # halt
fi

menuentry "$PROJECT_NAME OS (Rust Kernel - ${KERNEL_TARGET_NAME}.elf)" {
    echo "GRUB: Attempting to load /boot/${KERNEL_TARGET_NAME}.elf with multiboot2..."
    multiboot2 /boot/${KERNEL_TARGET_NAME}.elf
    echo "GRUB: multiboot2 command executed. Attempting to boot..."
    boot
    echo "GRUB Error: 'boot' command returned. Kernel failed to load or run."
}

menuentry "GRUB Debug - Check File" {
    echo "Checking for /boot/${KERNEL_TARGET_NAME}.elf ..."
    ls /boot/
    # Halt to see output
    # halt
}
EOF

# Generate ISO using GRUB
grub-mkrescue -o $ISO_NAME iso_root

if [ $? -ne 0 ]; then
    echo "❌ ISO creation failed"
    exit 1
fi

echo "✅ ISO created successfully: $ISO_NAME"
echo ""
echo "🚀 To run the ISO with QEMU (boots your Rust kernel via GRUB):"
echo "qemu-system-x86_64 -m 1G -serial stdio -cdrom $ISO_NAME -no-reboot -no-shutdown"
echo ""
echo "🚀 To test your custom assembly bootloader directly (if it's a floppy image):"
echo "qemu-system-x86_64 -fda ${ASM_BOOTLOADER_BIN} -no-reboot -no-shutdown"
echo ""

# MODIFIED: Updated Docker section
echo "🐳 To run the ISO with QEMU inside Docker (VNC display):"
echo "   (Ensure Docker is installed and running. You might need sudo.)"
echo "   Make sure no other service is using port 5900 on your host."
echo ""
echo "   Command to run:"
echo "   sudo docker run --rm -v \"\$(pwd):/data\" \\"
echo "       --privileged \\"
echo "       -p 5900:5900 \\"
echo "       tianon/qemu \\"
echo "       qemu-system-x86_64 -m 1G \\"
echo "       -display vnc=0.0.0.0:0 \\"
echo "       -serial stdio \\"
echo "       -cdrom /data/$ISO_NAME \\"
echo "       -no-reboot \\"
echo "       -no-shutdown"
echo ""
echo "   Then, connect with a VNC viewer to: localhost:5900"
echo ""
