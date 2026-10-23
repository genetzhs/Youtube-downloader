//! Queue rows and the details panel. They draw plain view data, built from
//! the real queue in `app.rs` or from samples in `demo.rs`, and report what
//! the user clicked as an [`Action`].

use egui::{Align, Color32, CornerRadius, Layout, Margin, Rect, Ui, Vec2, vec2};
use ytdl_core::options::Browser;
use ytdl_core::queue::ItemId;

use crate::icons::{self, Icon};
use crate::system::browser_name;
use crate::theme::{self, Weight, palette, radius, stroke};
use crate::widgets::{self as w, Badge, Kind, Ring};

#[derive(Clone, Debug)]
pub enum RowState {
    Active {
        ring: Ring,
        percent: Option<String>,
    },
    Waiting,
    Done,
    Cancelled,
    /// A problem the user can act on; `fix` is the suggested button.
    Problem {
        fix: Fix,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fix {
    UseCookies(Browser),
    Retry,
    None,
}

#[derive(Clone, Debug)]
pub struct RowView {
    pub id: ItemId,
    pub title: String,
    pub meta: String,
    pub state: RowState,
}

#[derive(Clone, Debug, Default)]
pub struct DetailsView {
    pub id: ItemId,
    pub title: String,
    pub channel: Option<String>,
    pub duration: Option<String>,
    pub thumbnail: Option<String>,
    pub fraction: Option<f32>,
    pub facts: Vec<(String, String)>,
    pub path_label: &'static str,
    pub path: String,
    pub stage: Stage,
    pub audio: bool,
}

/// Which buttons the details panel offers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Stage {
    #[default]
    Working,
    Done,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Select(ItemId),
    Cancel(ItemId),
    Remove(ItemId),
    Retry(ItemId),
    UseCookies(ItemId, Browser),
    Reveal(ItemId),
    Open(ItemId),
    CopyCommand(ItemId),
    ShowLog(ItemId),
}

pub fn row(
    ui: &mut Ui,
    row: &RowView,
    selected: bool,
    hovered: bool,
    actions: &mut Vec<Action>,
) -> egui::Response {
    let p = palette(ui.ctx());
    let problem = matches!(row.state, RowState::Problem { .. });
    let (fill, border) = if problem {
        (p.warn_bg, p.warn_border)
    } else if selected {
        (p.surface, p.selected_border)
    } else if hovered {
        (p.surface, Color32::TRANSPARENT)
    } else {
        (Color32::TRANSPARENT, Color32::TRANSPARENT)
    };
    // Sensing on the scope registers the row's click area behind its buttons,
    // so the buttons still get their own clicks.
    let resp = ui
        .scope_builder(egui::UiBuilder::new().sense(egui::Sense::click()), |ui| {
            w::card(fill, border, radius::CARD).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    match &row.state {
                        RowState::Active { ring, .. } => {
                            w::progress_ring(ui, *ring, 44.0);
                        }
                        RowState::Waiting => {
                            w::progress_ring(ui, Ring::Waiting, 44.0);
                        }
                        RowState::Done => w::badge(ui, Badge::Done, 44.0),
                        RowState::Cancelled => w::badge(ui, Badge::Cancelled, 44.0),
                        RowState::Problem { .. } => w::badge(ui, Badge::Warning, 44.0),
                    }
                    let right_w = match &row.state {
                        RowState::Done => 150.0,
                        RowState::Problem {
                            fix: Fix::UseCookies(_),
                        } => 210.0,
                        RowState::Problem { .. } | RowState::Cancelled => 130.0,
                        _ => 110.0,
                    };
                    ui.allocate_ui(
                        vec2((ui.available_width() - right_w).max(80.0), 44.0),
                        |ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 3.0;
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&row.title)
                                            .font(theme::font(14.0, Weight::SemiBold)),
                                    )
                                    .truncate(),
                                );
                                let meta_color = if problem { p.warn_text } else { p.faint };
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&row.meta)
                                            .font(theme::font(12.0, Weight::Regular))
                                            .color(meta_color),
                                    )
                                    .truncate(),
                                );
                            });
                        },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        match &row.state {
                            RowState::Active { percent, .. } => {
                                if w::icon_button(ui, Icon::Close, "Cancel").clicked() {
                                    actions.push(Action::Cancel(row.id));
                                }
                                if let Some(pct) = percent {
                                    w::text(ui, pct, 13.0, Weight::SemiBold, p.text);
                                }
                            }
                            RowState::Waiting => {
                                if w::icon_button(ui, Icon::Close, "Remove from queue").clicked() {
                                    actions.push(Action::Cancel(row.id));
                                }
                            }
                            RowState::Done => {
                                if w::button(ui, Kind::Secondary, None, "Show in folder", 36.0)
                                    .clicked()
                                {
                                    actions.push(Action::Reveal(row.id));
                                }
                            }
                            RowState::Cancelled => {
                                if w::icon_button(ui, Icon::Close, "Remove").clicked() {
                                    actions.push(Action::Remove(row.id));
                                }
                                if w::button(ui, Kind::Secondary, None, "Retry", 36.0).clicked() {
                                    actions.push(Action::Retry(row.id));
                                }
                            }
                            RowState::Problem { fix } => match fix {
                                Fix::UseCookies(b) => {
                                    let label = format!("Use {} cookies", browser_name(*b));
                                    if w::button(ui, Kind::Warning, None, &label, 36.0).clicked() {
                                        actions.push(Action::UseCookies(row.id, *b));
                                    }
                                }
                                Fix::Retry => {
                                    if w::button(ui, Kind::Warning, None, "Retry", 36.0).clicked() {
                                        actions.push(Action::Retry(row.id));
                                    }
                                }
                                Fix::None => {
                                    if w::icon_button(ui, Icon::Close, "Remove").clicked() {
                                        actions.push(Action::Remove(row.id));
                                    }
                                }
                            },
                        }
                    });
                });
            })
        })
        .response;
    if selected && !problem {
        ui.painter().rect_stroke(
            resp.rect.expand(1.0),
            CornerRadius::same(radius::CARD + 1),
            stroke(3.0, p.selected_glow),
            egui::StrokeKind::Outside,
        );
    }
    resp
}

