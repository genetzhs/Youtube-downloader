//! The app's own widgets, drawn to match the design. All colors come from
//! [`theme::palette`], so they follow the dark and light themes.

use std::f32::consts::{FRAC_PI_2, TAU};

use egui::{
    Color32, CornerRadius, CursorIcon, Galley, Rect, Response, Sense, Shape, Stroke, StrokeKind,
    Ui, Vec2, WidgetInfo, WidgetType, vec2,
};

use crate::icons::{self, Icon};
use crate::theme::{self, Weight, font, palette, radius, stroke};

fn galley(
    ui: &Ui,
    text: &str,
    size: f32,
    weight: Weight,
    color: Color32,
) -> std::sync::Arc<Galley> {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font(size, weight), color)
}

/// A pill-shaped choice, e.g. `1080p` or `MP3`.
pub fn chip(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let p = palette(ui.ctx());
    let fg = if selected { p.chip_selected_fg } else { p.text };
    let text = galley(ui, label, 13.0, Weight::Medium, fg);
    let size = vec2(text.size().x + 28.0, 36.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, label));
    if ui.is_rect_visible(rect) {
        let r = CornerRadius::same(radius::CONTROL);
        if selected {
            ui.painter().rect_filled(rect, r, p.chip_selected_bg);
        } else {
            let fill = if resp.hovered() {
                p.surface_hover
            } else {
                p.surface
            };
            let border = if resp.hovered() {
                p.border_strong
            } else {
                p.border
            };
            ui.painter().rect_filled(rect, r, fill);
            ui.painter()
                .rect_stroke(rect, r, stroke(1.0, border), StrokeKind::Inside);
        }
        let pos = rect.center() - text.size() / 2.0;
        ui.painter().galley(pos, text, fg);
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// A small muted label in front of a group of chips.
pub fn chip_label(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    ui.label(
        egui::RichText::new(text)
            .font(font(12.0, Weight::Regular))
            .color(p.faint),
    );
}

/// A thin vertical divider between chip groups.
pub fn divider(ui: &mut Ui) {
    let p = palette(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(13.0, 22.0), Sense::hover());
    ui.painter().vline(
        rect.center().x,
        rect.y_range(),
        stroke(1.0, p.border_strong),
    );
}

/// The Audio | Video switch. Returns true when the value changed.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, Icon, &str)],
) -> bool {
    let p = palette(ui.ctx());
    let labels: Vec<_> = options
        .iter()
        .map(|(v, _, l)| {
            let color = if v == value { p.text } else { p.muted };
            galley(ui, l, 13.0, Weight::SemiBold, color)
        })
        .collect();
    let seg_w: Vec<f32> = labels
        .iter()
        .map(|g| g.size().x + 18.0 + 8.0 + 36.0)
        .collect();
    let pad = 4.0;
    let total = vec2(seg_w.iter().sum::<f32>() + pad * 2.0, 44.0);
    let (outer, _) = ui.allocate_exact_size(total, Sense::hover());
    ui.painter()
        .rect_filled(outer, CornerRadius::same(12), p.segment_bg);

    let mut changed = false;
    let mut x = outer.left() + pad;
    for (i, ((v, icon, label), text)) in options.iter().zip(labels).enumerate() {
        let rect = Rect::from_min_size(egui::pos2(x, outer.top() + pad), vec2(seg_w[i], 36.0));
        x += seg_w[i];
        let id = ui.id().with(("segment", i));
        let resp = ui.interact(rect, id, Sense::click());
        let selected = v == value;
        resp.widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, true, selected, *label));
        if selected {
            let r = CornerRadius::same(9);
            // A one-pixel shadow under the selected segment.
            ui.painter().rect_filled(
                rect.translate(vec2(0.0, 1.0)),
                r,
                Color32::from_black_alpha(18),
            );
            ui.painter().rect_filled(rect, r, p.segment_selected);
        } else if resp.hovered() {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(9), p.surface_hover);
        }
        let color = if selected { p.text } else { p.muted };
        let content_w = 15.0 + 8.0 + text.size().x;
        let left = rect.center().x - content_w / 2.0;
        icons::paint(
            ui.painter(),
            Rect::from_center_size(egui::pos2(left + 7.5, rect.center().y), Vec2::splat(15.0)),
            *icon,
            color,
        );
        ui.painter().galley(
            egui::pos2(left + 23.0, rect.center().y - text.size().y / 2.0),
            text,
            color,
        );
        if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() && !selected {
            *value = *v;
            changed = true;
        }
    }
    changed
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Secondary,
    /// Secondary with red text, for Cancel.
    Danger,
    /// Warning-colored, for actions on a warning card.
    Warning,
    /// No background until hovered.
    Ghost,
}

