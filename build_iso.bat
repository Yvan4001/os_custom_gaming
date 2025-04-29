@echo off
setlocal enabledelayedexpansion

:: Variable configuration
set PROJECT_NAME=os_gaming
set TARGET_SPEC=x86_64-os_gaming

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

echo 🚀 Building kernel...
:: Build with core library from source
cargo build -Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem --target %TARGET_SPEC%.json -Z unstable-options
if errorlevel 1 (
    echo ❌ Kernel build failed
    exit /b 1
)

echo 📦 Creating bootimage...
:: Create bootimage with the same flags
cargo bootimage --target %TARGET_SPEC%.json
if errorlevel 1 (
    echo ❌ Bootimage creation failed
    exit /b 1
)

:: --- ISO Creation Steps ---
echo 📁 Preparing directories...
set BUILD_DIR=target\%TARGET_SPEC%\debug
set ISO_DIR=%BUILD_DIR%\iso
set BOOT_DIR=%ISO_DIR%\boot
set GRUB_DIR=%BOOT_DIR%\grub

:: Clean and create directories
if exist %ISO_DIR% rd /s /q %ISO_DIR%
mkdir %ISO_DIR% %BOOT_DIR% %GRUB_DIR%

:: Copy the bootimage
copy %BUILD_DIR%\bootimage-%PROJECT_NAME%.bin %BOOT_DIR%\kernel.bin
if errorlevel 1 (
    echo ❌ Failed to copy kernel binary
    exit /b 1
)

:: Create GRUB config
(
    echo set timeout=3
    echo set default=0
    echo menuentry "OS Gaming" {
    echo     multiboot /boot/kernel.bin
    echo     boot
    echo }
) > %GRUB_DIR%\grub.cfg

echo ✅ Build completed successfully
echo You can test the kernel using:
echo qemu-system-x86_64 -drive format=raw,file=target/%TARGET_SPEC%/debug/bootimage-%PROJECT_NAME%.bin

endlocal