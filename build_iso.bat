@echo off
setlocal

REM Variable configuration
set "PROJECT_NAME=FluxGrid"
REM KERNEL_ELF_NAME should match the name of your [[bin]] target in os_gaming/Cargo.toml
REM or your package name if it's a lib with an entry_point.
REM Assuming your binary is named "fluxgridOs" as per your Cargo.toml [[bin]] section.
set "KERNEL_TARGET_NAME=fluxgridOs"
set "TARGET_SPEC=x86_64-fluxgrid_os"
set "TARGET_JSON=%TARGET_SPEC%.json" REM Assumes this target spec JSON file exists
set "ASM_BOOTLOADER_SRC=bootloader\boot.asm" REM Path to your assembly bootloader
set "ASM_BOOTLOADER_BIN=bootloader\boot.bin" REM Output for compiled assembly bootloader
set "ISO_NAME=%PROJECT_NAME%.iso" REM Define ISO name as a variable

echo 🔧 Configuring build environment...
REM Ensure NASM is installed for assembling the bootloader
where nasm >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo ❌ NASM could not be found. Please install NASM.
    exit /b 1
)

echo 🛠️ Assembling custom bootloader (%ASM_BOOTLOADER_SRC%)...
nasm "%ASM_BOOTLOADER_SRC%" -f bin -o "%ASM_BOOTLOADER_BIN%"
if %ERRORLEVEL% neq 0 (
    echo ❌ Custom assembly bootloader compilation failed
    exit /b 1
)
echo ✅ Custom assembly bootloader compiled successfully: %ASM_BOOTLOADER_BIN%
echo   You can test it with: qemu-system-x86_64 -fda %ASM_BOOTLOADER_BIN%

echo 🛠️ Create an empty 1.44MB floppy disk image
REM This requires a 'dd' command available in your PATH.
REM Alternatively, for creating a zeroed file, fsutil can be used, but dd is needed for subsequent writes.
REM fsutil file createnew floppy.img 1474560
REM For dd:
dd if=/dev/zero of=floppy.img bs=512 count=2880
if %ERRORLEVEL% neq 0 (
    echo ❌ Failed to create floppy.img using dd. Make sure dd is in your PATH.
    echo    (You might need dd from Git for Windows, Cygwin, or a standalone port)
    exit /b 1
)

echo 🛠️ Write MBR (first 512 bytes) to sector 0
dd if=bootloader\boot.bin of=floppy.img conv=notrunc bs=512 count=1 seek=0
if %ERRORLEVEL% neq 0 (
    echo ❌ Failed to write MBR using dd.
    exit /b 1
)

echo 🛠️ Write Stage 2 (second 512 bytes) to sector 1
dd if=bootloader\boot.bin of=floppy.img conv=notrunc bs=512 count=1 skip=1 seek=1
if %ERRORLEVEL% neq 0 (
    echo ❌ Failed to write Stage 2 using dd.
    exit /b 1
)

echo 🛠️ Verify the MBR signature exists at the end of sector 0
REM This requires 'hexdump' and 'grep' (or 'findstr') available in your PATH.
REM Git for Windows includes these tools.
hexdump -C -s 510 -n 2 floppy.img | findstr "55 aa"
if %ERRORLEVEL% neq 0 (
    echo ⚠️ Could not verify MBR signature. 'hexdump' or 'findstr' might be missing or signature incorrect.
)

echo 🛠️ Verify Stage 2 starts at sector 1 (should show 'S' character)
REM This requires 'hexdump' available in your PATH.
hexdump -C -s 512 -n 16 floppy.img
if %ERRORLEVEL% neq 0 (
    echo ⚠️ Could not verify Stage 2 start. 'hexdump' might be missing.
)

echo 🚀 Building Rust kernel ELF (%KERNEL_TARGET_NAME%)...
REM The -Z build-std flags should be in your .cargo/config.toml for the target
REM Ensure your os_gaming/Cargo.toml has a [[bin]] section with name = "fluxgridOs"
REM or if your entry point is in lib.rs, that it's configured to produce an executable.
cargo +nightly build --target %TARGET_JSON% REM Add --release for release build

if %ERRORLEVEL% neq 0 (
    echo ❌ Rust kernel compilation failed
    exit /b 1
)
echo ✅ Rust kernel compiled successfully

REM Define the path to the compiled kernel ELF
REM This path assumes you are building in debug mode.
REM If you use `cargo build --target %TARGET_JSON% --release`, change "debug" to "release".
set "KERNEL_ELF_PATH=target\%TARGET_SPEC%\debug\%KERNEL_TARGET_NAME%"
if not exist "%KERNEL_ELF_PATH%" (
    echo ❌ ERROR: Kernel ELF not found at %KERNEL_ELF_PATH%
    echo   Check your Cargo.toml [[bin]] name and build mode (debug/release).
    exit /b 1
)