/// A text button with an optional leading icon.
pub fn button(ui: &mut Ui, kind: Kind, icon: Option<Icon>, label: &str, height: f32) -> Response {
    let p = palette(ui.ctx());
    let fg = match kind {
        Kind::Primary => p.on_accent,
        Kind::Danger => p.danger,
        Kind::Warning => p.warn_text,
        Kind::Secondary => p.text,
        Kind::Ghost => p.muted,
    };
    let (size_px, weight) = if kind == Kind::Primary && height >= 44.0 {
        (15.0, Weight::SemiBold)
    } else {
        (13.0, Weight::Medium)
    };
    let text = galley(ui, label, size_px, weight, fg);
    let icon_w = if icon.is_some() { 16.0 + 8.0 } else { 0.0 };
    let pad_x = if kind == Kind::Primary { 22.0 } else { 14.0 };
    let size = vec2(text.size().x + icon_w + pad_x * 2.0, height);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    if ui.is_rect_visible(rect) {
        let r = CornerRadius::same(if height >= 44.0 { 12 } else { radius::CONTROL });
        let hovered = resp.hovered();
        match kind {
            Kind::Primary => {
                let fill = if hovered { p.accent_hover } else { p.accent };
                ui.painter().rect_filled(rect, r, fill);
            }
            Kind::Ghost => {
                if hovered {
                    ui.painter().rect_filled(rect, r, p.surface_hover);
                }
            }
            _ => {
                let fill = if hovered { p.surface_hover } else { p.surface };
                let border = match kind {
                    Kind::Warning => p.warn_border,
                    _ if hovered => p.border_strong,
                    _ => p.border,
                };
                ui.painter().rect_filled(rect, r, fill);
                ui.painter()
                    .rect_stroke(rect, r, stroke(1.0, border), StrokeKind::Inside);
            }
        }
        let content_w = icon_w + text.size().x;
        let left = rect.center().x - content_w / 2.0;
        if let Some(icon) = icon {
            icons::paint(
                ui.painter(),
                Rect::from_center_size(egui::pos2(left + 8.0, rect.center().y), Vec2::splat(16.0)),
                icon,
                fg,
            );
        }
        ui.painter().galley(
            egui::pos2(left + icon_w, rect.center().y - text.size().y / 2.0),
            text,
            fg,
        );
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// A square icon-only button; `label` is its tooltip and accessible name.
pub fn icon_button(ui: &mut Ui, icon: Icon, label: &str) -> Response {
    let p = palette(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    let color = if resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius::CONTROL), p.surface_hover);
        p.text
    } else {
        p.muted
    };
    icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(19.0)),
        icon,
        color,
    );
    resp.on_hover_text(label)
        .on_hover_cursor(CursorIcon::PointingHand)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ring {
    /// 0.0..=1.0
    Progress(f32),
    /// Working, but how far along is unknown (starting, converting).
    Spinner,
    /// Dashed outline: waiting for a free slot.
    Waiting,
}

/// Circular progress indicator at the start of queue rows.
pub fn progress_ring(ui: &mut Ui, ring: Ring, diameter: f32) -> Response {
    let p = palette(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(diameter), Sense::hover());
    let c = rect.center();
    let width = (diameter / 11.0).round();
    let r = diameter / 2.0 - width / 2.0;
    let arc = |from: f32, sweep: f32| {
        let steps = (sweep / TAU * 64.0).ceil().max(2.0) as usize;
        let pts: Vec<_> = (0..=steps)
            .map(|i| {
                let a = from + sweep * i as f32 / steps as f32;
                c + vec2(a.cos(), a.sin()) * r
            })
            .collect();
        ui.painter()
            .add(Shape::line(pts, Stroke::new(width, p.accent)));
        // Round caps.
        for end in [from, from + sweep] {
            ui.painter()
                .circle_filled(c + vec2(end.cos(), end.sin()) * r, width / 2.0, p.accent);
        }
    };
    match ring {
        Ring::Progress(f) => {
            ui.painter()
                .circle_stroke(c, r, Stroke::new(width, p.track));
            let f = f.clamp(0.0, 1.0);
            if f > 0.0 {
                arc(-FRAC_PI_2, TAU * f);
            }
        }
        Ring::Spinner => {
            ui.painter()
                .circle_stroke(c, r, Stroke::new(width, p.track));
            let t = ui.input(|i| i.time) as f32;
            arc(t * TAU * 0.8, TAU * 0.28);
            ui.ctx().request_repaint();
        }
        Ring::Waiting => {
            let dashes = 18;
            for i in 0..dashes {
                let a0 = TAU * i as f32 / dashes as f32;
                let a1 = a0 + TAU / dashes as f32 * 0.5;
                ui.painter().line_segment(
                    [
                        c + vec2(a0.cos(), a0.sin()) * r,
                        c + vec2(a1.cos(), a1.sin()) * r,
                    ],
                    stroke(2.0, p.border_strong),
                );
            }
        }
    }
    resp
}

