mod config;
mod ffmpeg_tools;
mod i18n;
mod models;
mod theme;
mod ui;
mod ytdlp;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RustTube — YouTube Studio Downloader")
            .with_inner_size([1080.0, 740.0])
            .with_min_inner_size([860.0, 580.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RustTube",
        options,
        Box::new(|cc| Ok(Box::new(ui::RustTubeApp::new(cc)))),
    )
}
