use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/boot/boot.s");
    println!("cargo:rerun-if-changed=src/bootloaderCustom.rs");
    
    // Enable bootloader-custom-config feature if specified
    if std::env::var("CARGO_FEATURE_BOOTLOADER_CUSTOM_CONFIG").is_ok() {
        println!("cargo:rustc-cfg=bootloader_custom_config");
    }
    
    // Add bootloader configuration parameters
    println!("cargo:rustc-env=BOOTLOADER_CONFIG_MEMORY_OFFSET=0xFFFF_8000_0000_0000");
    println!("cargo:rustc-env=BOOTLOADER_CONFIG_EXCLUDE_RANGE=0x400000-0x401000");
    println!("cargo:rustc-env=BOOTLOADER_CONFIG_EXCLUDE=0x400000:0x401000");
    
    // Only run this for no_std builds
    if env::var("TARGET").unwrap_or_default() == "x86_64-unknown-none" {
        // Try to find the once_cell crate in git dependencies
        if let Ok(out_dir) = env::var("OUT_DIR") {
            let workspace_dir = Path::new(&out_dir).ancestors().nth(3).unwrap();
            let git_dir = workspace_dir.join(".cargo/git/checkouts");
            
            // Look for once_cell directories
            if git_dir.exists() {
                if let Ok(entries) = fs::read_dir(&git_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            if let Ok(subdirs) = fs::read_dir(&path) {
                                for subdir in subdirs.flatten() {
                                    let subpath = subdir.path();
                                    if subpath.is_dir() && subpath.to_string_lossy().contains("once_cell") {
                                        // Found the once_cell directory, now fix the file
                                        let race_path = subpath.join("src/race.rs");
                                        if race_path.exists() {
                                            println!("cargo:warning=Found once_cell at {}", race_path.display());
                                            
                                            // Read and patch the file
                                            if let Ok(content) = fs::read_to_string(&race_path) {
                                                if !content.contains("use core::option::Option::Some") {
                                                    let new_content = content.replace(
                                                        "use core::fmt;",
                                                        "use core::fmt;\nuse core::option::Option::Some;"
                                                    );
                                                    
                                                    println!("cargo:warning=Patching once_cell race.rs to fix Some import");
                                                    let _ = fs::write(&race_path, new_content);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}