//! GUI system for OS Gaming
//!
//! This module provides a complete graphical user interface system optimized for gaming.

mod renderer;
mod window_manager;
mod theme;
mod input;
mod font;

// These will be implemented later
mod widgets;

// Re-export main types
pub use renderer::{Renderer, Color, Rect, BlendMode, RendererError};
pub use window_manager::{WindowManager, Window};
pub use theme::Theme;

/// Initialize the renderer at specified resolution
pub fn init_renderer(width: u32, height: u32) -> Result<Renderer, &'static str> {
    match renderer::Renderer::new(width, height) {
        Ok(renderer) => Ok(renderer),
        Err(_) => Err("Failed to initialize renderer"),
    }
}

/// Initialize the window manager
pub fn init_window_manager(renderer: Renderer) -> Result<WindowManager, &'static str> {
    match window_manager::WindowManager::new(renderer) {
        Ok(wm) => Ok(wm),
        Err(_) => Err("Failed to initialize window manager"),
    }
}

/// Initialize font system
pub fn init_fonts() -> Result<(), &'static str> {
    // This is a placeholder for font initialization
    // In a real application, you would load fonts here
    Ok(())
}