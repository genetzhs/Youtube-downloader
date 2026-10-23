//! Finished downloads: recording them, notifying, and the History page.

use std::time::{SystemTime, UNIX_EPOCH};

use egui::{Align, Color32, Layout, Margin, Vec2};
use ytdl_core::history::Entry;
use ytdl_core::queue::Status;
use ytdl_runner::Finished;

use crate::app::{App, Page};
use crate::icons::{self, Icon};
use crate::paths;
use crate::system;
use crate::theme::{Weight, palette, radius};
use crate::widgets::{self as w, Badge, Kind};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Adds finished items to the history and, when the whole queue is done
/// while the window is in the background, shows a desktop notification.
pub fn record(app: &mut App, finished: &[Finished]) {
    for f in finished {
        let Some(item) = app.downloader.queue().get(f.id) else {
            continue;
        };
        app.history.push(Entry {
            url: item.url.clone(),
            title: item.info.as_ref().and_then(|i| i.title.clone()),
            path: item.final_path.clone(),
            summary: item.options.summary(),
            finished_at: now(),
            success: f.status == Status::Done,
        });
    }
    if let Some(path) = paths::history_file()
        && let Err(e) = app.history.save(&path)
    {
        eprintln!("could not save history: {e}");
    }

    let focused = app.ctx.input(|i| i.focused);
    if app.config.notify_when_done && !app.downloader.is_busy() && !focused {
        let c = app.downloader.queue().counts();
        let body = match (c.done, c.failed) {
            (d, 0) => format!("{d} finished"),
            (0, f) => format!("{f} failed"),
            (d, f) => format!("{d} finished, {f} failed"),
        };
        let _ = notify_rust::Notification::new()
            .summary("Downloads complete")
            .body(&body)
            .appname("YouTube Downloader")
            .show();
    }
}

/// `Today`, `Yesterday` or `5 days ago`.
fn when(finished_at: u64, now: u64) -> String {
    let days = now.saturating_sub(finished_at) / 86_400;
    match days {
        0 => "Today".into(),
        1 => "Yesterday".into(),
        d => format!("{d} days ago"),
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let p = palette(ui.ctx());
    ui.vertical_centered(|ui| {
        ui.set_max_width(860.0);
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("History").font(crate::theme::font(22.0, Weight::SemiBold)),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !app.history.entries.is_empty()
                        && w::button(ui, Kind::Ghost, None, "Clear history", 36.0).clicked()
                    {
                        app.history.clear();
                        if let Some(path) = paths::history_file() {
                            let _ = app.history.save(&path);
                        }
                    }
                });
            });
            ui.add_space(8.0);
            w::text_field(
                ui,
                &mut app.history_search,
                ui.available_width() - 24.0,
                false,
            )
            .on_hover_text("Search by title or link");
            if app.history_search.is_empty() {
                // A hint inside an empty field would need a custom widget; a line below is enough.
                w::text(
                    ui,
                    "Type to search by title or link",
                    12.0,
                    Weight::Regular,
                    p.faint,
                );
            }
            ui.add_space(12.0);

            let query = app.history_search.to_lowercase();
            let now = now();
            let entries: Vec<(usize, Entry)> = app
                .history
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    query.is_empty()
                        || e.url.to_lowercase().contains(&query)
                        || e.title
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&query)
                })
                .map(|(i, e)| (i, e.clone()))
                .collect();

            if entries.is_empty() {
                w::card(Color32::TRANSPARENT, p.border, radius::CARD)
                    .inner_margin(Margin::symmetric(24, 36))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            let (r, _) =
                                ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::hover());
                            icons::paint(ui.painter(), r, Icon::History, p.faint);
                            ui.add_space(6.0);
                            let msg = if app.history.entries.is_empty() {
                                "Finished downloads will be listed here"
                            } else {
                                "Nothing matches your search"
                            };
                            w::text(ui, msg, 14.0, Weight::Medium, p.muted);
                        });
                    });
                return;
            }

            let mut again: Option<String> = None;
            for (_, e) in entries.iter().take(300) {
                w::card(p.surface, p.border, radius::CARD).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 14.0;
                        w::badge(
                            ui,
                            if e.success {
                                Badge::Done
                            } else {
                                Badge::Warning
                            },
                            36.0,
                        );
                        let right_w = 230.0;
                        ui.allocate_ui(
                            egui::vec2((ui.available_width() - right_w).max(80.0), 40.0),
                            |ui| {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 3.0;
                                    let title = e.title.clone().unwrap_or_else(|| e.url.clone());
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(title)
                                                .font(crate::theme::font(14.0, Weight::SemiBold)),
                                        )
                                        .truncate(),
                                    );
                                    let status = if e.success { "" } else { " · failed" };
                                    let meta = format!(
                                        "{} · {}{status}",
                                        e.summary,
                                        when(e.finished_at, now)
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(meta)
                                                .font(crate::theme::font(12.0, Weight::Regular))
                                                .color(p.faint),
                                        )
                                        .truncate(),
                                    );
                                });
                            },
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if w::button(ui, Kind::Secondary, Some(Icon::Download), "Again", 34.0)
                                .on_hover_text("Download again with the current settings")
                                .clicked()
                            {
                                again = Some(e.url.clone());
                            }
                            if let Some(path) = e.path.as_ref().filter(|p| p.exists())
                                && w::icon_button(ui, Icon::Folder, "Show in folder").clicked()
                            {
                                system::reveal(path);
                            }
                        });
                    });
                });
                ui.add_space(6.0);
            }
            if let Some(url) = again {
                let out_dir = app.config.out_dir(&app.default_dirs);
                app.downloader.add_text(&url, &app.config.options, &out_dir);
                app.page = Page::Downloads;
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::when;

    #[test]
    fn relative_days() {
        let now = 10 * 86_400 + 500;
        assert_eq!(when(now - 100, now), "Today");
        assert_eq!(when(now - 86_400, now), "Yesterday");
        assert_eq!(when(now - 5 * 86_400, now), "5 days ago");
    }
}
