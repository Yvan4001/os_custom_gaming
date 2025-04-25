//! Window management system
//!
//! This module handles window creation, movement, focus, and rendering.
#![no_std]

extern crate alloc;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use alloc::{string::String, vec::Vec};
use bincode::{Decode, Encode};
use spin::Mutex;


use super::renderer::{Color, Rect, Renderer, RendererError};
use super::theme::Theme;

/// Unique identifier for windows
pub type WindowId = u32;

/// Window properties
pub struct Window {
    id: WindowId,
    title: String,
    rect: Rect, // Store the rectangle directly
    rect_mutex: *const Mutex<Rect>, // Pointer to a mutex for safe access
    visible: AtomicBool,
    focused: AtomicBool,
    render_callback: Option<fn(&mut Renderer, &Window)>,
    event_callback: Option<fn(&Window, &WindowEvent) -> bool>,
    background_color: Color,
    border_color: Color,
    user_data: Option<*mut u8>, // Raw pointer to user-defined data
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    MouseMove { x: i32, y: i32 },
    MousePress { button: u8, x: i32, y: i32 },
    MouseRelease { button: u8, x: i32, y: i32 },
    KeyPress { key: u16 },
    KeyRelease { key: u16 },
    WindowResize { width: u32, height: u32 },
    WindowClose,
    WindowFocus,
    WindowBlur,
}


/// Window events
#[derive(Debug, Clone, Copy)]
pub enum WindowEvent {
    MouseMove {
        x: i32,
        y: i32,
    },
    MouseDown {
        x: i32,
        y: i32,
        button: u8,
    },
    MouseUp {
        x: i32,
        y: i32,
        button: u8,
    },
    KeyDown {
        key: u16,
        scancode: u16,
        modifiers: u8,
    },
    KeyUp {
        key: u16,
        scancode: u16,
        modifiers: u8,
    },
    Focus,
    Blur,
    Close,
    Resize {
        width: u32,
        height: u32,
    },
    Move {
        x: i32,
        y: i32,
    },
}

/// Window manager that handles window creation, events, and rendering
pub struct WindowManager {
    renderer: Renderer,
    windows: Mutex<Vec<Window>>,
    next_window_id: AtomicU32,
    focused_window: AtomicU32,
    dragging_window: AtomicU32,
    drag_offset_x: i32,
    drag_offset_y: i32,
    theme: Theme,
    exit_requested: AtomicBool,
}

impl Clone for Window {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            title: self.title.clone(),
            rect: self.rect,
            rect_mutex: self.rect_mutex,
            visible: AtomicBool::new(self.visible.load(Ordering::Relaxed)),
            focused: AtomicBool::new(self.focused.load(Ordering::Relaxed)),
            render_callback: self.render_callback,
            event_callback: self.event_callback,
            background_color: self.background_color,
            border_color: self.border_color,
            user_data: self.user_data,
        }
    }
}

impl Window {
    /// Create a new window with the given properties
    pub fn new(id: WindowId, title: &str, rect: Rect) -> Self {
        // Create a static Mutex for the rect
        static RECT_MUTEX: Mutex<Rect> = Mutex::new(Rect::new(0, 0, 0, 0));
        let rect_mutex = &RECT_MUTEX as *const _;
        Self {
            id,
            title: String::from(title),
            rect, // Store the original rect
            rect_mutex, // Store the pointer to the mutex
            visible: AtomicBool::new(false),
            focused: AtomicBool::new(false),
            render_callback: None,
            event_callback: None,
            background_color: Color::UI_BACKGROUND,
            border_color: Color::UI_ACCENT,
            user_data: None,
        }
    }

    /// Get the window ID
    pub fn id(&self) -> WindowId {
        self.id
    }
    /// Get the window rectangle
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Set the window rectangle
    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    /// Check if the window is visible
    pub fn is_visible(&self) -> bool {
        self.visible.load(Ordering::Relaxed)
    }

    /// Set window visibility
    pub fn set_visible(&self, visible: bool) {
        self.visible.store(visible, Ordering::Relaxed);
    }

    /// Check if the window is focused
    pub fn is_focused(&self) -> bool {
        self.focused.load(Ordering::Relaxed)
    }

