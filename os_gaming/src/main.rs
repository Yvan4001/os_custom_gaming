#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use log::{info, error};
use os_gaming::{gui, kernel, logger};
use core::option::Option::Some;
use core::clone::Clone;
use core::default::Default;
use core::marker::Send;
use core::marker::Sync;
use core::result::Result::Ok;
use core::result::Result::Err;
use core::result::Result;
use core::mem::drop;
use core::ops::FnOnce;
use core::option::Option;
use core::option::Option::None;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialisation du logger
    logger::init().expect("Logger initialization failed");
    info!("Starting OS Gaming...");

    // Initialisation du kernel
    info!("Init Kernel...");
    match init_kernel() {
        Ok(_) => info!("Kernel successfully initialized"),
        Err(e) => {
            error!("Error when Kernel initialize: {:?}", e);
            loop {}
        }
    }

    // Initialisation du GUI
    info!("Init GUI...");
    match init_gui() {
        Ok(_) => info!("GUI successfully initialized"),
        Err(e) => {
            error!("Error when GUI initialize: {:?}", e);
            loop {}
        }
    }

    loop {}
}

fn init_kernel() -> Result<(), &'static str> {
    kernel::init();
    Ok(())
}

fn init_gui() -> Result<(), &'static str> {
    gui::init_gui(Default::default());
    Ok(())
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {}", info);
    loop {}
}