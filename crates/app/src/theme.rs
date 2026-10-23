//! Colors, fonts and spacing for the dark and light themes.
//!
//! Widgets never hard-code colors: they ask [`palette`] for the active one,
//! so switching theme repaints everything correctly.

use std::sync::Arc;

use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, Theme,
    ThemePreference, Visuals,
};
use ytdl_core::config::Theme as ThemeSetting;

pub struct Palette {
    pub bg: Color32,
    pub surface: Color32,
    pub surface_hover: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub on_accent: Color32,
    pub segment_bg: Color32,
    pub selected_border: Color32,
    pub selected_glow: Color32,
    pub track: Color32,
    pub success: Color32,
    pub success_bg: Color32,
    pub warn_bg: Color32,
    pub warn_border: Color32,
    pub warn_icon_bg: Color32,
    pub warn_icon: Color32,
    pub warn_text: Color32,
    pub danger: Color32,
}

const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

pub const DARK: Palette = Palette {
    bg: hex(0x1F2125),
    surface: hex(0x282A2F),
    surface_hover: hex(0x31343A),
    border: hex(0x3A3D44),
    border_strong: hex(0x4B4F57),
    text: hex(0xEDEDEF),
    muted: hex(0xA1A1AA),
    faint: hex(0x9C9CA6),
    accent: hex(0xD32F2F),
    accent_hover: hex(0xB71C1C),
    on_accent: hex(0xFFFFFF),
    segment_bg: hex(0x31343A),
    selected_border: hex(0x9A2A2A),
    selected_glow: Color32::from_rgba_premultiplied(60, 10, 10, 60),
    track: hex(0x3A3D44),
    success: hex(0x4ADE80),
    success_bg: hex(0x10301C),
    warn_bg: hex(0x2E2617),
    warn_border: hex(0x4A3818),
    warn_icon_bg: hex(0x3D2E12),
    warn_icon: hex(0xF5B544),
    warn_text: hex(0xE8C88A),
    danger: hex(0xFF8A8A),
};

pub const LIGHT: Palette = Palette {
    bg: hex(0xF5F4F0),
    surface: hex(0xFFFFFF),
    surface_hover: hex(0xEFEDE8),
    border: hex(0xE6E4DE),
    border_strong: hex(0xD3D0C7),
    text: hex(0x18181B),
    muted: hex(0x5F5E66),
    faint: hex(0x67666E),
    accent: hex(0xD32F2F),
    accent_hover: hex(0xB71C1C),
    on_accent: hex(0xFFFFFF),
    segment_bg: hex(0xEAE8E2),
    selected_border: hex(0xF3C2C2),
    selected_glow: hex(0xFDECEC),
    track: hex(0xECEAE4),
    success: hex(0x17803D),
    success_bg: hex(0xE9F7EE),
    warn_bg: hex(0xFFF8EC),
    warn_border: hex(0xF2DDB4),
    warn_icon_bg: hex(0xFCEBCB),
    warn_icon: hex(0x8A5300),
    warn_text: hex(0x6B4A12),
    danger: hex(0xA32D2D),
};

pub fn palette(ctx: &egui::Context) -> &'static Palette {
    match ctx.theme() {
        Theme::Dark => &DARK,
        Theme::Light => &LIGHT,
    }
}

/// Corner radii used across the app.
pub mod radius {
    pub const SMALL: u8 = 8;
    pub const CONTROL: u8 = 10;
    pub const CARD: u8 = 14;
    pub const PANEL: u8 = 18;
}

#[derive(Clone, Copy)]
pub enum Weight {
    Regular,
    Medium,
    SemiBold,
}

impl Weight {
    fn family(self) -> FontFamily {
        match self {
            Weight::Regular => FontFamily::Proportional,
            Weight::Medium => FontFamily::Name("medium".into()),
            Weight::SemiBold => FontFamily::Name("semibold".into()),
        }
    }
}

/// `Stroke::new` takes `impl Into<f32>`, so plain float literals are ambiguous.
pub fn stroke(width: f32, color: Color32) -> Stroke {
    Stroke::new(width, color)
}

pub fn font(size: f32, weight: Weight) -> FontId {
    FontId::new(size, weight.family())
}

pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

/// Sets fonts, both themes' visuals and spacing. Call once at startup.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    ctx.set_visuals_of(Theme::Dark, visuals(&DARK, true));
    ctx.set_visuals_of(Theme::Light, visuals(&LIGHT, false));
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.interact_size.y = 36.0;
        style
            .text_styles
            .insert(egui::TextStyle::Body, font(14.0, Weight::Regular));
        style
            .text_styles
            .insert(egui::TextStyle::Button, font(13.0, Weight::Medium));
        style
            .text_styles
            .insert(egui::TextStyle::Small, font(12.0, Weight::Regular));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, font(20.0, Weight::SemiBold));
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, mono(12.0));
    });
}

