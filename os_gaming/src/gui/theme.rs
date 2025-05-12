//! UI theming system
//!
//! This module defines the visual appearance of UI elements.

use super::renderer::Color;

/// UI theme definition
#[derive(Clone)]
pub struct Theme {
    // Window colors
    pub window_background: Color,
    pub window_border_active: Color,
    pub window_border_inactive: Color,
    pub title_bar_active: Color,
    pub title_bar_inactive: Color,
    pub title_text_active: Color,
    pub title_text_inactive: Color,
    
    // Desktop colors
    pub desktop_background: Color,
    
    // Button colors
    pub button_normal: Color,
    pub button_hover: Color,
    pub button_active: Color,
    pub button_text: Color,
    pub button_border: Color,
    
    // Text colors
    pub text_normal: Color,
    pub text_disabled: Color,
    pub text_highlight: Color,
    
    // Control colors
    pub control_background: Color,
    pub control_foreground: Color,
    pub control_border: Color,
    
    // Other UI elements
    pub selection_background: Color,
    pub selection_text: Color,
    pub scrollbar_background: Color,
    pub scrollbar_handle: Color,
    
    // Fonts
    pub font_family: &'static str,
    pub font_size: u16,
}

impl Default for Theme {
    /// Create the default dark theme
    fn default() -> Self {
        Self {
            // Window colors
            window_background: Color::rgb(30, 30, 30),
            window_border_active: Color::rgb(0, 120, 215),
            window_border_inactive: Color::rgb(80, 80, 80),
            title_bar_active: Color::rgb(40, 40, 40),
            title_bar_inactive: Color::rgb(30, 30, 30),
            title_text_active: Color::rgb(255, 255, 255),
            title_text_inactive: Color::rgb(200, 200, 200),
            
            // Desktop colors
            desktop_background: Color::rgb(20, 20, 20),
            
            // Button colors
            button_normal: Color::rgb(60, 60, 60),
            button_hover: Color::rgb(70, 70, 70),
            button_active: Color::rgb(0, 120, 215),
            button_text: Color::rgb(255, 255, 255),
            button_border: Color::rgb(100, 100, 100),
            
            // Text colors
            text_normal: Color::rgb(240, 240, 240),
            text_disabled: Color::rgb(130, 130, 130),
            text_highlight: Color::rgb(0, 120, 215),
            
            // Control colors
            control_background: Color::rgb(40, 40, 40),
            control_foreground: Color::rgb(240, 240, 240),
            control_border: Color::rgb(80, 80, 80),
            
            // Other UI elements
            selection_background: Color::rgb(0, 120, 215),
            selection_text: Color::rgb(255, 255, 255),
            scrollbar_background: Color::rgb(30, 30, 30),
            scrollbar_handle: Color::rgb(80, 80, 80),
            
            // Fonts
            font_family: "Roboto",
            font_size: 14,
        }
    }
}

impl Theme {
    /// Create a light theme
    pub fn light() -> Self {
        Self {
            // Window colors
            window_background: Color::rgb(240, 240, 240),
            window_border_active: Color::rgb(0, 120, 215),
            window_border_inactive: Color::rgb(180, 180, 180),
            title_bar_active: Color::rgb(220, 220, 220),
            title_bar_inactive: Color::rgb(230, 230, 230),
            title_text_active: Color::rgb(0, 0, 0),
            title_text_inactive: Color::rgb(100, 100, 100),
            
            // Desktop colors
            desktop_background: Color::rgb(220, 220, 220),
            
            // Button colors
            button_normal: Color::rgb(230, 230, 230),
            button_hover: Color::rgb(210, 210, 210),
            button_active: Color::rgb(0, 120, 215),
            button_text: Color::rgb(0, 0, 0),
            button_border: Color::rgb(180, 180, 180),
            
            // Text colors
            text_normal: Color::rgb(0, 0, 0),
            text_disabled: Color::rgb(150, 150, 150),
            text_highlight: Color::rgb(0, 120, 215),
            
            // Control colors
            control_background: Color::rgb(255, 255, 255),
            control_foreground: Color::rgb(0, 0, 0),
            control_border: Color::rgb(180, 180, 180),
            
            // Other UI elements
            selection_background: Color::rgb(0, 120, 215),
            selection_text: Color::rgb(255, 255, 255),
            scrollbar_background: Color::rgb(240, 240, 240),
            scrollbar_handle: Color::rgb(180, 180, 180),
            
            // Fonts
            font_family: "Roboto",
            font_size: 14,
        }
    }
    
    /// Create a gaming theme with accent color
    pub fn gaming(accent_color: Color) -> Self {
        let mut theme = Self::default();
        theme.window_border_active = accent_color;
        theme.button_active = accent_color;
        theme.text_highlight = accent_color;
        theme.selection_background = accent_color;
        theme
    }

    pub fn load(theme_name: &str) -> Self {
        match theme_name {
            "light" => Self::light(),
            "gaming" => Self::gaming(Color::rgb(0, 120, 215)),
            _ => Self::default(),
        }
    }
}