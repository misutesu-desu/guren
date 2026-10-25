#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
#[cfg(windows)]
mod assoc;
mod folder;
mod i18n;
#[cfg(windows)]
mod ipc;
mod loader;
mod theme;
mod view;

use eframe::egui;

fn main() -> eframe::Result {
    let start = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    #[cfg(windows)]
    if ipc::hand_off(start.as_deref()) {
        return Ok(());
    }
    // Creating the window and its GPU context takes a few hundred milliseconds,
    // so the photo is decoded in the meantime.
    let loader = loader::Loader::new();
    if let Some(path) = start.as_ref().filter(|p| p.is_file()) {
        loader.want([path.clone()]);
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Guren")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([480.0, 320.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Guren",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc, start, loader)))),
    )
}