pub fn apply_setting(ctx: &egui::Context, setting: ThemeSetting) {
    ctx.set_theme(match setting {
        ThemeSetting::System => ThemePreference::System,
        ThemeSetting::Dark => ThemePreference::Dark,
        ThemeSetting::Light => ThemePreference::Light,
    });
}

fn fonts() -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    let mut add = |name: &str, bytes: &'static [u8]| {
        defs.font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    };
    add("geist", include_bytes!("../fonts/Geist-Regular.ttf"));
    add("geist-medium", include_bytes!("../fonts/Geist-Medium.ttf"));
    add(
        "geist-semibold",
        include_bytes!("../fonts/Geist-SemiBold.ttf"),
    );
    add(
        "geist-mono",
        include_bytes!("../fonts/GeistMono-Regular.ttf"),
    );

    // egui's bundled fonts stay as fallbacks for Greek, symbols and emoji in titles.
    let fallbacks = defs.families[&FontFamily::Proportional].clone();
    let with_fallbacks = |first: &str| {
        let mut list = vec![first.to_owned()];
        list.extend(fallbacks.iter().cloned());
        list
    };
    defs.families
        .insert(FontFamily::Proportional, with_fallbacks("geist"));
    for (family, font) in [("medium", "geist-medium"), ("semibold", "geist-semibold")] {
        defs.families
            .insert(FontFamily::Name(family.into()), with_fallbacks(font));
    }
    defs.families
        .get_mut(&FontFamily::Monospace)
        .expect("egui defines monospace")
        .insert(0, "geist-mono".to_owned());
    defs
}

fn visuals(p: &Palette, dark: bool) -> Visuals {
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.panel_fill = p.bg;
    v.window_fill = p.surface;
    v.window_stroke = stroke(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(radius::CARD);
    v.menu_corner_radius = CornerRadius::same(radius::CONTROL);
    v.extreme_bg_color = p.surface;
    v.faint_bg_color = p.surface_hover;
    v.code_bg_color = p.surface_hover;
    // Red links on dark gray need a lighter shade to stay readable.
    v.hyperlink_color = if dark { hex(0xFF8A80) } else { p.accent_hover };
    v.selection.bg_fill = p.accent.gamma_multiply(0.45);
    v.selection.stroke = stroke(1.0, p.text);
    v.override_text_color = Some(p.text);

    let w = &mut v.widgets;
    let r = CornerRadius::same(radius::CONTROL);
    w.noninteractive.bg_fill = p.surface;
    w.noninteractive.weak_bg_fill = p.surface;
    w.noninteractive.bg_stroke = stroke(1.0, p.border);
    w.noninteractive.fg_stroke = stroke(1.0, p.text);
    w.noninteractive.corner_radius = r;
    for (state, fill, border) in [
        (&mut w.inactive, p.surface, p.border),
        (&mut w.hovered, p.surface_hover, p.border_strong),
        (&mut w.active, p.surface_hover, p.border_strong),
        (&mut w.open, p.surface_hover, p.border_strong),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = stroke(1.0, border);
        state.fg_stroke = stroke(1.5, p.text);
        state.corner_radius = r;
        state.expansion = 0.0;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG relative luminance.
    fn luminance(c: Color32) -> f64 {
        let ch = |v: u8| {
            let v = v as f64 / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Body text must reach 4.5:1 (WCAG AA) on every surface it sits on.
    #[test]
    fn text_is_readable_in_both_themes() {
        for p in [&DARK, &LIGHT] {
            for bg in [p.bg, p.surface, p.surface_hover] {
                for fg in [p.text, p.muted, p.faint] {
                    let c = contrast(fg, bg);
                    assert!(c >= 4.5, "{fg:?} on {bg:?} is only {c:.2}:1");
                }
            }
            // Selected chips, the Audio/Video switch and buttons all use the accent.
            assert!(contrast(p.on_accent, p.accent) >= 4.5);
            assert!(contrast(p.on_accent, p.accent_hover) >= 4.5);
            assert!(contrast(p.success, p.success_bg) >= 4.5);
            assert!(contrast(p.warn_text, p.warn_bg) >= 4.5);
            assert!(contrast(p.danger, p.surface) >= 4.5);
        }
    }
}
