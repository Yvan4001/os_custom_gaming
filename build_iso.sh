#!/bin/bash
set -e

# --- Configuration ---
PROJECT_NAME="FluxGrid"
KERNEL_TARGET_NAME="fluxgridOs"
TARGET_SPEC="x86_64-fluxgrid_os"
ISO_NAME="$PROJECT_NAME.iso"
ISO_DIR="iso_root"

# --- Build Rust Kernel ---
echo "🚀 Building Rust kernel ELF..."
cargo +nightly build --target $TARGET_SPEC.json
if [ $? -ne 0 ]; then echo "❌ Rust kernel compilation failed"; exit 1; fi
echo "✅ Rust kernel compiled successfully"

KERNEL_ELF_PATH="target/$TARGET_SPEC/debug/$KERNEL_TARGET_NAME"
if [ ! -f "$KERNEL_ELF_PATH" ]; then
    echo "❌ ERROR: Kernel ELF not found at $KERNEL_ELF_PATH"
    exit 1
fi
echo "Kernel ELF found: $KERNEL_ELF_PATH"

echo "Compile bootloader..."
nasm -f elf64 bootloader/boot.asm -o bootloader/boot.elf

# Copy the bootloader ELF to the ISO
cp bootloader/boot.elf "$ISO_DIR/boot/boot.elf"

# --- Create Bootable ISO with GRUB ---
echo "📀 Creating ISO file with GRUB..."
mkdir -p "$ISO_DIR/boot/grub"

# Copy the kernel to the correct location for GRUB
cp "$KERNEL_ELF_PATH" "$ISO_DIR/boot/$KERNEL_TARGET_NAME.elf"

# Create the GRUB configuration file
cat > "$ISO_DIR/boot/grub/grub.cfg" << EOF
set timeout=3
set default=0

menuentry "FluxGrid Bootloader" {
    echo "GRUB: Loading custom bootloader..."
    multiboot2 /boot/boot.elf
    boot
}
EOF

echo "   Generating bootable ISO image..."
grub-mkrescue -o "$ISO_NAME" "$ISO_DIR"

if [ $? -ne 0 ]; then
    echo "❌ ISO creation with grub-mkrescue failed."
    exit 1
fi

echo "✅ ISO created successfully: $ISO_NAME"
echo ""
echo "🚀 To run the created ISO directly with QEMU:"
echo "   qemu-system-x86_64 -m 1G -cdrom $ISO_NAME"
echo ""

# --- Example Docker Commands (for reference) ---
echo "🐳 To run the ISO with QEMU inside Docker:"
echo "   (Ensure Docker is installed and you might need sudo)"
echo "   sudo docker run --rm -v \"\$(pwd):/data\" \\"
echo "       --privileged \\"
echo "       -p 5900:5900 \\"
echo "       tianon/qemu \\"
echo "       qemu-system-x86_64 -m 1G -cdrom /data/$ISO_NAME -display vnc=0.0.0.0:0"
echo ""

echo "🔧 To debug the ISO boot process with GDB:"
echo "   1. Run QEMU with the GDB server enabled:"
echo "   sudo docker run --rm -v \"\$(pwd):/data\" \\"
echo "       --privileged \\"
echo "       -p 5900:5900 -p 1234:1234 \\"
echo "       tianon/qemu \\"
echo "       qemu-system-x86_64 -m 1G -cdrom /data/$ISO_NAME -S -gdb tcp::1234"
echo "   2. In another terminal, start GDB:"
echo "      gdb"
echo "      (gdb) target remote localhost:1234"
echo "      (gdb) file target/x86_64-fluxgrid_os/debug/$KERNEL_TARGET_NAME"
echo "      (gdb) break _start"
echo "      (gdb) continue"

