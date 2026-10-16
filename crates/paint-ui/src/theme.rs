//! PaintRS look: own palette (not the Windows one), flat fills, small corner radii.

use egui::{Color32, CornerRadius, Stroke, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
    System,
}

/// Color tokens used by custom-drawn widgets. Egui's own visuals are derived from these.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub dark: bool,
    /// Menu bar and status bar.
    pub chrome: Color32,
    pub ribbon: Color32,
    /// Area around the canvas.
    pub workspace: Color32,
    /// Popups, dialogs, floating panels.
    pub panel: Color32,
    pub separator: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub text_disabled: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub selected: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub canvas_frame: Color32,
    pub warning_bg: Color32,
    pub warning_text: Color32,
}

impl Palette {
    pub const DARK: Palette = Palette {
        dark: true,
        chrome: Color32::from_rgb(0x1B, 0x1C, 0x1F),
        ribbon: Color32::from_rgb(0x24, 0x25, 0x29),
        workspace: Color32::from_rgb(0x13, 0x14, 0x16),
        panel: Color32::from_rgb(0x2A, 0x2B, 0x30),
        separator: Color32::from_rgb(0x36, 0x37, 0x3D),
        border: Color32::from_rgb(0x43, 0x44, 0x4B),
        text: Color32::from_rgb(0xEC, 0xEA, 0xE6),
        text_muted: Color32::from_rgb(0xA3, 0xA0, 0x9B),
        text_disabled: Color32::from_rgb(0x66, 0x65, 0x62),
        hover: Color32::from_rgb(0x33, 0x34, 0x3A),
        pressed: Color32::from_rgb(0x3C, 0x3D, 0x44),
        selected: Color32::from_rgb(0x3A, 0x31, 0x2B),
        accent: Color32::from_rgb(0xE5, 0x8A, 0x5C),
        on_accent: Color32::from_rgb(0x1B, 0x14, 0x10),
        canvas_frame: Color32::from_rgb(0x3E, 0x3F, 0x45),
        warning_bg: Color32::from_rgb(0x4A, 0x3B, 0x16),
        warning_text: Color32::from_rgb(0xF5, 0xE3, 0xB0),
    };

    pub const LIGHT: Palette = Palette {
        dark: false,
        chrome: Color32::from_rgb(0xF1, 0xEF, 0xEB),
        ribbon: Color32::from_rgb(0xFA, 0xF9, 0xF6),
        workspace: Color32::from_rgb(0xD9, 0xD6, 0xD0),
        panel: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        separator: Color32::from_rgb(0xDD, 0xDA, 0xD4),
        border: Color32::from_rgb(0xC9, 0xC5, 0xBE),
        text: Color32::from_rgb(0x1E, 0x1D, 0x1B),
        text_muted: Color32::from_rgb(0x67, 0x64, 0x5F),
        text_disabled: Color32::from_rgb(0xA9, 0xA5, 0x9F),
        hover: Color32::from_rgb(0xEA, 0xE7, 0xE1),
        pressed: Color32::from_rgb(0xDF, 0xDB, 0xD4),
        selected: Color32::from_rgb(0xF6, 0xE4, 0xD8),
        accent: Color32::from_rgb(0xC2, 0x5E, 0x2B),
        on_accent: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        canvas_frame: Color32::from_rgb(0xB9, 0xB5, 0xAE),
        warning_bg: Color32::from_rgb(0xFB, 0xEB, 0xC2),
        warning_text: Color32::from_rgb(0x4A, 0x37, 0x08),
    };

    pub fn for_dark(dark: bool) -> &'static Palette {
        if dark {
            &Self::DARK
        } else {
            &Self::LIGHT
        }
    }
}

/// Small radii: the brief asks for "не сильные закругления".
pub const RADIUS: u8 = 4;
pub const RADIUS_POPUP: u8 = 6;

/// Applies the palette to egui's built-in widgets.
pub fn apply(ctx: &egui::Context, p: &Palette) {
    let mut v = if p.dark { Visuals::dark() } else { Visuals::light() };
    let r = CornerRadius::same(RADIUS);
    v.panel_fill = p.chrome;
    v.window_fill = p.panel;
    v.extreme_bg_color = if p.dark { Color32::from_rgb(0x18, 0x19, 0x1C) } else { Color32::WHITE };
    v.faint_bg_color = p.hover;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(RADIUS_POPUP);
    v.menu_corner_radius = CornerRadius::same(RADIUS_POPUP);
    v.selection.bg_fill = p.accent.gamma_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.accent;

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.panel;
    w.noninteractive.weak_bg_fill = p.panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.separator);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = r;

    w.inactive.bg_fill = p.hover;
    w.inactive.weak_bg_fill = Color32::TRANSPARENT;
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.inactive.corner_radius = r;

    w.hovered.bg_fill = p.hover;
    w.hovered.weak_bg_fill = p.hover;
    w.hovered.bg_stroke = Stroke::new(1.0, p.border);
    w.hovered.fg_stroke = Stroke::new(1.0, p.text);
    w.hovered.corner_radius = r;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = p.pressed;
    w.active.weak_bg_fill = p.pressed;
    w.active.bg_stroke = Stroke::new(1.0, p.accent);
    w.active.fg_stroke = Stroke::new(1.0, p.text);
    w.active.corner_radius = r;
    w.active.expansion = 0.0;

    w.open.bg_fill = p.pressed;
    w.open.weak_bg_fill = p.pressed;
    w.open.bg_stroke = Stroke::new(1.0, p.border);
    w.open.fg_stroke = Stroke::new(1.0, p.text);
    w.open.corner_radius = r;

    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.button_padding = egui::vec2(8.0, 4.0);
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.menu_margin = egui::Margin::same(4);
        s.spacing.interact_size.y = 24.0;
        s.animation_time = 0.08;
    });
}
