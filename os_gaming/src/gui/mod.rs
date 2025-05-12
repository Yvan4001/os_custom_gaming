//! GUI system for OS Gaming
//!
//! This module provides a complete graphical user interface system optimized for gaming.

pub mod renderer;
pub mod window_manager;
pub mod theme;
pub mod input;
pub mod font;
pub mod windows_layout;

use core::arch::asm;
use crate::Config;

// These will be implemented later
mod widgets;

// Re-export main types
pub use renderer::{Renderer, Color, Rect, BlendMode, RendererError};
pub use window_manager::{WindowManager, Window};
pub use font::FontManager;
pub use theme::Theme;
pub use windows_layout::WindowLayoutConfig;
use crate::kernel::cpu;
use crate::kernel::cpu::get_cpu_info;

pub struct Instant {
    timestamp: u64,
}

impl Instant {
    pub fn now() -> Self {
        Self {
            timestamp: read_tsc()
        }
    }
}

// Lire le Time Stamp Counter
fn read_tsc() -> u64 {
    unsafe {
        let low: u32;
        let high: u32;
        asm!(
        "rdtsc",
        out("eax") low,
        out("edx") high,
        options(nomem, nostack)
        );
        ((high as u64) << 32) | (low as u64)
    }
}


fn processor_frequency() -> Option<cpu::identification::CpuInfo> {
    return get_cpu_info()
}


/// Initialize the GUI components
pub fn init_gui(config: Config) -> Result<(), &'static str> {
    // Initialize the renderer at the specified resolution
    let w_config = WindowLayoutConfig::new();
    let renderer = init_renderer(w_config.grid_size.0, w_config.grid_size.1)?;

    // Initialize the window manager
    let window_manager = init_window_manager(renderer)?;

    window_manager.close_all_windows();
    window_manager.create_window("OS Gaming", w_config.grid_size.0, w_config.grid_size.1, true)?;

    // Initialize the font system
    init_fonts()?;

    // Run the main application loop
    run_app(config);

    Ok(())
}

/// Initialize the renderer at specified resolution
pub fn init_renderer(width: u32, height: u32) -> Result<Renderer, &'static str> {
    match Renderer::new(width, height) {
        Ok(renderer) => Ok(renderer),
        Err(_) => Err("Failed to initialize renderer"),
    }
}

/// Initialize the window manager
pub fn init_window_manager(renderer: Renderer) -> Result<WindowManager, &'static str> {
    match WindowManager::new(renderer) {
        Ok(wm) => Ok(wm),
        Err(_) => Err("Failed to initialize window manager"),
    }
}

/// Initialize font system
pub fn init_fonts() -> Result<(), &'static str> {
    // Get the font manager singleton
    let mut font_manager = FontManager::new();

    // Define font paths - adjust these to actual paths in your filesystem
    let system_font_paths = [
        "/usr/share/fonts/fluxGridOs/system_regular.ttf",
        "/usr/share/fonts/fluxGridOs/system_bold.ttf",
        "/usr/share/fonts/fluxGridOs/system_italic.ttf",
        "/usr/share/fonts/fluxGridOs/monospace.ttf",
        "/usr/share/fonts/fluxGridOs/gaming.ttf",
    ];

    // Try to load the system fonts
    let mut default_font_loaded = false;
    for font_path in &system_font_paths {
        match font_manager.load_font("AssetFont", font_path.as_ref()) {
            Ok(font_id) => {
                // Set the first successfully loaded font as default
                if !default_font_loaded {
                    font_manager.setup_default_fonts();
                    default_font_loaded = true;
                }
            },
            Err(err) => {
                log::warn!("Failed to load font {}: {}", font_path, err);
                // Continue trying other fonts
            }
        }
    }

    // If no system fonts could be loaded, try to load a fallback embedded font
    if !default_font_loaded {
        log::warn!("No system fonts loaded, falling back to embedded font");
        
        // Include a basic embedded font as a byte array
        static FALLBACK_FONT: &[u8] = include_bytes!("assets/arial-font/arial.ttf");
        
        match font_manager.load_font_from_memory("FallbackFont", FALLBACK_FONT) {
            Ok(font_id) => {
                font_manager.setup_default_fonts();
                default_font_loaded = true;
            },
            Err(err) => {
                log::error!("Failed to load fallback font: {}", err);
                return Err("No usable fonts could be loaded");
            }
        }
    }

    // Configure font sizes for different UI elements
    font_manager.set_size_for_element("window.title", 16.0);
    font_manager.set_size_for_element("button.label", 14.0);
    font_manager.set_size_for_element("menu.item", 14.0);
    font_manager.set_size_for_element("tooltip", 12.0);
    font_manager.set_size_for_element("system.notification", 14.0);
    font_manager.set_size_for_element("console", 12.0);

    log::info!("Font system initialized successfully");
    Ok(())
}