pub enum Badge {
    Done,
    Warning,
    Cancelled,
}

/// The round status mark at the start of finished or failed rows.
pub fn badge(ui: &mut Ui, badge: Badge, diameter: f32) {
    let p = palette(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(diameter), Sense::hover());
    let (bg, fg, icon) = match badge {
        Badge::Done => (p.success_bg, p.success, Icon::Check),
        Badge::Warning => (p.warn_icon_bg, p.warn_icon, Icon::Warning),
        Badge::Cancelled => (p.surface_hover, p.muted, Icon::Close),
    };
    ui.painter()
        .circle_filled(rect.center(), diameter / 2.0, bg);
    icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(diameter * 0.45)),
        icon,
        fg,
    );
}

/// A thin horizontal progress bar filling the available width.
pub fn progress_bar(ui: &mut Ui, fraction: f32) {
    let p = palette(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
    let r = CornerRadius::same(3);
    ui.painter().rect_filled(rect, r, p.track);
    let mut fill = rect;
    fill.set_width(rect.width() * fraction.clamp(0.0, 1.0));
    ui.painter().rect_filled(fill, r, p.accent);
}

/// A colored dot followed by a short status text.
pub fn status_dot(ui: &mut Ui, ok: bool, text: &str) {
    let p = palette(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(7.0, 14.0), Sense::hover());
    let color = if ok {
        Color32::from_rgb(0x22, 0xA3, 0x5A)
    } else {
        p.warn_icon
    };
    ui.painter().circle_filled(rect.center(), 3.5, color);
    ui.label(
        egui::RichText::new(text)
            .font(font(12.0, Weight::Regular))
            .color(p.muted),
    );
}

pub struct LinkField {
    /// Enter was pressed or Download was clicked.
    pub submitted: bool,
}

/// The large link field with the Download button.
pub fn link_field(ui: &mut Ui, text: &mut String, paste_hint: &str) -> LinkField {
    let p = palette(ui.ctx());
    let mut submitted = false;
    let frame = egui::Frame::new()
        .fill(p.surface)
        .stroke(stroke(1.0, p.border))
        .corner_radius(CornerRadius::same(radius::PANEL))
        .inner_margin(egui::Margin {
            left: 18,
            right: 8,
            top: 8,
            bottom: 8,
        })
        .shadow(egui::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(if ui.visuals().dark_mode { 70 } else { 14 }),
        });
    frame.show(ui, |ui| {
        let row = vec2(ui.available_width(), 48.0);
        ui.allocate_ui_with_layout(
            row,
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                icons::paint(ui.painter(), icon_rect, Icon::Link, p.faint);

                let button_w = 150.0;
                let hint = galley(ui, paste_hint, 12.0, Weight::Regular, p.faint);
                let field_w = (ui.available_width() - button_w - hint.size().x - 24.0).max(120.0);
                let edit = egui::TextEdit::singleline(text)
                    .frame(false)
                    .font(font(16.0, Weight::Regular))
                    .text_color(p.text)
                    .hint_text(
                        egui::RichText::new("Paste a YouTube or any yt-dlp supported link…")
                            .color(p.faint),
                    )
                    .desired_width(field_w)
                    .vertical_align(egui::Align::Center)
                    .min_size(vec2(field_w, 48.0));
                let resp = ui.add(edit);
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    submitted = true;
                }
                let (hint_rect, _) = ui.allocate_exact_size(hint.size(), Sense::hover());
                ui.painter().galley(hint_rect.min, hint, p.faint);
                if button(ui, Kind::Primary, Some(Icon::Download), "Download", 48.0).clicked() {
                    submitted = true;
                }
            },
        );
    });
    LinkField { submitted }
}

