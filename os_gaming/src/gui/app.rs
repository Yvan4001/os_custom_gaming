use eframe::{egui, Frame};
use crate::{Config, DisplayMode, PerformanceProfile};
use crate::system;
use std::collections::VecDeque;

const MAX_LINES: usize = 1000;
const DEFAULT_PROMPT: &str = "> ";


pub struct Console {
    lines: VecDeque<String>,
    current_input: String,
    prompt: String,
    cursor_pos: usize,
    history: VecDeque<String>,
    history_index: Option<usize>,
}

pub struct OsGamingApp {
    config: Config,
    system_info: SystemInfo,
    show_settings: bool,
}

struct SystemInfo {
    cpu_usage: f32,
    memory_used: u64,
    memory_total: u64,
    gpu_info: String,
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            gpu_info: "Unknown".to_string(),
        }
    }
}

impl OsGamingApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, config: Config) -> Self {
        Self {
            config,
            system_info: SystemInfo::default(),
            show_settings: false,
        }
    }
    
    fn update_system_info(&mut self) {
        if let Ok(info) = system::get_system_info() {
            self.system_info.cpu_usage = info.cpu_usage;
            self.system_info.memory_used = info.memory_used;
            self.system_info.memory_total = info.memory_total;
            self.system_info.gpu_info = info.gpu_info;
        }
    }
}

