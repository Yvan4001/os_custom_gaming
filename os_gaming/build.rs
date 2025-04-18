fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    
    // Set environment variables for the build
    println!("cargo:rustc-env=RUST_BACKTRACE=1");
    
    // Tell cargo to link with the required libraries
    println!("cargo:rustc-link-lib=static=compiler-rt");
} 