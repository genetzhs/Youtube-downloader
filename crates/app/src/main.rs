//! YouTube Downloader: a desktop app for yt-dlp.
//!
//! Options (mainly for development and screenshots):
//!   --demo                 fill the queue with sample rows
//!   --theme dark|light     use this theme without saving it
//!   --page history|settings  open on this page
//!   <url>...               add these links to the queue (used by "Open with")

mod app;
mod demo;
mod history_page;
mod icons;
mod paths;
mod settings_page;
mod system;
mod theme;
mod views;
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

    let page = match args
        .iter()
        .position(|a| a == "--page")
        .and_then(|i| args.get(i + 1))
    {
        Some(p) if p == "history" => app::Page::History,
        Some(p) if p == "settings" => app::Page::Settings,
        _ => app::Page::Downloads,
    };
    let links: Vec<String> = args
        .iter()
        .filter(|a| a.contains("://") || a.starts_with("www."))
        .cloned()
        .collect();

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
        Box::new(move |cc| {
            let mut app = app::App::new(cc, demo, theme);
            app.page = page;
            if !links.is_empty() {
                app.add_links(&links.join("\n"));
            }
            Ok(Box::new(app))
        }),
    )
}