impl eframe::App for OsGamingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // Update system info every 1 second
        if ctx.input(|i| i.time) % 1.0 < 0.1 {
            self.update_system_info();
        }
        if ui.button("Test Sound").clicked() {
            // Play a simple beep
            let _ = kernel::drivers::sound_system::beep(440, 200);
        }
        
        // Volume slider
        let mut volume = kernel::drivers::sound_system::get_volume();
        ui.horizontal(|ui| {
            ui.label("Volume:");
            if ui.add(egui::Slider::new(&mut volume, 0..=100)).changed() {
                kernel::drivers::sound_system::set_volume(volume);
            }
            
            if ui.button(if kernel::drivers::sound_system::is_enabled() {
                "Mute"
            } else {
                "Unmute"
            }).clicked() {
                kernel::drivers::sound_system::set_enabled(
                    !kernel::drivers::sound_system::is_enabled()
                );
            }
        });

        if ui.button("Network Settings").clicked() {
            self.show_network_settings = true;
        }
        
        if self.show_network_settings {
            egui::Window::new("Network Settings")
                .open(&mut self.show_network_settings)
                .show(ctx, |ui| {
                    ui.heading("Network Interfaces");
                    
                    for iface in self.network_manager.get_interfaces() {
                        ui.collapsing(format!("{} - {}", iface.get_name(), mac_to_string(&iface.get_mac_address())), |ui| {
                            ui.label(format!("MAC: {}", mac_to_string(&iface.get_mac_address())));
                            
                            if let Some(ip) = iface.get_ip_address() {
                                ui.label(format!("IP: {}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]));
                            } else {
                                ui.label("IP: Not configured");
                            }
                            
                            ui.label(format!("MTU: {}", iface.get_mtu()));
                            
                            if ui.button("Configure").clicked() {
                                // Show configuration dialog for this interface
                                self.selected_iface = Some(iface.get_name().to_string());
                            }
                        });
                    }
                    
                    if let Some(ref iface_name) = self.selected_iface {
                        if let Some(iface) = self.network_manager.get_interface_mut(iface_name) {
                            ui.separator();
                            ui.heading(format!("Configure {}", iface_name));
                            
                            let mut ip_str = String::new();
                            if let Some(ip) = iface.get_ip_address() {
                                ip_str = format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
                            }
                            
                            ui.horizontal(|ui| {
                                ui.label("IP Address:");
                                if ui.text_edit_singleline(&mut ip_str).changed() {
                                    // Parse IP string and set if valid
                                    if let Some(ip) = parse_ip(&ip_str) {
                                        iface.set_ip_address(ip);
                                    }
                                }
                            });
                            
                            if ui.button("Apply").clicked() {
                                // Apply settings and reset interface
                                let _ = iface.reset();
                                self.selected_iface = None;
                            }
                        }
                    }
                });
        }
        
        // Helper functions
        fn mac_to_string(mac: &[u8; 6]) -> String {
            format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    mac[0], mac[1], mac[2], mac[3], mac[4], mac[5])
        }
        
        fn parse_ip(ip_str: &str) -> Option<[u8; 4]> {
            let parts: Vec<&str> = ip_str.split('.').collect();
            if parts.len() != 4 {
                return None;
            }
            
            let mut ip = [0u8; 4];
            for (i, part) in parts.iter().enumerate() {
                if let Ok(num) = part.parse::<u8>() {
                    ip[i] = num;
                } else {
                    return None;
                }
            }
            
            Some(ip)
        }
        
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Settings").clicked() {
                        self.show_settings = true;
                        ui.close_menu();
                    }
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("System", |ui| {
                    if ui.button("Performance Mode").clicked() {
                        self.config.performance_profile = PerformanceProfile::Performance;
                        system::apply_profile(&self.config);
                        ui.close_menu();
                    }
                    if ui.button("Balanced Mode").clicked() {
                        self.config.performance_profile = PerformanceProfile::Balanced;
                        system::apply_profile(&self.config);
                        ui.close_menu();
                    }
                    if ui.button("Power Saver").clicked() {
                        self.config.performance_profile = PerformanceProfile::PowerSaver;
                        system::apply_profile(&self.config);
                        ui.close_menu();
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("CPU: {:.1}%", self.system_info.cpu_usage));
                    ui.label(format!("MEM: {}/{} MB", 
                        self.system_info.memory_used / 1024 / 1024, 
                        self.system_info.memory_total / 1024 / 1024));
                });
            });
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("OS Gaming Dashboard");
            ui.add_space(20.0);
            
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading("System Status");
                    ui.label(format!("CPU Usage: {:.1}%", self.system_info.cpu_usage));
                    ui.label(format!("Memory: {} / {} MB", 
                        self.system_info.memory_used / 1024 / 1024, 
                        self.system_info.memory_total / 1024 / 1024));
                    ui.label(format!("GPU: {}", self.system_info.gpu_info));
                    ui.label(format!("Performance Profile: {:?}", self.config.performance_profile));
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    ui.vertical(|ui| {
                        if ui.button("Launch Game").clicked() {
                            // Launch game code here
                        }
                        if ui.button("Optimize System").clicked() {
                            system::optimize();
                        }
                    });
                });
            });
        });
        
        if self.show_settings {
            egui::Window::new("Settings")
                .open(&mut self.show_settings)
                .show(ctx, |ui| {
                    ui.heading("Display Settings");
                    ui.horizontal(|ui| {
                        ui.label("Display Mode:");
                        ui.radio_value(&mut self.config.display_mode, DisplayMode::Windowed, "Windowed");
                        ui.radio_value(&mut self.config.display_mode, DisplayMode::Borderless, "Borderless");
                        ui.radio_value(&mut self.config.display_mode, DisplayMode::Fullscreen, "Fullscreen");
                    });
                    
                    ui.heading("Performance Settings");
                    ui.horizontal(|ui| {
                        ui.label("Profile:");
                        ui.radio_value(&mut self.config.performance_profile, PerformanceProfile::PowerSaver, "Power Saver");
                        ui.radio_value(&mut self.config.performance_profile, PerformanceProfile::Balanced, "Balanced");
                        ui.radio_value(&mut self.config.performance_profile, PerformanceProfile::Performance, "Performance");
                        ui.radio_value(&mut self.config.performance_profile, PerformanceProfile::Custom, "Custom");
                    });
                    
                    ui.add_space(10.0);
                    if ui.button("Apply").clicked() {
                        system::apply_profile(&self.config);
                        self.show_settings = false;
                    }
                });
        }
    }
}

impl Console {
    pub fn new() -> Self {
        let mut console = Self {
            lines: VecDeque::with_capacity(MAX_LINES),
            current_input: String::new(),
            prompt: DEFAULT_PROMPT.to_string(),
            cursor_pos: 0,
            history: VecDeque::with_capacity(100),
            history_index: None,
        };
        
        console.println("OS Gaming Console v0.1.0");
        console.println("Type 'help' for commands");
        console
    }
    