pub fn details(ui: &mut Ui, d: &DetailsView, actions: &mut Vec<Action>) {
    let p = palette(ui.ctx());
    w::card(p.surface, p.border, radius::PANEL)
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 14.0;

            let width = ui.available_width();
            let size = vec2(width, width * 9.0 / 16.0);
            let thumb = match &d.thumbnail {
                Some(url) => {
                    ui.add(
                        egui::Image::new(url.as_str())
                            .fit_to_exact_size(size)
                            .maintain_aspect_ratio(false)
                            .corner_radius(CornerRadius::same(12))
                            .bg_fill(p.track),
                    )
                    .rect
                }
                None => {
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, CornerRadius::same(12), p.track);
                    let icon = Rect::from_center_size(rect.center(), Vec2::splat(32.0));
                    let placeholder = if d.audio { Icon::Music } else { Icon::Video };
                    icons::paint(ui.painter(), icon, placeholder, p.faint);
                    rect
                }
            };
            if let Some(dur) = &d.duration {
                let galley = ui.painter().layout_no_wrap(
                    dur.clone(),
                    theme::font(11.0, Weight::SemiBold),
                    Color32::WHITE,
                );
                let badge = Rect::from_min_size(
                    thumb.right_bottom() - vec2(galley.size().x + 22.0, 30.0),
                    vec2(galley.size().x + 12.0, 20.0),
                );
                ui.painter().rect_filled(
                    badge,
                    CornerRadius::same(6),
                    Color32::from_black_alpha(190),
                );
                ui.painter()
                    .galley(badge.center() - galley.size() / 2.0, galley, Color32::WHITE);
            }

            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&d.title)
                            .font(theme::font(16.0, Weight::SemiBold))
                            .color(p.text),
                    )
                    .wrap(),
                );
                if let Some(ch) = &d.channel {
                    w::text(ui, ch, 12.0, Weight::Regular, p.faint);
                }
            });
            if let Some(f) = d.fraction {
                w::progress_bar(ui, f);
            }
            for pair in d.facts.chunks(2) {
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
                w::text(ui, d.path_label, 12.0, Weight::Regular, p.faint);
                w::mono_box(ui, &d.path);
            });
            ui.horizontal_wrapped(|ui| {
                let id = d.id;
                match d.stage {
                    Stage::Working => {
                        if w::button(ui, Kind::Secondary, Some(Icon::Copy), "Copy command", 40.0)
                            .clicked()
                        {
                            actions.push(Action::CopyCommand(id));
                        }
                        if w::button(ui, Kind::Danger, None, "Cancel", 40.0).clicked() {
                            actions.push(Action::Cancel(id));
                        }
                    }
                    Stage::Done => {
                        if w::button(ui, Kind::Secondary, None, "Open", 40.0).clicked() {
                            actions.push(Action::Open(id));
                        }
                        if w::button(
                            ui,
                            Kind::Secondary,
                            Some(Icon::Folder),
                            "Show in folder",
                            40.0,
                        )
                        .clicked()
                        {
                            actions.push(Action::Reveal(id));
                        }
                    }
                    Stage::Stopped => {
                        if w::button(ui, Kind::Secondary, None, "Retry", 40.0).clicked() {
                            actions.push(Action::Retry(id));
                        }
                        if w::button(ui, Kind::Ghost, None, "Remove", 40.0).clicked() {
                            actions.push(Action::Remove(id));
                        }
                    }
                }
                if w::button(ui, Kind::Ghost, None, "Log", 40.0).clicked() {
                    actions.push(Action::ShowLog(id));
                }
            });
        });
}
