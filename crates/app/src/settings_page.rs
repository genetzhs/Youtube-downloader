//! The Settings page. Every change is saved right away.

use std::path::PathBuf;

use egui::{Align, Layout};
use ytdl_core::config::Theme;
use ytdl_core::options::{Browser, JsRuntime};
use ytdl_core::tools::Source;

use crate::app::App;
use crate::paths;
use crate::system::browser_name;
use crate::theme::{Weight, palette};
use crate::widgets::{self as w, Kind};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let mut changed = false;
    ui.vertical_centered(|ui| {
        ui.set_max_width(860.0);
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            ui.label(
                egui::RichText::new("Settings").font(crate::theme::font(22.0, Weight::SemiBold)),
            );
            ui.add_space(12.0);
            changed |= downloads(app, ui);
            ui.add_space(14.0);
            changed |= sign_in(app, ui);
            ui.add_space(14.0);
            changed |= files(app, ui);
            ui.add_space(14.0);
            changed |= tools(app, ui);
            ui.add_space(14.0);
            changed |= appearance(app, ui);
            ui.add_space(14.0);
            about(ui);
        });
    });
    if changed {
        app.save_config();
    }
}

fn folder_row(
    ui: &mut egui::Ui,
    first: bool,
    title: &str,
    desc: &str,
    value: &mut Option<PathBuf>,
    default: &PathBuf,
) -> bool {
    let mut changed = false;
    w::setting_row(ui, first, title, desc, |ui| {
        if w::button(ui, Kind::Secondary, None, "Browse…", 38.0).clicked() {
            let start = value.clone().unwrap_or_else(|| default.clone());
            if let Some(dir) = rfd::FileDialog::new().set_directory(&start).pick_folder() {
                *value = Some(dir);
                changed = true;
            }
        }
        let mut text = paths::display(value.as_ref().unwrap_or(default));
        if w::text_field(ui, &mut text, 300.0, true).changed() {
            *value = (!text.trim().is_empty()).then(|| expand_home(text.trim()));
            changed = true;
        }
    });
    changed
}

fn expand_home(s: &str) -> PathBuf {
    match s.strip_prefix("~/") {
        Some(rest) => paths::home().join(rest),
        None if s == "~" => paths::home(),
        None => PathBuf::from(s),
    }
}

fn downloads(app: &mut App, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    let (audio_default, video_default) = app.default_dirs.clone();
    w::section(ui, "Downloads", |ui| {
        changed |= folder_row(
            ui,
            true,
            "Audio folder",
            "Where Audio downloads are saved",
            &mut app.config.audio_dir,
            &audio_default,
        );
        changed |= folder_row(
            ui,
            false,
            "Video folder",
            "Where Video downloads are saved",
            &mut app.config.video_dir,
            &video_default,
        );
        w::setting_row(
            ui,
            false,
            "File name",
            "yt-dlp output template, e.g. %(uploader)s - %(title)s.%(ext)s",
            |ui| {
                let resp =
                    w::text_field(ui, &mut app.config.options.filename_template, 300.0, true);
                if resp.changed() {
                    changed = true;
                }
                if resp.lost_focus() && app.config.options.filename_template.trim().is_empty() {
                    app.config.options.filename_template = "%(title)s.%(ext)s".into();
                    changed = true;
                }
            },
        );
        w::setting_row(
            ui,
            false,
            "Downloads at the same time",
            "More is faster on a fast connection",
            |ui| {
                for n in (1..=4).rev() {
                    if w::chip(ui, &n.to_string(), app.config.max_parallel == n).clicked() {
                        app.config.max_parallel = n;
                        app.downloader.set_max_parallel(n);
                        changed = true;
                    }
                }
            },
        );
    });
    changed
}

fn sign_in(app: &mut App, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    let installed = crate::system::installed_browsers();
    w::section(ui, "Sign-in & anti-bot", |ui| {
        w::setting_row(
            ui,
            true,
            "Cookies from browser",
            "Lets yt-dlp act as your signed-in browser. Needed when YouTube asks to confirm you're not a bot.",
            |_| {},
        );
        ui.horizontal_wrapped(|ui| {
            let current = app.config.options.cookies_from;
            if w::chip(ui, "Don't use", current.is_none()).clicked() {
                app.config.options.cookies_from = None;
                changed = true;
            }
            let mut browsers: Vec<Browser> = installed.clone();
            browsers.extend(Browser::ALL.iter().filter(|b| !installed.contains(b)));
            for b in browsers {
                if cfg!(not(target_os = "macos")) && b == Browser::Safari {
                    continue;
                }
                let label = if installed.contains(&b) {
                    browser_name(b).to_string()
                } else {
                    format!("{} (not found)", browser_name(b))
                };
                if w::chip(ui, &label, current == Some(b)).clicked() {
                    app.config.options.cookies_from = Some(b);
                    changed = true;
                }
            }
        });
        ui.add_space(8.0);
        w::setting_row(
            ui,
            false,
            "JavaScript runtime",
            "Solves YouTube's JavaScript challenge. Uses the one installed on your system.",
            |ui| {
                let current = app.config.options.js_runtime;
                if w::chip(ui, "Off", current.is_none()).clicked() {
                    app.config.options.js_runtime = None;
                    changed = true;
                }
                for rt in JsRuntime::ALL.iter().rev() {
                    if w::chip(ui, rt.as_str(), current == Some(*rt)).clicked() {
                        app.config.options.js_runtime = Some(*rt);
                        changed = true;
                    }
                }
            },
        );
    });
    changed
}

