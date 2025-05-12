#!/bin/bash
set -e

# Variable configuration
PROJECT_NAME="FluxGrid"
TARGET_SPEC="x86_64-fluxgrid_os"
TARGET_JSON="$TARGET_SPEC.json"

echo "🔧 Configuring build environment..."

# Ensure we have nightly toolchain components
rustup component add rust-src --toolchain nightly
rustup component add llvm-tools-preview --toolchain nightly
cargo install bootimage

# Ensure ISO creation dependencies
apt-get update && apt-get install -y xorriso grub-pc-bin mtools

# Force system to use APT version of QEMU
export PATH="/usr/bin:$PATH"
# Clear snap-specific library paths if present
unset LD_LIBRARY_PATH
unset SNAP

echo "🚀 Building kernel with bootimage..."
cargo +nightly bootimage \
    -Z build-std=core,alloc,compiler_builtins \
    -Z build-std-features=compiler-builtins-mem \
    --target $TARGET_JSON \
    -Z unstable-options

if [ $? -ne 0 ]; then
    echo "❌ Bootimage creation failed"
    exit 1
fi

echo "✅ Bootimage created successfully"

echo "📀 Creating ISO file..."
# Create ISO directory structure
mkdir -p iso_root/boot/grub

# Copy bootimage to ISO directory
cp target/$TARGET_SPEC/debug/bootimage-fluxgridOs.bin iso_root/boot/

# Create GRUB configuration
cat > iso_root/boot/grub/grub.cfg << EOF
set timeout=5
set default=0

menuentry "$PROJECT_NAME OS" {
    multiboot2 /boot/bootimage-fluxgridOs.bin
    boot
}
EOF

# Generate ISO
grub-mkrescue -o $PROJECT_NAME.iso iso_root

if [ $? -ne 0 ]; then
    echo "❌ ISO creation failed"
    exit 1
fi

echo "✅ ISO created successfully: $PROJECT_NAME.iso"

echo "🚀 Show os when docker launch"
echo "vncviewer localhost:5900"

echo "🚀 Running in Docker image..."
sudo docker run --rm -v "$(pwd):/data" \
    --privileged \
    -p 5900:5900 \
    tianon/qemu \
    qemu-system-x86_64 -m 1G \
    -display vnc=0.0.0.0:0 \
    -serial stdio \
    -drive format=raw,file=/data/target/$TARGET_SPEC/debug/bootimage-fluxgridOs.bin \
    -no-reboot \
    -no-shutdown