    /// Set window focus state
    pub fn set_focused(&self, focused: bool) {
        self.focused.store(focused, Ordering::Relaxed);
    }

    /// Set window render callback
    pub fn set_render_callback(&mut self, callback: fn(&mut Renderer, &Window)) {
        self.render_callback = Some(callback);
    }

    /// Set window event callback
    pub fn set_event_callback(&mut self, callback: fn(&Window, &WindowEvent) -> bool) {
        self.event_callback = Some(callback);
    }
    /// Set window background color
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }
    /// Set window border color
    pub fn set_border_color(&mut self, color: Color) {
        self.border_color = color;
    }
    /// Set user data pointer
    pub fn set_user_data(&mut self, data: *mut u8) {
        self.user_data = Some(data);
    }
}

impl WindowManager {
    /// Create a new window manager
    pub fn new(renderer: Renderer) -> Result<Self, &'static str> {
        Ok(Self {
            renderer,
            windows: Mutex::new(Vec::new()),
            next_window_id: AtomicU32::new(1),
            focused_window: AtomicU32::new(0),
            dragging_window: AtomicU32::new(0),
            drag_offset_x: 0,
            drag_offset_y: 0,
            theme: Theme::default(),
            exit_requested: AtomicBool::new(false),
        })
    }

    /// Create a new window
    pub fn create_window(
        &self,
        title: &str,
        width: u32,
        height: u32,
        centered: bool,
    ) -> Result<WindowId, &'static str> {
        let id = self.next_window_id.fetch_add(1, Ordering::Relaxed);
        let (screen_width, screen_height) = self.renderer.dimensions();

        let x = if centered {
            ((screen_width - width) / 2) as i32
        } else {
            20
        };

        let y = if centered {
            ((screen_height - height) / 2) as i32
        } else {
            20
        };

        let rect = Rect::new(x, y, width, height);
        let window = Window::new(id, title, rect);

        // Add window to list
        let mut windows = self.windows.lock();
        windows.push(window);

        Ok(id)
    }

    /// Show a window
    pub fn show_window(&self, id: WindowId) {
        let mut windows = self.windows.lock();

        if let Some(window) = windows.iter_mut().find(|w| w.id() == id) {
            window.set_visible(true);
            self.focus_window(id);
        }
    }

    /// Hide a window
    pub fn hide_window(&self, id: WindowId) {
        let mut windows = self.windows.lock();

        if let Some(window) = windows.iter_mut().find(|w| w.id() == id) {
            window.set_visible(false);

            // If this was the focused window, focus another window
            if self.focused_window.load(Ordering::Relaxed) == id {
                self.focused_window.store(0, Ordering::Relaxed);

                // Find another visible window to focus
                for w in windows.iter() {
                    if w.is_visible() && w.id() != id {
                        self.focused_window.store(w.id(), Ordering::Relaxed);
                        w.set_focused(true);
                        break;
                    }
                }
            }
        }
    }

    /// Close a window
    pub fn close_window(&self, id: WindowId) {
        // Hide the window first
        self.hide_window(id);

        // Then remove it from the list
        let mut windows = self.windows.lock();
        if let Some(index) = windows.iter().position(|w| w.id() == id) {
            windows.remove(index);
        }
    }

    /// Close all windows
    pub fn close_all_windows(&self) {
        let mut windows = self.windows.lock();
        windows.clear();
        self.focused_window.store(0, Ordering::Relaxed);
    }

    /// Focus a specific window
    pub fn focus_window(&self, id: WindowId) {
        let mut windows = self.windows.lock();
        let old_focused = self.focused_window.load(Ordering::Relaxed);

        // Unfocus previously focused window
        if old_focused != 0 {
            if let Some(window) = windows.iter_mut().find(|w| w.id() == old_focused) {
                window.set_focused(false);

                // Send blur event
                if let Some(callback) = window.event_callback {
                    let _ = callback(window, &WindowEvent::Blur);
                }
            }
        }

        // Focus new window
        if let Some(window) = windows.iter_mut().find(|w| w.id() == id) {
            window.set_focused(true);
            self.focused_window.store(id, Ordering::Relaxed);

            // Send focus event
            if let Some(callback) = window.event_callback {
                let _ = callback(window, &WindowEvent::Focus);
            }

            // Move window to front (top of render order)
            if let Some(index) = windows.iter().position(|w| w.id() == id) {
                let window = windows.remove(index);
                windows.push(window);
            }
        }
    }

    pub fn has_window(&self, id: WindowId) -> bool {
        let windows = self.windows.lock();
        windows.iter().any(|w| w.id() == id)
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Update window manager state
    pub fn update(&mut self) {
        // Process system events would go here
        // But we're assuming that's done by the caller
    }

    /// Handle mouse movement
    pub fn handle_mouse_event(&mut self, x: i32, y: i32, buttons: u8, scroll_delta: i8) {
        // Handle window dragging
        let dragging_id = self.dragging_window.load(Ordering::Relaxed);
        if dragging_id != 0 && buttons & 1 != 0 {
            // Continue dragging window
            let mut windows = self.windows.lock();
            if let Some(window) = windows.iter_mut().find(|w| w.id() == dragging_id) {
                let mut rect = window.rect();
                rect.x = x - self.drag_offset_x;
                rect.y = y - self.drag_offset_y;
                window.set_rect(rect);

                // Send move event
                if let Some(callback) = window.event_callback {
                    let _ = callback(
                        window,
                        &WindowEvent::Move {
                            x: rect.x,
                            y: rect.y,
                        },
                    );
                }
            }
            return;
        } else if dragging_id != 0 {
            // Stop dragging
            self.dragging_window.store(0, Ordering::Relaxed);
        }

        // Check for hits
        let mut windows = self.windows.lock();

        // Process from top (front) to bottom
        for i in (0..windows.len()).rev() {
            let window = &mut windows[i];
            if !window.is_visible() {
                continue;
            }

            let rect = window.rect();
            if rect.contains(x, y) {
                // Window was hit

                // Focus window if clicked
                if buttons & 1 != 0 && !window.is_focused() {
                    self.focus_window(window.id());
                }

                // Check if clicking on title bar (for dragging)
                let title_bar_height = 25; // Example height
                let in_title_bar = y >= rect.y && y < rect.y + title_bar_height;

                if buttons & 1 != 0 && in_title_bar {
                    // Start dragging
                    self.dragging_window.store(window.id(), Ordering::Relaxed);
                    self.drag_offset_x = x - rect.x;
                    self.drag_offset_y = y - rect.y;
                } else {
                    // Send mouse event to window
                    let window_x = x - rect.x;
                    let window_y = y - rect.y;

                    if let Some(callback) = window.event_callback {
                        if buttons & 1 != 0 {
                            let _ = callback(
                                window,
                                &WindowEvent::MouseDown {
                                    x: window_x,
                                    y: window_y,
                                    button: 1,
                                },
                            );
                        } else {
                            let _ = callback(
                                window,
                                &WindowEvent::MouseMove {
                                    x: window_x,
                                    y: window_y,
                                },
                            );
                        }
                    }
                }

                break;
            }
        }
    }

    /// Handle key events
    pub fn handle_key_event(&mut self, key: u16, pressed: bool, modifiers: u8) {
        let focused_id = self.focused_window.load(Ordering::Relaxed);
        if focused_id == 0 {
            return;
        }

        let windows = self.windows.lock();
        if let Some(window) = windows.iter().find(|w| w.id() == focused_id) {
            if let Some(callback) = window.event_callback {
                let event = if pressed {
                    WindowEvent::KeyDown {
                        key,
                        scancode: key,
                        modifiers,
                    }
                } else {
                    WindowEvent::KeyUp {
                        key,
                        scancode: key,
                        modifiers,
                    }
                };

                let _ = callback(window, &event);
            }
        }
    }

    /// Handle touch events
    pub fn handle_touch_event(&mut self, id: u8, x: i32, y: i32, pressure: u8) {
        // Convert touch to mouse event
        let buttons = if pressure > 0 { 1 } else { 0 };
        self.handle_mouse_event(x, y, buttons, 0);
    }
    /// Render all windows
    pub fn render(&mut self) -> Result<(), RendererError> {
        // Collect window references into a local Vec to avoid borrowing conflict
        let windows_to_render = {
            let windows = self.windows.lock();
            windows
                .iter()
                .filter(|w| w.is_visible())
                .cloned()
                .collect::<Vec<_>>()
        };
        
        // Now render each window
        for window in windows_to_render {
            self.render_window(&window)?;
        }
        Ok(())
    }

    /// Render a single window
    fn render_window(&mut self, window: &Window) -> Result<(), RendererError> {
        let rect = window.rect();

        // Draw window background
        self.renderer.fill_rect(rect, window.background_color);

        // Draw window border
        let border_color = if window.is_focused() {
            self.theme.window_border_active
        } else {
            self.theme.window_border_inactive
        };

        self.renderer.draw_rect(rect, border_color);

        // Draw title bar
        let title_bar_height = 25;
        let title_bar_rect = Rect::new(rect.x, rect.y, rect.width, title_bar_height);

        let title_bar_color = if window.is_focused() {
            self.theme.title_bar_active
        } else {
            self.theme.title_bar_inactive
        };

        self.renderer.fill_rect(title_bar_rect, title_bar_color);

        // Draw window title
        // In a real implementation, this would use a text rendering function

        // Draw window content
        if let Some(render_fn) = window.render_callback {
            // Set clipping to window content area
            let content_rect = Rect::new(
                rect.x,
                rect.y + title_bar_height as i32,
                rect.width,
                rect.height - title_bar_height,
            );

            self.renderer.set_clip_rect(Some(content_rect));

            // Call the window's render function
            render_fn(&mut self.renderer, window);

            // Clear clipping
            self.renderer.set_clip_rect(None);
        }

        Ok(())
    }
    /// Get window by ID
    pub fn get_window(&self, id: WindowId) -> Option<Window> {
        let windows = self.windows.lock();
        windows.iter().find(|w| w.id() == id).cloned()
    }

    /// Save a simplified representation of window layout for serialization
    pub fn save_layout(&self) -> Vec<(WindowId, String, Rect)> {
        let windows = self.windows.lock();
        windows.iter()
            .map(|window| (window.id, window.title.clone(), window.rect))
            .collect()
    }

    pub fn handle_key_press(&mut self, key: u16) {
        // Handle key press events
        if key == 27 { // Escape key
            self.exit_requested.store(true, Ordering::Relaxed);
        }
    }
    pub fn handle_key_release(&mut self, key: u16) {
        // Handle key release events
        if key == 27 { // Escape key
            self.exit_requested.store(false, Ordering::Relaxed);
        }
    }

    pub fn handle_mouse_move(&mut self, x: i32, y: i32) {
        // Handle mouse move events
        self.handle_mouse_event(x, y, 0, 0);
    }
    pub fn handle_mouse_press(&mut self, button: u8, x: i32, y: i32) {
        // Handle mouse press events
        self.handle_mouse_event(x, y, button as u8, 0);
    }
    pub fn handle_mouse_release(&mut self, button: u8, x: i32, y: i32) {
        // Handle mouse release events
        self.handle_mouse_event(x, y, 0, 0);
    }

    pub fn handle_event(&mut self, event: Event) {
        // Handle window events
        self.handle_event(event);
    }

    pub fn handle_mouse_scroll(&mut self, delta: i32, x: i32, y: i32) {
        // Handle mouse scroll events
        self.handle_mouse_event(x, y, 0, delta as i8);
    }
    pub fn exit_requested(&self) -> bool {
        self.exit_requested.load(Ordering::Relaxed)
    }
    pub fn handle_window_resize(&mut self, width: u32, height: u32) {
        // Handle window resize events
        self.handle_window_resize(width, height);
    }

    pub fn handle_window_focus(&mut self) {
        // Handle window focus events
        self.handle_window_focus();
    }
    pub fn handle_window_blur(&mut self) {
        // Handle window blur events
        self.handle_window_blur();
    }

    pub fn shutdown(&mut self) {
        // Handle shutdown events
        self.close_all_windows();
    }
}
