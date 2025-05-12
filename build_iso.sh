#!/bin/bash
set -e

# Variable configuration
PROJECT_NAME="os_gaming"
TARGET_SPEC="x86_64-os_gaming"
TARGET_JSON="$TARGET_SPEC.json"

echo "🔧 Configuring build environment..."

# Setup memory map
echo "📝 Setting up memory map..."
mkdir -p target
if [ ! -f "memory-map.txt" ]; then
    echo "Creating memory-map.txt..."
    cat > memory-map.txt << EOF
/* Memory map for custom OS */
MEMORY
{
  /* Reserved region - problematic address we want to avoid */
  RESERVED (r) : ORIGIN = 0x400000, LENGTH = 0x1000
  /* Code region - where our kernel code will be placed */
  CODE (rx) : ORIGIN = 0x100000, LENGTH = 0x300000
  /* Data region - for stack, heap, etc */
  DATA (rw) : ORIGIN = 0x500000, LENGTH = 0x300000
}
SECTIONS
{
  /* Place .text in CODE region, not in RESERVED */
  .text : { *(.text*) } > CODE
  /* Place other sections in DATA region */
  .rodata : { *(.rodata*) } > DATA
  .data : { *(.data*) } > DATA
  .bss : { *(.bss*) } > DATA
}
EOF
fi
cp memory-map.txt target/memory-map.txt
export RUSTFLAGS="-Z force-unstable-if-unmarked -C link-arg=--script=$(pwd)/target/memory-map.txt"

# Ensure we have nightly toolchain components
rustup component add rust-src --toolchain nightly
rustup component add llvm-tools-preview --toolchain nightly

echo "🚀 Building kernel..."
cargo +nightly build \
    -Z build-std=core,alloc,compiler_builtins \
    -Z build-std-features=compiler-builtins-mem \
    --target $TARGET_JSON \
    -Z unstable-options \
    --features bootloader-custom-config

if [ $? -ne 0 ]; then
    echo "❌ Kernel build failed"
    exit 1
fi

echo "📦 Creating bootimage..."
cargo +nightly bootimage --target $TARGET_JSON --features bootloader-custom-config

if [ $? -ne 0 ]; then
    echo "❌ Bootimage creation failed"
    exit 1
fi

# --- ISO Creation Steps ---
echo "📁 Preparing directories..."
BUILD_DIR="target/$TARGET_SPEC/debug"
ISO_DIR="$BUILD_DIR/iso"
BOOT_DIR="$ISO_DIR/boot"
GRUB_DIR="$BOOT_DIR/grub"

# Clean and create directories
rm -rf $ISO_DIR
mkdir -p $ISO_DIR $BOOT_DIR $GRUB_DIR

# Copy the bootimage
cp $BUILD_DIR/bootimage-$PROJECT_NAME.bin $BOOT_DIR/kernel.bin

# Create GRUB config
cat > $GRUB_DIR/grub.cfg << EOF
set timeout=3
set default=0
menuentry "OS Gaming" {
    multiboot /boot/kernel.bin
    boot
}
EOF

if command -v grub-mkrescue &> /dev/null; then
    grub-mkrescue -o $PROJECT_NAME.iso $ISO_DIR
    echo "✅ ISO created successfully: $PROJECT_NAME.iso"
else
    echo "⚠️ grub-mkrescue not found, ISO not created."
    echo "✅ Build completed successfully"
fi

echo "📋 Available QEMU test commands:"

echo "🔵 Standard boot command:"
echo "qemu-system-x86_64 -drive format=raw,file=target/$TARGET_SPEC/debug/bootimage-$PROJECT_NAME.bin"

echo "🔵 Enhanced boot command with memory protection:"
echo "qemu-system-x86_64 -m 256M -machine q35 -drive format=raw,file=target/$TARGET_SPEC/debug/bootimage-$PROJECT_NAME.bin"

echo "🔵 Debug boot command with memory exclusion:"
echo "qemu-system-x86_64 -m 256M -machine q35 -no-reboot -device isa-debug-exit,iobase=0xf4,iosize=0x04 -drive format=raw,file=target/$TARGET_SPEC/debug/bootimage-$PROJECT_NAME.bin"

echo "🔵 Full diagnostic boot command:"
echo "qemu-system-x86_64 -m 256M -machine q35 -no-reboot -device isa-debug-exit,iobase=0xf4,iosize=0x04 -serial stdio -monitor stdio -d int,guest_errors,cpu_reset -D qemu.log -drive format=raw,file=target/$TARGET_SPEC/debug/bootimage-$PROJECT_NAME.bin"