/// Lays out a horizontal row centered in the available width. egui places
/// widgets left to right, so the row is measured once and shifted on the
/// next frame (a repaint is requested when its width changes).
pub fn centered_row(ui: &mut Ui, id: &str, add: impl FnOnce(&mut Ui)) {
    let id = ui.id().with(id);
    let last_w: f32 = ui.data(|d| d.get_temp(id)).unwrap_or(0.0);
    let avail = ui.available_width();
    ui.horizontal(|ui| {
        let offset = ((avail - last_w) / 2.0).max(0.0);
        ui.add_space(offset);
        let start = ui.cursor().left();
        add(ui);
        let w = ui.min_rect().right() - start;
        if (w - last_w).abs() > 0.5 {
            ui.data_mut(|d| d.insert_temp(id, w));
            ui.ctx().request_repaint();
        }
    });
}

/// The rounded card used for queue rows and side panels.
pub fn card(fill: Color32, border: Color32, radius_px: u8) -> egui::Frame {
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke(1.0, border))
        .corner_radius(CornerRadius::same(radius_px))
        .inner_margin(egui::Margin::symmetric(14, 12))
}

/// Text helpers so call sites stay short.
pub fn text(ui: &mut Ui, s: &str, size: f32, weight: Weight, color: Color32) -> Response {
    ui.label(egui::RichText::new(s).font(font(size, weight)).color(color))
}

pub fn mono_box(ui: &mut Ui, s: &str) {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .fill(p.bg)
        .corner_radius(CornerRadius::same(radius::SMALL))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    egui::RichText::new(s)
                        .font(theme::mono(12.0))
                        .color(p.muted),
                )
                .truncate(),
            );
        });
}

/// An on/off switch. Returns its response; `value` is flipped on click.
pub fn switch(ui: &mut Ui, value: &mut bool, label: &str) -> Response {
    let p = palette(ui.ctx());
    let (rect, mut resp) = ui.allocate_exact_size(vec2(40.0, 22.0), Sense::click());
    if resp.clicked() {
        *value = !*value;
        resp.mark_changed();
    }
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, *value, label));
    let t = ui.ctx().animate_bool_responsive(resp.id, *value);
    let track = if *value { p.accent } else { p.border_strong };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(11), track);
    let x = egui::lerp(rect.left() + 11.0..=rect.right() - 11.0, t);
    ui.painter()
        .circle_filled(egui::pos2(x, rect.center().y), 8.0, Color32::WHITE);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// One line of a settings card: title and description on the left, the
/// control on the right. `first` skips the divider above it.
pub fn setting_row(
    ui: &mut Ui,
    first: bool,
    title: &str,
    description: &str,
    control: impl FnOnce(&mut Ui),
) {
    let p = palette(ui.ctx());
    if !first {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter()
            .hline(r.x_range(), r.center().y, stroke(1.0, p.border));
    }
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.allocate_ui_with_layout(
            vec2(260.0_f32.min(ui.available_width()), 40.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                text(ui, title, 14.0, Weight::Medium, p.text);
                if !description.is_empty() {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(description)
                                .font(font(12.0, Weight::Regular))
                                .color(p.faint),
                        )
                        .wrap(),
                    );
                }
            },
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), control);
    });
    ui.add_space(6.0);
}

/// A titled card for a group of settings.
pub fn section(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    let p = palette(ui.ctx());
    card(p.surface, p.border, radius::CARD)
        .inner_margin(egui::Margin::symmetric(20, 14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            text(ui, &title.to_uppercase(), 12.0, Weight::SemiBold, p.muted);
            ui.add_space(4.0);
            add(ui);
        });
}

/// A single-line text field styled like the rest of the app.
pub fn text_field(ui: &mut Ui, value: &mut String, width: f32, mono_font: bool) -> Response {
    let p = palette(ui.ctx());
    let f = if mono_font {
        theme::mono(13.0)
    } else {
        font(13.0, Weight::Regular)
    };
    egui::Frame::new()
        .fill(p.bg)
        .stroke(stroke(1.0, p.border))
        .corner_radius(CornerRadius::same(radius::CONTROL))
        .inner_margin(egui::Margin::symmetric(10, 9))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(value)
                    .frame(false)
                    .font(f)
                    .text_color(p.text)
                    .desired_width(width),
            )
        })
        .inner
}
