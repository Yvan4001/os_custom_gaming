use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use eframe::egui::{self, FontDefinitions, FontFamily, TextStyle};

pub struct FontManager {
    fonts: HashMap<String, usize>,
    font_definitions: FontDefinitions,
}

impl FontManager {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            font_definitions: FontDefinitions::default(),
        }
    }

    pub fn load_font(&mut self, name: &str, path: impl AsRef<Path>) -> Result<(), String> {
        let font_data = std::fs::read(path)
            .map_err(|e| format!("Failed to load font {}: {}", name, e))?;
        let font_index = self.font_definitions.font_data.len();

        self.font_definitions.font_data.insert(
            name.to_string(),
            Arc::new(egui::FontData::from_owned(font_data)),
        );
        self.fonts.insert(name.to_string(), font_index);
        
        Ok(())
    }

    pub fn set_font_family(&mut self, family: FontFamily, font_name: &str) -> Result<(), String> {
        if !self.fonts.contains_key(font_name) {
            return Err(format!("Font '{}' not loaded", font_name));
        }

        self.font_definitions
            .families
            .entry(family)
            .or_default()
            .push(font_name.to_string());

        Ok(())
    }

    pub fn configure_text_styles(&mut self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        
        // Configure text styles with different sizes
        style.text_styles = [
            (TextStyle::Heading, egui::FontId::new(24.0, FontFamily::Proportional)),
            (TextStyle::Body, egui::FontId::new(16.0, FontFamily::Proportional)),
            (TextStyle::Monospace, egui::FontId::new(14.0, FontFamily::Monospace)),
            (TextStyle::Button, egui::FontId::new(16.0, FontFamily::Proportional)),
            (TextStyle::Small, egui::FontId::new(12.0, FontFamily::Proportional)),
        ].into();

        ctx.set_style(style);
    }

    pub fn apply_to_context(&self, ctx: &egui::Context) {
        ctx.set_fonts(self.font_definitions.clone());
    }

    pub fn setup_default_fonts(&mut self) -> Result<(), String> {
        // This would be replaced with actual font loading in a real application
        // For example: self.load_font("roboto", "assets/fonts/Roboto-Regular.ttf")?;
        
        // Set default font families
        self.font_definitions.families.insert(
            FontFamily::Proportional,
            vec!["Roboto-Regular".to_string()]
        );
        self.font_definitions.families.insert(
            FontFamily::Monospace,
            vec!["Roboto-Mono".to_string()]
        );
        
        Ok(())
    }
}