#![windows_subsystem = "windows"]

mod app;
mod client;
mod config;
mod embedded;
mod installer;
mod protocol;
mod theme;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("青色大脑 修改器")
            .with_inner_size([1000.0, 760.0])
            .with_resizable(false)
            // 不用系统标题栏，标题区自己画，和游戏的界面风格一致
            .with_decorations(false)
            .with_icon(theme::window_icon()),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "CyanBrainTrainer",
        options,
        Box::new(|cc| Ok(Box::new(app::TrainerApp::new(cc)))),
    )
}