REM Optional: Check kernel file size (a very small or zero size indicates a problem)
for %%F in ("%KERNEL_ELF_PATH%") do set KERNEL_SIZE_BYTES=%%~zF
if %KERNEL_SIZE_BYTES% LSS 10000 ( REM Arbitrary small size check (e.g., 10KB)
    echo ⚠️ WARNING: Kernel ELF file size is very small (%KERNEL_SIZE_BYTES% bytes). This might indicate a build issue.
)
echo   Kernel ELF found: %KERNEL_ELF_PATH% (Size: %KERNEL_SIZE_BYTES% bytes)


echo 📀 Creating ISO file with GRUB...
set "ISO_DIR=iso_root"
REM Create ISO directory structure
if not exist "%ISO_DIR%\boot\grub" mkdir "%ISO_DIR%\boot\grub"

REM Copy Rust kernel ELF to ISO directory, using a consistent name
copy /Y "%KERNEL_ELF_PATH%" "%ISO_DIR%\boot\%KERNEL_TARGET_NAME%.elf" >nul
if %ERRORLEVEL% neq 0 (
    echo ❌ Failed to copy kernel ELF.
    exit /b 1
)

REM Create GRUB configuration
(
    echo set timeout=3
    echo set default=0
    echo.
    echo # Debug - show which files GRUB can see
    echo ls /boot
    echo.
    echo menuentry "FluxGrid OS (Rust Kernel)" {
    echo     insmod all_video
    echo     echo "Loading FluxGridOS kernel..."
    echo     multiboot2 /boot/%KERNEL_TARGET_NAME%.elf
    echo     echo "Kernel loaded, transferring control..."
    echo     boot
    echo }
) > "%ISO_DIR%\boot\grub\grub.cfg"
if %ERRORLEVEL% neq 0 (
    echo ❌ Failed to create grub.cfg.
    exit /b 1
)

REM Generate ISO using GRUB
REM This requires 'grub-mkrescue.exe' available in your PATH (e.g. from a GRUB for Windows package)
grub-mkrescue -o %ISO_NAME% %ISO_DIR%
if %ERRORLEVEL% neq 0 (
    echo ❌ ISO creation failed. 'grub-mkrescue' might be missing or encountered an error.
    exit /b 1
)

echo ✅ ISO created successfully: %ISO_NAME%
echo.
echo 🚀 To run the ISO with QEMU (boots your Rust kernel via GRUB):
echo qemu-system-x86_64 -m 1G -serial stdio -cdrom %ISO_NAME% -no-reboot -no-shutdown
echo.
echo 🚀 To test your custom assembly bootloader directly (if it's a floppy image):
echo qemu-system-x86_64 -fda %ASM_BOOTLOADER_BIN% -no-reboot -no-shutdown
echo.

echo 🐳 CUSTOM BOOTLOADER - Normal Mode:
echo docker run --rm -v "%cd%:/data" ^
echo     --privileged ^
echo     -p 5900:5900 ^
echo     tianon/qemu ^
echo     qemu-system-x86_64 -m 1G ^
echo         -fda /data/floppy.img ^
echo         -display vnc=0.0.0.0:0 ^
echo         -serial stdio ^
echo         -no-reboot ^
echo         -no-shutdown
echo.
echo 🐳 CUSTOM BOOTLOADER - Debug Mode (GDB):
echo docker run --rm -v "%cd%:/data" ^
echo     --privileged ^
echo     -p 5900:5900 ^
echo     -p 1234:1234 ^
echo     tianon/qemu ^
echo     qemu-system-x86_64 -m 1G ^
echo         -cdrom /data/%PROJECT_NAME%.iso ^
echo         -monitor stdio ^
echo         -no-reboot ^
echo         -no-shutdown ^
echo         -S ^
echo         -gdb tcp::1234
echo.
echo 🔍 For debug mode:
echo 1. Start the Docker container with the above command.
echo 2. Open a new terminal and connect GDB to the QEMU GDB server:
echo    gdb
echo    file target\%TARGET_SPEC%\debug\%KERNEL_TARGET_NAME%
echo 3. Use GDB commands to debug your kernel:
echo    - 'layout asm' to view assembly code
echo    - 'layout src' to view source code (if available)
echo    - 'continue' to start execution
echo    - 'break ^<function^>' to set breakpoints (use ^ to escape < and > in echo)
echo    - 'info registers' to inspect CPU registers
echo 4. To interact with the QEMU monitor, press 'Ctrl+Alt+2' in the VNC viewer.
echo    - Use 'info registers' or 'x ^<address^>' to inspect memory.
echo    - Press 'Ctrl+Alt+1' to return to the VGA display.
echo.

echo 🐳 Docker commands for running the ISO:
echo docker run --rm -v "%cd%:/data" ^
    --privileged -p 5900:5900 tianon/qemu ^
    qemu-system-x86_64 -m 1G ^
    -cdrom /data/%PROJECT_NAME%.iso ^
    -display vnc=0.0.0.0:0 ^
    -vga std ^
    -serial stdio ^
    -no-reboot -no-shutdown
echo.

endlocal