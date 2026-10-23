//! The main window.

use std::path::PathBuf;

use egui::{Align, Color32, CornerRadius, Layout, Margin, Rect, UiBuilder, Vec2, vec2};
use ytdl_core::config::{Config, Theme as ThemeSetting, default_dirs};
use ytdl_core::options::{AudioFormat, AudioQuality, Container, Mode, Resolution};

use crate::icons::{self, Icon};
use crate::paths;
use crate::theme::{self, Weight, palette, radius, stroke};
use crate::widgets::{self as w, Badge, Kind};

/// Shared with the demo rows in [`crate::demo`].
#[derive(Clone)]
pub enum RowState {
    Downloading { fraction: f32, percent: String },
    Waiting,
    Done,
    NeedsCookies,
}

#[derive(Clone)]
pub struct Row {
    pub title: String,
    pub meta: String,
    pub state: RowState,
}

pub struct App {
    config: Config,
    config_path: Option<PathBuf>,
    /// Theme forced from the command line; not saved.
    theme_override: Option<ThemeSetting>,
    default_dirs: (PathBuf, PathBuf),
    link: String,
    rows: Vec<Row>,
    selected: usize,
    /// Row under the pointer in the previous frame, so its background is
    /// drawn behind the content instead of over it.
    hovered: Option<usize>,
    hovered_next: Option<usize>,
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Audio,
    Video,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        demo: bool,
        theme_override: Option<ThemeSetting>,
    ) -> Self {
        theme::install(&cc.egui_ctx);
        let config_path = paths::config_file();
        let config = config_path
            .as_deref()
            .and_then(|p| Config::load(p).ok())
            .unwrap_or_default();
        theme::apply_setting(&cc.egui_ctx, theme_override.unwrap_or(config.theme));
        Self {
            config,
            config_path,
            theme_override,
            default_dirs: default_dirs(&paths::home(), cfg!(target_os = "macos")),
            link: String::new(),
            rows: if demo {
                crate::demo::rows()
            } else {
                Vec::new()
            },
            selected: 0,
            hovered: None,
            hovered_next: None,
        }
    }

    fn save_config(&self) {
        if let Some(path) = &self.config_path
            && let Err(e) = self.config.save(path)
        {
            eprintln!("could not save settings: {e}");
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        let full = ui.max_rect();
        ui.horizontal(|ui| {
            ui.set_height(44.0);
            // Logo: the download arrow in an inverted rounded square.
            let (logo, _) = ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(logo, CornerRadius::same(8), p.text);
            icons::paint(ui.painter(), logo.shrink(7.0), Icon::Download, p.bg);
            w::text(ui, "YouTube Downloader", 15.0, Weight::SemiBold, p.text);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if w::icon_button(ui, Icon::Sliders, "Settings").clicked() {
                    // Settings arrive in M5.
                }
                if w::icon_button(ui, Icon::History, "History").clicked() {
                    // History arrives in M5.
                }
                self.theme_button(ui);
            });
        });

        // The Audio | Video switch sits in the true center of the window.
        let center = Rect::from_center_size(full.center(), vec2(320.0, 44.0));
        ui.scope_builder(UiBuilder::new().max_rect(center), |ui| {
            w::centered_row(ui, "tabs", |ui| {
                let mut tab = if self.config.is_video() {
                    Tab::Video
                } else {
                    Tab::Audio
                };
                let changed = w::segmented(
                    ui,
                    &mut tab,
                    &[
                        (Tab::Audio, Icon::Music, "Audio"),
                        (Tab::Video, Icon::Video, "Video"),
                    ],
                );
                if changed {
                    self.config.set_video(tab == Tab::Video);
                    self.save_config();
                }
            });
        });
    }

    /// Cycles System → Light → Dark. The Settings page will also offer it.
    fn theme_button(&mut self, ui: &mut egui::Ui) {
        let current = self.theme_override.unwrap_or(self.config.theme);
        let (icon, name) = match current {
            ThemeSetting::System => (Icon::Monitor, "System"),
            ThemeSetting::Light => (Icon::Sun, "Light"),
            ThemeSetting::Dark => (Icon::Moon, "Dark"),
        };
        let label = format!("Theme: {name} (click to change)");
        if w::icon_button(ui, icon, &label).clicked() {
            let next = match current {
                ThemeSetting::System => ThemeSetting::Light,
                ThemeSetting::Light => ThemeSetting::Dark,
                ThemeSetting::Dark => ThemeSetting::System,
            };
            self.theme_override = None;
            self.config.theme = next;
            theme::apply_setting(ui.ctx(), next);
            self.save_config();
        }
    }

    fn add_section(&mut self, ui: &mut egui::Ui) {
        let paste_hint = if cfg!(target_os = "macos") {
            "⌘V adds instantly"
        } else {
            "Ctrl+V adds instantly"
        };
        ui.vertical_centered(|ui| {
            ui.set_max_width(820.0);
            let field = w::link_field(ui, &mut self.link, paste_hint);
            if field.submitted && !self.link.trim().is_empty() {
                // The queue is connected in M4.
            }
        });
        ui.add_space(14.0);

        let mut changed = false;
        match self.config.options.mode {
            Mode::Video {
                resolution,
                container,
            } => {
                w::centered_row(ui, "video-choices", |ui| {
                    w::chip_label(ui, "Quality");
                    for r in Resolution::CHOICES {
                        let label = r.0.map_or("Best".to_string(), |h| format!("{h}p"));
                        if w::chip(ui, &label, r == resolution).clicked() {
                            self.config.options.mode = Mode::Video {
                                resolution: r,
                                container,
                            };
                            changed = true;
                        }
                    }
                    w::divider(ui);
                    w::chip_label(ui, "Format");
                    for c in Container::ALL {
                        if w::chip(
                            ui,
                            &c.as_str().to_uppercase().replace("WEBM", "WebM"),
                            c == container,
                        )
                        .clicked()
                        {
                            self.config.options.mode = Mode::Video {
                                resolution,
                                container: c,
                            };
                            changed = true;
                        }
                    }
                });
            }
            Mode::Audio { format, quality } => {
                w::centered_row(ui, "audio-choices", |ui| {
                    w::chip_label(ui, "Format");
                    for f in AudioFormat::ALL {
                        if f == AudioFormat::Vorbis {
                            continue; // still available from Settings
                        }
                        let label = match f {
                            AudioFormat::Best => "Original".to_string(),
                            AudioFormat::Opus => "Opus".to_string(),
                            other => other.as_str().to_uppercase(),
                        };
                        if w::chip(ui, &label, f == format).clicked() {
                            self.config.options.mode = Mode::Audio { format: f, quality };
                            changed = true;
                        }
                    }
                    if !format.is_lossless() {
                        w::divider(ui);
                        w::chip_label(ui, "Bitrate");
                        for q in
                            [None, Some(320), Some(256), Some(192), Some(128)].map(AudioQuality)
                        {
                            let label = q.0.map_or("Best".to_string(), |k| format!("{k}k"));
                            if w::chip(ui, &label, q == quality).clicked() {
                                self.config.options.mode = Mode::Audio { format, quality: q };
                                changed = true;
                            }
                        }
                    }
                });
            }
        }
        ui.add_space(6.0);
        let video = self.config.is_video();
        w::centered_row(ui, "extras", |ui| {
            let o = &mut self.config.options;
            for (label, value) in [
                ("Single video, skip playlist", &mut o.no_playlist),
                ("Cover art", &mut o.embed_thumbnail),
                ("Subtitles", &mut o.embed_subs),
            ] {
                if label == "Subtitles" && !video {
                    continue;
                }
                let text = format!("{label} · {}", if *value { "on" } else { "off" });
                if w::chip(ui, &text, false).clicked() {
                    *value = !*value;
                    changed = true;
                }
            }
        });
        if changed {
            self.save_config();
        }
    }

    fn queue(&mut self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            w::text(ui, "Queue", 14.0, Weight::SemiBold, p.text);
            let active = self
                .rows
                .iter()
                .filter(|r| matches!(r.state, RowState::Downloading { .. }))
                .count();
            let waiting = self
                .rows
                .iter()
                .filter(|r| matches!(r.state, RowState::Waiting))
                .count();
            w::text(
                ui,
                &format!(
                    "{active} of {} slots active · {waiting} waiting",
                    self.config.max_parallel
                ),
                12.0,
                Weight::Regular,
                p.faint,
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                w::button(ui, Kind::Ghost, None, "Pause all", 36.0);
            });
        });
        ui.add_space(4.0);

        if self.rows.is_empty() {
            self.empty_state(ui);
            return;
        }
        for i in 0..self.rows.len() {
            self.row(ui, i);
            ui.add_space(4.0);
        }
    }

    fn empty_state(&self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        w::card(Color32::TRANSPARENT, p.border, radius::CARD)
            .inner_margin(Margin::symmetric(24, 40))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());
                    icons::paint(ui.painter(), r, Icon::Download, p.faint);
                    ui.add_space(6.0);
                    w::text(
                        ui,
                        "Your downloads will appear here",
                        15.0,
                        Weight::SemiBold,
                        p.text,
                    );
                    w::text(
                        ui,
                        "Paste one or more links above to get started.",
                        13.0,
                        Weight::Regular,
                        p.muted,
                    );
                });
            });
    }

    fn row(&mut self, ui: &mut egui::Ui, i: usize) {
        let p = palette(ui.ctx());
        let row = self.rows[i].clone();
        let selected = i == self.selected;
        let warning = matches!(row.state, RowState::NeedsCookies);
        let (fill, border) = if warning {
            (p.warn_bg, p.warn_border)
        } else if selected {
            (p.surface, p.selected_border)
        } else if self.hovered == Some(i) {
            (p.surface, Color32::TRANSPARENT)
        } else {
            (Color32::TRANSPARENT, Color32::TRANSPARENT)
        };
        let resp = w::card(fill, border, radius::CARD)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    match &row.state {
                        RowState::Downloading { fraction, .. } => {
                            w::progress_ring(ui, Some(*fraction), 44.0);
                        }
                        RowState::Waiting => {
                            w::progress_ring(ui, None, 44.0);
                        }
                        RowState::Done => w::badge(ui, Badge::Done, 44.0),
                        RowState::NeedsCookies => w::badge(ui, Badge::Warning, 44.0),
                    }
                    let right_w = match row.state {
                        RowState::Done => 150.0,
                        RowState::NeedsCookies => 200.0,
                        _ => 110.0,
                    };
                    ui.allocate_ui(vec2(ui.available_width() - right_w, 44.0), |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 3.0;
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&row.title)
                                        .font(theme::font(14.0, Weight::SemiBold)),
                                )
                                .truncate(),
                            );
                            let meta_color = if warning { p.warn_text } else { p.faint };
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&row.meta)
                                        .font(theme::font(12.0, Weight::Regular))
                                        .color(meta_color),
                                )
                                .truncate(),
                            );
                        });
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        match &row.state {
                            RowState::Downloading { percent, .. } => {
                                w::icon_button(ui, Icon::Pause, "Pause");
                                w::text(ui, percent, 13.0, Weight::SemiBold, p.text);
                            }
                            RowState::Waiting => {
                                w::icon_button(ui, Icon::Close, "Remove");
                            }
                            RowState::Done => {
                                w::button(ui, Kind::Secondary, None, "Show in folder", 36.0);
                            }
                            RowState::NeedsCookies => {
                                w::button(ui, Kind::Warning, None, "Use Firefox cookies", 36.0);
                            }
                        }
                    });
                });
            })
            .response;
        if selected && !warning {
            ui.painter().rect_stroke(
                resp.rect.expand(1.0),
                CornerRadius::same(radius::CARD + 1),
                stroke(3.0, p.selected_glow),
                egui::StrokeKind::Outside,
            );
        }
        let click = ui.interact(resp.rect, ui.id().with(("row", i)), egui::Sense::click());
        if click.clicked() {
            self.selected = i;
        }
        if click.hovered() {
            self.hovered_next = Some(i);
        }
        if self.hovered != self.hovered_next && self.hovered_next == Some(i) {
            ui.ctx().request_repaint();
        }
    }

    fn details(&self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        let Some(row) = self.rows.get(self.selected) else {
            return;
        };
        w::card(p.surface, p.border, radius::PANEL)
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 14.0;
                // Thumbnail placeholder until real thumbnails load (M4).
                let w_avail = ui.available_width();
                let (thumb, _) = ui
                    .allocate_exact_size(vec2(w_avail, w_avail * 9.0 / 16.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(thumb, CornerRadius::same(12), p.track);
                let badge =
                    Rect::from_min_size(thumb.right_bottom() - vec2(52.0, 30.0), vec2(42.0, 20.0));
                ui.painter().rect_filled(
                    badge,
                    CornerRadius::same(6),
                    Color32::from_black_alpha(190),
                );
                w::paint_text(
                    ui,
                    badge.center(),
                    egui::Align2::CENTER_CENTER,
                    "12:48",
                    11.0,
                    Weight::SemiBold,
                    Color32::WHITE,
                );

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    w::text(ui, &row.title, 16.0, Weight::SemiBold, p.text);
                    w::text(ui, "[Channel]", 12.0, Weight::Regular, p.faint);
                });
                if let RowState::Downloading { fraction, .. } = row.state {
                    w::progress_bar(ui, fraction);
                }
                for pair in [
                    [
                        ("Quality", "1080p · 60 fps"),
                        ("Format", "MP4 · H.264 + AAC"),
                    ],
                    [("Size", "248 MB"), ("Speed", "12.4 MB/s")],
                ] {
                    ui.columns(2, |cols| {
                        for (col, (k, v)) in cols.iter_mut().zip(pair) {
                            col.spacing_mut().item_spacing.y = 2.0;
                            w::text(col, k, 12.0, Weight::Regular, p.faint);
                            w::text(col, v, 13.0, Weight::Medium, p.text);
                        }
                    });
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    w::text(ui, "Saving to", 12.0, Weight::Regular, p.faint);
                    let dir = paths::display(&self.config.out_dir(&self.default_dirs));
                    w::mono_box(ui, &format!("{dir}/[Video title].mp4"));
                });
                ui.horizontal(|ui| {
                    w::button(ui, Kind::Secondary, Some(Icon::Copy), "Copy command", 40.0);
                    w::button(ui, Kind::Danger, None, "Cancel", 40.0);
                });
            });
    }

    fn footer(&self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
            icons::paint(ui.painter(), r, Icon::Folder, p.muted);
            let dir = paths::display(&self.config.out_dir(&self.default_dirs));
            w::text(ui, &dir, 12.0, Weight::Regular, p.muted);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if ui
                    .link(egui::RichText::new("Show log").font(theme::font(12.0, Weight::Medium)))
                    .clicked()
                {
                    // The log view arrives with the real queue (M4).
                }
                ui.add_space(10.0);
                // Real tool status is connected in M4.
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    w::status_dot(ui, true, "yt-dlp up to date");
                    ui.add_space(10.0);
                    w::status_dot(ui, true, "ffmpeg ready");
                });
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let p = palette(ctx);
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(Margin::symmetric(24, 12)),
            )
            .show(ctx, |ui| self.header(ui));
        egui::TopBottomPanel::bottom("footer")
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(Margin::symmetric(24, 10)),
            )
            .show(ctx, |ui| self.footer(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg).inner_margin(Margin {
                left: 24,
                right: 24,
                top: 32,
                bottom: 20,
            }))
            .show(ctx, |ui| {
                self.hovered = self.hovered_next.take();
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        self.add_section(ui);
                        ui.add_space(24.0);
                        let wide = ui.available_width() >= 900.0 && !self.rows.is_empty();
                        if wide {
                            let side = 340.0;
                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing.x = 20.0;
                                ui.allocate_ui(
                                    // Leave room for the scroll bar on the right.
                                    vec2(ui.available_width() - side - 32.0, 0.0),
                                    |ui| {
                                        ui.vertical(|ui| self.queue(ui));
                                    },
                                );
                                ui.allocate_ui_with_layout(
                                    vec2(side, 0.0),
                                    Layout::top_down(Align::Min),
                                    |ui| {
                                        ui.set_width(side);
                                        self.details(ui);
                                    },
                                );
                            });
                        } else {
                            self.queue(ui);
                            if !self.rows.is_empty() {
                                ui.add_space(16.0);
                                self.details(ui);
                            }
                        }
                    });
            });
    }
}
