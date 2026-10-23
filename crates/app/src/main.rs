//! YouTube Downloader: a desktop app for yt-dlp.
//!
//! Options (mainly for development and screenshots):
//!   --demo                 fill the queue with sample rows
//!   --theme dark|light     use this theme without saving it

mod app;
mod demo;
mod icons;
mod paths;
mod theme;
mod widgets;

use ytdl_core::config::Theme;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let demo = args.iter().any(|a| a == "--demo");
    let theme = args
        .iter()
        .position(|a| a == "--theme")
        .and_then(|i| args.get(i + 1))
        .and_then(|t| match t.as_str() {
            "dark" => Some(Theme::Dark),
            "light" => Some(Theme::Light),
            "system" => Some(Theme::System),
            _ => None,
        });

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("YouTube Downloader")
        .with_app_id("youtube-downloader")
        .with_inner_size([1280.0, 860.0])
        .with_min_inner_size([900.0, 640.0]);
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon_256.png"))
    {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "YouTube Downloader",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, demo, theme)))),
    )
}
