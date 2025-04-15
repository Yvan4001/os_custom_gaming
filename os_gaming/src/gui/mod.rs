use crate::Config;

pub fn run_app(config: Config) {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "OS Gaming",
        native_options,
        Box::new(|cc| Ok(Box::new(app::OsGamingApp::new(cc, config))))
    ).expect("Failed to start GUI application");
}