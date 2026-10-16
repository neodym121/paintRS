//! Small custom-drawn widgets shared by the ribbon, menu bar and status bar.

use egui::{Align2, Color32, CornerRadius, Id, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::fonts;
use crate::theme::{Palette, RADIUS};

pub fn palette(ui: &Ui) -> &'static Palette {
    Palette::for_dark(ui.visuals().dark_mode)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BtnState {
    pub selected: bool,
    pub disabled: bool,
}

impl BtnState {
    pub fn selected(selected: bool) -> Self {
        Self { selected, disabled: false }
    }

    pub fn enabled(enabled: bool) -> Self {
        Self { selected: false, disabled: !enabled }
    }
}

/// Background for a flat button: hover / pressed / selected (accent outline), nothing at rest.
pub fn paint_button_bg(ui: &Ui, rect: Rect, resp: &Response, st: BtnState) {
    let p = palette(ui);
    let painter = ui.painter();
    let r = CornerRadius::same(RADIUS);
    if st.selected {
        painter.rect_filled(rect, r, p.selected);
        painter.rect_stroke(rect, r, Stroke::new(1.0, p.accent), StrokeKind::Inside);
    } else if !st.disabled && (resp.is_pointer_button_down_on() || resp.dragged()) {
        painter.rect_filled(rect, r, p.pressed);
    } else if !st.disabled && resp.hovered() {
        painter.rect_filled(rect, r, p.hover);
    }
}

pub fn fg(ui: &Ui, st: BtnState) -> Color32 {
    let p = palette(ui);
    if st.disabled {
        p.text_disabled
    } else {
        p.text
    }
}

/// Square icon button at a fixed rect.
pub fn icon_button(ui: &mut Ui, rect: Rect, id: Id, icon: char, size: f32, st: BtnState) -> Response {
    let sense = if st.disabled { Sense::hover() } else { Sense::click() };
    let resp = ui.interact(rect, id, sense);
    paint_button_bg(ui, rect, &resp, st);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, icon, fonts::icon(size), fg(ui, st));
    resp
}

/// Small "▾" chevron.
pub fn chevron(ui: &Ui, center: Pos2, color: Color32) {
    ui.painter().text(center, Align2::CENTER_CENTER, crate::icons::CHEVRON_DOWN, fonts::icon(11.0), color);
}

pub fn group_label(ui: &Ui, center: Pos2, text: &str) {
    let p = palette(ui);
    ui.painter().text(center, Align2::CENTER_CENTER, text, fonts::regular(12.0), p.text_muted);
}

pub fn vline(ui: &Ui, x: f32, y0: f32, y1: f32, color: Color32) {
    ui.painter().vline(x + 0.5, y0..=y1, Stroke::new(1.0, color));
}

/// Tooltip text with an optional shortcut in parentheses.
pub fn tip(text: &str, shortcut: Option<&str>) -> String {
    match shortcut {
        Some(s) if !s.is_empty() => format!("{text} ({s})"),
        _ => text.to_owned(),
    }
}

/// A rect of `size` centered at `c`.
pub fn centered(c: Pos2, size: Vec2) -> Rect {
    Rect::from_center_size(c, size)
}