fn files(app: &mut App, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    let o = &mut app.config.options;
    w::section(ui, "Files", |ui| {
        for (i, (title, desc, value)) in [
            (
                "Tags",
                "Write title, artist and other metadata into the file",
                &mut o.embed_metadata,
            ),
            (
                "Info file",
                "Save a .info.json file with all details next to each download",
                &mut o.write_info_json,
            ),
            (
                "Replace existing files",
                "Download again even if the file is already there",
                &mut o.overwrite,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            w::setting_row(ui, i == 0, title, desc, |ui| {
                changed |= w::switch(ui, value, title).changed();
            });
        }
    });
    changed
}

fn tools(app: &mut App, ui: &mut egui::Ui) -> bool {
    let p = palette(ui.ctx());
    let mut changed = false;
    let mut update_now = false;
    let mut refresh = false;
    w::section(ui, "Tools", |ui| {
        let t = &app.tools;
        let ytdlp_desc = match (&t.ytdlp, t.ytdlp_source) {
            (Some(path), Some(source)) => {
                let from = match source {
                    Source::Override => "your custom path",
                    Source::Updated => "updated by the app",
                    Source::Bundled => "included with the app",
                    Source::System => "found on this system",
                };
                format!("{} · {from}", paths::display(path))
            }
            _ => "Not found. Check for update downloads it.".into(),
        };
        let version = if t.updating {
            "Updating…".to_string()
        } else if let Some(e) = &t.update_error {
            format!("Update failed: {e}")
        } else {
            t.ytdlp_version.clone().unwrap_or_default()
        };
        w::setting_row(ui, true, "yt-dlp", &ytdlp_desc, |ui| {
            if w::button(ui, Kind::Secondary, None, "Check for update", 38.0).clicked()
                && !app.tools.updating
            {
                update_now = true;
            }
            w::text(ui, &version, 12.0, Weight::Regular, p.muted);
        });
        w::setting_row(
            ui,
            false,
            "Update yt-dlp automatically",
            "Checks once each time the app starts",
            |ui| {
                changed |= w::switch(
                    ui,
                    &mut app.config.auto_update_ytdlp,
                    "Update yt-dlp automatically",
                )
                .changed();
            },
        );
        let ffmpeg_desc = app.tools.ffmpeg.as_ref().map_or(
            "Not found. Audio conversion and merging video need it.".to_string(),
            |p| paths::display(p),
        );
        w::setting_row(ui, false, "ffmpeg", &ffmpeg_desc, |_| {});
        w::setting_row(
            ui,
            false,
            "Custom yt-dlp",
            "Use this yt-dlp instead (leave empty for the app's own)",
            |ui| {
                let mut text = app
                    .config
                    .ytdlp_override
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                if w::text_field(ui, &mut text, 300.0, true).changed() {
                    app.config.ytdlp_override =
                        (!text.trim().is_empty()).then(|| expand_home(text.trim()));
                    changed = true;
                    refresh = true;
                }
            },
        );
    });
    if update_now {
        app.update_ytdlp_now();
    }
    if refresh {
        app.refresh_tools();
    }
    changed
}

fn appearance(app: &mut App, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    let mut theme = None;
    w::section(ui, "Appearance", |ui| {
        w::setting_row(
            ui,
            true,
            "Theme",
            "System follows your desktop's light or dark setting",
            |ui| {
                for (t, label) in [
                    (Theme::Light, "Light"),
                    (Theme::Dark, "Dark"),
                    (Theme::System, "System"),
                ] {
                    if w::chip(ui, label, app.config.theme == t).clicked() {
                        theme = Some(t);
                    }
                }
            },
        );
        w::setting_row(
            ui,
            false,
            "Notify when finished",
            "Shows a notification when all downloads are done and the window is in the background",
            |ui| {
                changed |= w::switch(ui, &mut app.config.notify_when_done, "Notify when finished")
                    .changed();
            },
        );
    });
    if let Some(t) = theme {
        app.set_theme(t); // saves on its own
    }
    changed
}

fn about(ui: &mut egui::Ui) {
    let p = palette(ui.ctx());
    w::section(ui, "About", |ui| {
        w::text(
            ui,
            &format!("YouTube Downloader {}", env!("CARGO_PKG_VERSION")),
            14.0,
            Weight::Medium,
            p.text,
        );
        ui.horizontal_wrapped(|ui| {
            ui.hyperlink_to(
                "Project page",
                "https://github.com/genetzhs/Youtube-downloader",
            );
            w::text(ui, "·", 12.0, Weight::Regular, p.faint);
            ui.hyperlink_to(
                "Report a problem",
                "https://github.com/genetzhs/Youtube-downloader/issues",
            );
            w::text(ui, "·", 12.0, Weight::Regular, p.faint);
            ui.hyperlink_to("yt-dlp", "https://github.com/yt-dlp/yt-dlp");
        });
        w::text(
            ui,
            "Downloads use yt-dlp and ffmpeg. Download only content you have the right to download.",
            12.0,
            Weight::Regular,
            p.faint,
        );
    });
}
