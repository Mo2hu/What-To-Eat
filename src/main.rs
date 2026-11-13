#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([620.0, 600.0])
            .with_min_inner_size([480.0, 420.0])
            .with_title("今天吃什么"),
        ..Default::default()
    };

    eframe::run_native(
        "WTE",
        options,
        Box::new(|cc| Ok(Box::new(app::WteApp::new(cc)))),
    )
}
