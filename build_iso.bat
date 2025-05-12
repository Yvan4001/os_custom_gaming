@echo off
setlocal enabledelayedexpansion

:: Set proper working directory
cd /d %~dp0

:: Variable configuration
set RUSTFLAGS=-Z force-unstable-if-unmarked
set PROJECT_NAME=fluxGridOs
set TARGET_SPEC=x86_64-fluxGridOs

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

:: Copy memory map to the correct location for the build process
echo 📝 Setting up memory map...
mkdir target 2>NUL
if not exist "memory-map.txt" (
    echo Creating memory-map.txt...
    (
        echo /* Memory map for custom OS */
        echo MEMORY
        echo {
        echo   /* Reserved region - problematic address we want to avoid */
        echo   RESERVED (r^) ^: ORIGIN = 0x400000, LENGTH = 0x1000
        echo   /* Code region - where our kernel code will be placed */
        echo   CODE (rx^) ^: ORIGIN = 0x100000, LENGTH = 0x300000
        echo   /* Data region - for stack, heap, etc */
        echo   DATA (rw^) ^: ORIGIN = 0x500000, LENGTH = 0x300000
        echo }
        echo SECTIONS
        echo {
        echo   /* Place .text in CODE region, not in RESERVED */
        echo   .text ^: { *(.text*^) } ^> CODE
        echo   /* Place other sections in DATA region */
        echo   .rodata ^: { *(.rodata*^) } ^> DATA
        echo   .data ^: { *(.data*^) } ^> DATA
        echo   .bss ^: { *(.bss*^) } ^> DATA
        echo }
    ) > memory-map.txt
)
copy memory-map.txt target\memory-map.txt
set RUSTFLAGS=%RUSTFLAGS% -C link-arg=--script=%CD%\target\memory-map.txt

echo 🚀 Building kernel...
:: Build with core library from source
cargo build -Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem --target %TARGET_SPEC%.json -Z unstable-options --features bootloader-custom-config

if errorlevel 1 (
    echo ❌ Kernel build failed
    exit /b 1
)

echo 📦 Creating bootimage...
:: Create bootimage with the same flags and custom config
cargo bootimage --target %TARGET_SPEC%.json --features bootloader-custom-config
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

echo 📋 Available QEMU test commands:

echo 🔵 Standard boot command:
echo qemu-system-x86_64 -drive format=raw,file=target/%TARGET_SPEC%/debug/bootimage-%PROJECT_NAME%.bin

echo 🔵 Enhanced boot command with memory protection:
echo qemu-system-x86_64 -m 256M -machine q35 -drive format=raw,file=target/%TARGET_SPEC%/debug/bootimage-%PROJECT_NAME%.bin

echo 🔵 Debug boot command with memory exclusion:
echo qemu-system-x86_64 -m 256M -machine q35 -no-reboot -device isa-debug-exit,iobase=0xf4,iosize=0x04 -drive format=raw,file=target/%TARGET_SPEC%/debug/bootimage-%PROJECT_NAME%.bin

echo 🔵 Full diagnostic boot command:
echo qemu-system-x86_64 -m 256M -machine q35 -no-reboot -device isa-debug-exit,iobase=0xf4,iosize=0x04 -serial stdio -monitor stdio -d int,guest_errors,cpu_reset -D qemu.log -drive format=raw,file=target/%TARGET_SPEC%/debug/bootimage-%PROJECT_NAME%.bin


echo 📦 Check if bootable image created:
dir target\%TARGET_SPEC%\debug\bootimage-%PROJECT_NAME%.bin
endlocal