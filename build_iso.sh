#!/bin/bash
set -e

# Variable configuration
PROJECT_NAME="fluxgridOs"
TARGET_SPEC="x86_64-fluxgrid_os"
TARGET_JSON="$TARGET_SPEC.json"

echo "🔧 Configuring build environment..."

# Ensure we have nightly toolchain components
rustup component add rust-src --toolchain nightly
rustup component add llvm-tools-preview --toolchain nightly
cargo install bootimage

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
    -drive format=raw,file=/data/target/x86_64-fluxgrid_os/debug/bootimage-fluxgridOs.bin