use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use hashbrown::HashMap;

/// Configuration for window layouts in the GUI.
pub struct WindowLayoutConfig {
    /// Default window size (width, height)
    pub default_size: (u32, u32),
    /// Default window position (x, y)
    pub default_position: (i32, i32),
    /// Margins between windows
    pub margin: u32,
    /// Maps window IDs to their specific configurations
    pub window_configs: HashMap<String, WindowConfig>,
    /// Determines if windows should snap to edges
    pub snap_to_edges: bool,
    /// Grid size for snapping (width, height)
    pub grid_size: (u32, u32),
}

/// Configuration for individual windows
#[derive(Clone)]
pub struct WindowConfig {
    /// Window size (width, height)
    pub size: (u32, u32),
    /// Window position (x, y)
    pub position: (i32, i32),
    /// Whether the window can be resized
    pub resizable: bool,
    /// Whether the window can be moved
    pub movable: bool,
    /// Minimum size constraints
    pub min_size: Option<(u32, u32)>,
    /// Maximum size constraints
    pub max_size: Option<(u32, u32)>,
}

impl Default for WindowLayoutConfig {
    fn default() -> Self {
        WindowLayoutConfig {
            default_size: (800, 600),
            default_position: (100, 100),
            margin: 5,
            window_configs: HashMap::new(),
            snap_to_edges: true,
            grid_size: (10, 10),
        }
    }
}

impl WindowLayoutConfig {
    /// Create a new window layout configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add or update a specific window configuration
    pub fn add_window_config(&mut self, id: &str, config: WindowConfig) {
        self.window_configs.insert(id.to_string(), config);
    }
    
    /// Get configuration for a specific window, falling back to defaults if not found
    pub fn get_window_config(&self, id: &str) -> WindowConfig {
        self.window_configs.get(id).cloned().unwrap_or_else(|| {
            WindowConfig {
                size: self.default_size,
                position: self.default_position,
                resizable: true,
                movable: true,
                min_size: None,
                max_size: None,
            }
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        // Deserialize bytes into WindowLayoutConfig
        // This is a placeholder for actual deserialization logic
        // In a real application, you would use a library like serde
        Ok(Self::default())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, &'static str> {
        // Serialize WindowLayoutConfig into bytes
        // This is a placeholder for actual serialization logic
        // In a real application, you would use a library like serde
        Ok(vec![])
    }
}

impl WindowConfig {
    /// Create a new window configuration with default values
    pub fn new(size: (u32, u32), position: (i32, i32)) -> Self {
        WindowConfig {
            size,
            position,
            resizable: true,
            movable: true,
            min_size: None,
            max_size: None,
        }
    }
}