    pub fn println(&mut self, text: &str) {
        self.lines.push_back(text.to_string());
        if self.lines.len() > MAX_LINES {
            self.lines.pop_front();
        }
    }
    
    pub fn handle_key(&mut self, c: char) -> bool {
        match c {
            '\n' => {
                let command = self.current_input.clone();
                self.lines.push_back(format!("{}{}", self.prompt, self.current_input));
                self.current_input.clear();
                self.cursor_pos = 0;
                
                if !command.is_empty() {
                    self.history.push_back(command.clone());
                    if self.history.len() > 100 {
                        self.history.pop_front();
                    }
                }
                
                self.history_index = None;
                return true; // Command submitted
            }
            '\u{0008}' => { // Backspace
                if self.cursor_pos > 0 {
                    self.current_input.remove(self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                }
            }
            '\u{001B}' => {} // Escape key, ignore for now
            _ => {
                self.current_input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
        }
        false // No command submitted
    }
    
    pub fn handle_special_key(&mut self, key: egui::Key) {
        match key {
            egui::Key::ArrowLeft => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            egui::Key::ArrowRight => {
                if self.cursor_pos < self.current_input.len() {
                    self.cursor_pos += 1;
                }
            }
            egui::Key::ArrowUp => {
                if self.history.is_empty() {
                    return;
                }
                
                let history_len = self.history.len();
                self.history_index = Some(match self.history_index {
                    Some(idx) if idx > 0 => idx - 1,
                    _ => history_len - 1,
                });
                
                if let Some(idx) = self.history_index {
                    self.current_input = self.history[idx].clone();
                    self.cursor_pos = self.current_input.len();
                }
            }
            egui::Key::ArrowDown => {
                if self.history.is_empty() || self.history_index.is_none() {
                    return;
                }
                
                let history_len = self.history.len();
                self.history_index = Some(match self.history_index {
                    Some(idx) if idx < history_len - 1 => idx + 1,
                    _ => 0,
                });
                
                if let Some(idx) = self.history_index {
                    self.current_input = self.history[idx].clone();
                    self.cursor_pos = self.current_input.len();
                }
            }
            egui::Key::Home => {
                self.cursor_pos = 0;
            }
            egui::Key::End => {
                self.cursor_pos = self.current_input.len();
            }
            _ => {}
        }
    }
    
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut command_executed = false;
        
        let text_style = egui::TextStyle::Monospace;
        let row_height = ui.text_style_height(&text_style);
        
        // Console output area (scrollable)
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .max_height(ui.available_height() - row_height * 2.0)
            .show(ui, |ui| {
                for line in &self.lines {
                    ui.label(line);
                }
            });
        
        // Input area with prompt
        ui.horizontal(|ui| {
            ui.label(&self.prompt);
            
            // Custom text editor with cursor
            let response = ui.add(egui::TextEdit::singleline(&mut self.current_input)
                .font(egui::TextStyle::Monospace)
                .desired_width(ui.available_width())
                .lock_focus(true)
                .cursor_at_end(false));
                
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let command = self.current_input.clone();
                self.lines.push_back(format!("{}{}", self.prompt, self.current_input));
                self.current_input.clear();
                self.cursor_pos = 0;
                
                if !command.is_empty() {
                    self.history.push_back(command.clone());
                    if self.history.len() > 100 {
                        self.history.pop_front();
                    }
                }
                
                self.history_index = None;
                command_executed = true;
            }
            
            // Handle keyboard input
            if response.has_focus() {
                ui.input(|i| {
                    // Process special keys
                    for key in &[
                        egui::Key::ArrowLeft, egui::Key::ArrowRight,
                        egui::Key::ArrowUp, egui::Key::ArrowDown,
                        egui::Key::Home, egui::Key::End,
                    ] {
                        if i.key_pressed(*key) {
                            self.handle_special_key(*key);
                        }
                    }
                });
            }
        });
        
        command_executed
    }
}