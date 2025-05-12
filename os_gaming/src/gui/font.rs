#![no_std]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::{format, vec};
use hashbrown::HashMap;
use core::convert::AsRef;

#[derive(Default)]
pub struct FontDefinitions {
    pub font_data: HashMap<String, Arc<FontData>>,
    pub families: HashMap<FontFamily, Vec<String>>,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum FontFamily {
    Proportional,
    Monospace,
}

#[derive(Clone)]
pub struct FontData {
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum TextStyle {
    Heading,
    Body,
    Monospace,
    Button,
    Small,
}

pub struct FontId {
    pub size: f32,
    pub family: FontFamily,
}

impl FontId {
    pub fn new(size: f32, family: FontFamily) -> Self {
        Self { size, family }
    }
}

pub struct FontManager {
    fonts: HashMap<String, usize>,
    font_definitions: FontDefinitions,
    sizes: HashMap<String, f32>,
}

impl FontManager {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            font_definitions: FontDefinitions::default(),
            sizes: HashMap::new(),
        }
    }

    pub fn load_font(&mut self, name: &str, font_data: &[u8]) -> Result<(), String> {
        let font_index = self.font_definitions.font_data.len();

        self.font_definitions.font_data.insert(
            name.to_string(),
            Arc::new(FontData {
                data: font_data.to_vec(),
            }),
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
            .or_insert_with(Vec::new)
            .push(font_name.to_string());

        Ok(())
    }

    pub fn setup_default_fonts(&mut self) -> Result<(), String> {
        // Définition des familles de polices par défaut
        self.font_definitions.families.insert(
            FontFamily::Proportional,
            vec!["default-prop".to_string()]
        );
        self.font_definitions.families.insert(
            FontFamily::Monospace,
            vec!["default-mono".to_string()]
        );

        Ok(())
    }

    pub fn set_size_for_element(&mut self, element: &str, size: f32) {
        self.sizes.insert(element.to_string(), size);
    }

    pub fn load_font_from_memory(&mut self, name: &str, data: &[u8]) -> Result<(), String> {
        let font_index = self.font_definitions.font_data.len();
        self.font_definitions.font_data.insert(
            name.to_string(),
            Arc::new(FontData {
                data: data.to_vec(),
            }),
        );
        self.fonts.insert(name.to_string(), font_index);

        Ok(())
    }
}

impl Default for FontManager {
    fn default() -> Self {
        Self::new()
    }
}