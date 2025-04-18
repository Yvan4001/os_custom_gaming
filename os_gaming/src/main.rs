
#[cfg(feature = "std")]

use std::process;

use os_gaming::{self, Config};
use os_gaming::system::init;
use core::arch::asm;

fn main() {
    #[cfg(feature = "std")]
    env_logger::init();
    
    match os_gaming::init() {
        Ok(config) => {
            #[cfg(feature = "std")]
            {
                println!("OS Gaming initialized successfully!");
                // Launch GUI application
                os_gaming::gui::init_gui(config);
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize: {}", e);
            #[cfg(feature = "std")]
            process::exit(1);
        }
    }
}