/// Run the main application loop
pub fn run_app(config: Config) {
    // Get required components
    let renderer = match Renderer::new(config.width, config.height) {
        Ok(r) => r,
        Err(_) => {
            log::error!("Failed to create renderer");
            return;
        }
    };
    
    let mut window_manager = match WindowManager::new(renderer) {
        Ok(wm) => wm,
        Err(_) => {
            log::error!("Failed to get window manager instance");
            return;
        }
    };

    let mut input_handler = input::InputManager::new();

    // Create main system window if it doesn't exist yet
    // Using a window ID (u32) instead of a string
    const MAIN_SYSTEM_ID: u32 = 1; // Choose an appropriate ID value
    if !window_manager.has_window(MAIN_SYSTEM_ID) {
        if let Err(e) = window_manager.create_window("MainSystem", config.width, config.height, true) {
            log::error!("Failed to create main system window: {}", e);
            return;
        }
    }

    // Load theme based on config
    let theme = Theme::load(&config.theme);
    window_manager.set_theme(theme);

    // Target frame rate
    let target_frame_time = 1.0 / config.refresh_rate as f64;
    let mut last_frame_time = Instant::now();
    
    // FPS counter
    let mut frames = 0;
    let mut fps_timer = Instant::now();
    let mut current_fps = 0;

    // Main loop running flag
    let mut running = true;
    
    log::info!("Entering main application loop");
    
    // Main application loop
    while running {
        // Process input events
        input_handler.update();
        while let Some(event) = input_handler.next_event() {
            match event {
                input::Event::Quit => {
                    log::info!("Quit event received, exiting application loop");
                    running = false;
                    break;
                },
                input::Event::KeyPress(key) => {
                    if key == input::Key::Escape {
                        if config.exit_on_escape {
                            log::info!("Escape key pressed, exiting application loop");
                            running = false;
                            break;
                        }
                    }
                    // Pass event to window manager
                    window_manager.handle_key_press(key as u16);
                },
                input::Event::KeyRelease(key) => {
                    window_manager.handle_key_release(key as u16);
                },
                input::Event::MouseMove(x, y) => {
                    window_manager.handle_mouse_move(x as i32, y as i32);
                },
                input::Event::MousePress(button) => {
                    let (x, y) = input_handler.get_mouse_position();
                    window_manager.handle_mouse_press(button as u8, x as i32, y as i32);
                },
                input::Event::MouseRelease(button) => {
                    let (x, y) = input_handler.get_mouse_position();
                    window_manager.handle_mouse_release(button as u8, x as i32, y as i32);
                },
                input::Event::MouseScroll(delta) => {
                    let (x, y) = input_handler.get_mouse_position();
                    window_manager.handle_mouse_scroll(delta as i32, x as i32, y as i32);
                },
                input::Event::WindowResize(width, height) => {
                    window_manager.handle_window_resize(width as u32, height as u32);
                },
                input::Event::WindowClose => {
                    log::info!("Window close event received");
                    running = false;
                    break;
                },
                input::Event::WindowFocus => {
                    log::info!("Window focus event received");
                    window_manager.handle_window_focus();
                },
                input::Event::WindowBlur => {
                    log::info!("Window blur event received");
                    window_manager.handle_window_blur();
                }
            }
        }

        // Update window states
        window_manager.update();
        
        // Render all windows
        window_manager.render();
        
    }
    
    // Perform cleanup
    log::info!("Exiting application loop, performing cleanup");
    window_manager.shutdown();
}