@echo off
setlocal enabledelayedexpansion

:: Set proper working directory
cd /d %~dp0

:: Variable configuration
set RUSTFLAGS=-Z force-unstable-if-unmarked
set PROJECT_NAME=FluxGrid
set TARGET_SPEC=x86_64-fluxgrid_os
set TARGET_JSON=%TARGET_SPEC%.json

echo 🔧 Configuring build environment...

:: Check for required tools
where rustc >nul 2>&1
if errorlevel 1 (
    echo ❌ Rust is not installed or not in PATH
    exit /b 1
)

:: Ensure we have nightly toolchain and components
rustup default nightly
rustup component add rust-src
rustup component add llvm-tools-preview
cargo install bootimage

:: Check for ISO creation tools
where xorriso >nul 2>&1
if errorlevel 1 (
    echo ⚠️ xorriso not found - ISO creation might fail
)

echo 🚀 Building kernel with bootimage...
cargo +nightly bootimage ^
    -Z build-std=core,alloc,compiler_builtins ^
    -Z build-std-features=compiler-builtins-mem ^
    --target %TARGET_JSON% ^
    -Z unstable-options

if errorlevel 1 (
    echo ❌ Bootimage creation failed
    exit /b 1
)

echo ✅ Bootimage created successfully

echo 📀 Creating ISO file...
:: Create ISO directory structure
if exist iso_root rmdir /s /q iso_root
mkdir iso_root\boot\grub

:: Copy bootimage to ISO directory
copy target\%TARGET_SPEC%\debug\bootimage-fluxgridOs.bin iso_root\boot\

:: Create GRUB configuration
(
    echo set timeout=5
    echo set default=0
    echo.
    echo menuentry "%PROJECT_NAME% OS" {
    echo     multiboot2 /boot/bootimage-fluxgridOs.bin
    echo     boot
    echo }
) > iso_root\boot\grub\grub.cfg

:: Generate ISO (requires xorriso/grub-mkrescue for Windows)
grub-mkrescue -o %PROJECT_NAME%.iso iso_root

if errorlevel 1 (
    echo ❌ ISO creation failed
    exit /b 1
)

echo ✅ ISO created successfully: %PROJECT_NAME%.iso

echo 🚀 Show os when QEMU launches
echo Run the QEMU command below to launch your OS:

echo 🚀 For running in QEMU:
echo qemu-system-x86_64 -m 1G ^
    -serial stdio ^
    -drive format=raw,file=target\%TARGET_SPEC%\debug\bootimage-fluxgridOs.bin ^
    -no-reboot ^
    -no-shutdown

echo.
echo 🚀 For running with Docker (if Docker Desktop is installed):
echo docker run --rm -v "%CD%:/data" ^
    --privileged ^
    -p 5901:5901 ^
    tianon/qemu ^
    qemu-system-x86_64 -m 1G ^
    -display vnc=0.0.0.0:0 ^
    -serial stdio ^
    -drive format=raw,file=/data/target/%TARGET_SPEC%/debug/bootimage-fluxgridOs.bin ^
    -no-reboot ^
    -no-shutdown

echo.
echo Connect with VNC viewer at localhost:5900 after running the Docker command