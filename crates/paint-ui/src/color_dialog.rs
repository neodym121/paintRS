//! "Edit colors" dialog (docs/behavior/colors.md): hue/saturation spectrum, value slider, preview,
//! HEX field, RGB/HSV fields, 48 basic colors and 24 custom colors with a "+" button.
//! Layout follows the original; the look and the basic color set are our own.

use egui::{Align2, Color32, Id, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use paint_core::Rgba;

use crate::editor::{ColorSlot, Editor};
use crate::fonts;
use crate::ribbon::to_color32;

const SPECTRUM: Vec2 = Vec2::new(256.0, 256.0);
const SPECTRUM_TEX: usize = 256;

pub struct ColorDialog {
    pub slot: ColorSlot,
    h: f32,
    s: f32,
    v: f32,
    hex: String,
    hsv_mode: bool,
}

impl ColorDialog {
    pub fn new(slot: ColorSlot, c: Rgba) -> Self {
        let (h, s, v) = c.to_hsv();
        Self { slot, h, s, v, hex: c.to_hex(), hsv_mode: false }
    }

    pub fn color(&self) -> Rgba {
        Rgba::from_hsv(self.h, self.s, self.v)
    }

    /// Takes an RGB color; hue/saturation are kept where the color does not define them
    /// (grays and black), so the spectrum marker does not jump.
    fn set_rgb(&mut self, c: Rgba) {
        let (h, s, v) = c.to_hsv();
        if v > 0.0 {
            if s > 0.0 {
                self.h = h;
            }
            self.s = s;
        }
        self.v = v;
        self.sync_hex();
    }

    fn sync_hex(&mut self) {
        self.hex = self.color().to_hex();
    }
}

/// Our own 12 × 4 basic colors: ten hues in four tones, then two neutral columns.
pub fn basic_colors() -> [Rgba; 48] {
    const HUES: [f32; 10] = [0.0, 24.0, 42.0, 58.0, 90.0, 140.0, 178.0, 202.0, 230.0, 285.0];
    const TONES: [(f32, f32); 4] = [(0.32, 1.0), (0.78, 0.95), (0.85, 0.68), (0.85, 0.38)];
    const NEUTRAL: [[Rgba; 2]; 4] = [
        [Rgba::rgb(0xFF, 0xFF, 0xFF), Rgba::rgb(0xE8, 0xE1, 0xD5)],
        [Rgba::rgb(0xC6, 0xC6, 0xC6), Rgba::rgb(0xA8, 0x9C, 0x8B)],
        [Rgba::rgb(0x84, 0x84, 0x84), Rgba::rgb(0x6A, 0x5E, 0x52)],
        [Rgba::rgb(0x44, 0x44, 0x44), Rgba::rgb(0x00, 0x00, 0x00)],
    ];
    let mut out = [Rgba::BLACK; 48];
    for (row, (s, v)) in TONES.into_iter().enumerate() {
        for (col, h) in HUES.into_iter().enumerate() {
            out[row * 12 + col] = Rgba::from_hsv(h, s, v);
        }
        out[row * 12 + 10] = NEUTRAL[row][0];
        out[row * 12 + 11] = NEUTRAL[row][1];
    }
    out
}

fn spectrum_image() -> egui::ColorImage {
    let n = SPECTRUM_TEX;
    let mut pixels = Vec::with_capacity(n * n);
    for y in 0..n {
        for x in 0..n {
            let h = x as f32 / (n - 1) as f32 * 360.0;
            let s = 1.0 - y as f32 / (n - 1) as f32;
            let c = Rgba::from_hsv(h, s, 1.0);
            pixels.push(Color32::from_rgb(c.r, c.g, c.b));
        }
    }
    egui::ColorImage::new([n, n], pixels)
}

/// Circle swatch; returns the response.
fn swatch(
    ui: &mut egui::Ui,
    center: Pos2,
    r: f32,
    id: Id,
    color: Option<Rgba>,
    ring: bool,
) -> egui::Response {
    let p = crate::widgets::palette(ui);
    let rect = Rect::from_center_size(center, Vec2::splat(2.0 * r + 4.0));
    let resp = ui.interact(rect, id, Sense::click());
    let painter = ui.painter();
    if resp.hovered() {
        painter.circle_filled(center, r + 3.0, p.hover);
    }
    match color {
        Some(c) => {
            painter.circle_filled(center, r, to_color32(c));
            painter.circle_stroke(center, r, Stroke::new(1.0, p.border));
        }
        None => {
            painter.circle_stroke(center, r - 0.5, Stroke::new(1.0, p.border));
        }
    }
    if ring {
        painter.circle_stroke(center, r + 2.5, Stroke::new(1.5, p.text));
    }
    resp
}

impl Editor {
    pub(crate) fn open_color_dialog(&mut self) {
        let slot = self.active_slot;
        self.color_dialog = Some(ColorDialog::new(slot, self.active_color()));
    }

    /// Adds a color to the custom row of the palette: no duplicates; when the row is full the
    /// oldest color goes away (observed in the original).
    pub(crate) fn push_custom_color(&mut self, c: Rgba) {
        if self.custom_colors.contains(&Some(c)) {
            return;
        }
        if let Some(slot) = self.custom_colors.iter_mut().find(|s| s.is_none()) {
            *slot = Some(c);
        } else {
            self.custom_colors.rotate_left(1);
            if let Some(last) = self.custom_colors.last_mut() {
                *last = Some(c);
            }
        }
    }

    pub(crate) fn color_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(mut dlg) = self.color_dialog.take() else { return };
        let tr = self.tr();
        let p = self.palette(ctx);
        let tex = self
            .spectrum
            .get_or_insert_with(|| {
                ctx.load_texture("color_spectrum", spectrum_image(), egui::TextureOptions::LINEAR)
            })
            .id();
        let frame = egui::Frame::popup(&ctx.global_style())
            .fill(p.panel)
            .inner_margin(egui::Margin::same(24))
            .corner_radius(crate::theme::RADIUS_POPUP);
        let mut ok = false;
        let mut cancel = false;
        let basics = basic_colors();
        let modal = egui::Modal::new(Id::unique("edit_colors")).frame(frame).show(ctx, |ui| {
            let size = Vec2::new(600.0, 520.0);
            let (area, _) = ui.allocate_exact_size(size, Sense::hover());
            let o = area.min;
            let at = |x: f32, y: f32| Pos2::new(o.x + x, o.y + y);
            ui.painter().text(at(0.0, 0.0), Align2::LEFT_TOP, tr.edit_colors, fonts::semibold(18.0), p.text);

            // Spectrum: hue left → right, saturation top (full) → bottom (white).
            let spec = Rect::from_min_size(at(0.0, 40.0), SPECTRUM);
            let resp = ui.interact(spec, Id::unique("cd_spectrum"), Sense::click_and_drag());
            if let Some(pos) = resp
                .interact_pointer_pos()
                .filter(|_| resp.is_pointer_button_down_on() || resp.dragged() || resp.clicked())
            {
                dlg.h = ((pos.x - spec.min.x) / spec.width()).clamp(0.0, 1.0) * 360.0;
                dlg.s = 1.0 - ((pos.y - spec.min.y) / spec.height()).clamp(0.0, 1.0);
                if dlg.v == 0.0 {
                    dlg.v = 1.0;
                }
                dlg.sync_hex();
            }
            let painter = ui.painter();
            painter.image(tex, spec, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            painter.rect_stroke(spec, 0.0, Stroke::new(1.0, p.border), StrokeKind::Outside);
            let marker = Pos2::new(
                spec.min.x + dlg.h / 360.0 * spec.width(),
                spec.min.y + (1.0 - dlg.s) * spec.height(),
            );
            painter.with_clip_rect(spec.expand(8.0)).circle_stroke(
                marker,
                6.0,
                Stroke::new(2.0, Color32::BLACK),
            );
            painter.with_clip_rect(spec.expand(8.0)).circle_stroke(
                marker,
                4.5,
                Stroke::new(1.5, Color32::WHITE),
            );

            // Preview of the chosen color.
            let preview = Rect::from_min_size(at(270.0, 40.0), Vec2::new(44.0, SPECTRUM.y));
            painter.rect_filled(preview, 4.0, to_color32(dlg.color()));
            painter.rect_stroke(preview, 4.0, Stroke::new(1.0, p.border), StrokeKind::Inside);

            // Value slider: full brightness at the top, black at the bottom.
            let bar = Rect::from_min_size(at(330.0, 40.0), Vec2::new(14.0, SPECTRUM.y));
            let resp = ui.interact(
                bar.expand2(Vec2::new(8.0, 6.0)),
                Id::unique("cd_value"),
                Sense::click_and_drag(),
            );
            if let Some(pos) = resp
                .interact_pointer_pos()
                .filter(|_| resp.is_pointer_button_down_on() || resp.dragged() || resp.clicked())
            {
                dlg.v = 1.0 - ((pos.y - bar.min.y) / bar.height()).clamp(0.0, 1.0);
                dlg.sync_hex();
            }
            let top = Rgba::from_hsv(dlg.h, dlg.s, 1.0);
            let mut mesh = egui::Mesh::default();
            let (tc, bc) = (Color32::from_rgb(top.r, top.g, top.b), Color32::BLACK);
            mesh.colored_vertex(bar.left_top(), tc);
            mesh.colored_vertex(bar.right_top(), tc);
            mesh.colored_vertex(bar.right_bottom(), bc);
            mesh.colored_vertex(bar.left_bottom(), bc);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            let painter = ui.painter();
            painter.add(mesh);
            painter.rect_stroke(bar, 3.0, Stroke::new(1.0, p.border), StrokeKind::Outside);
            let ky = bar.min.y + (1.0 - dlg.v) * bar.height();
            painter.circle_filled(Pos2::new(bar.center().x, ky), 7.0, p.text);
            painter.circle_stroke(Pos2::new(bar.center().x, ky), 7.0, Stroke::new(1.0, p.border));

            // HEX, mode and the three channel fields.
            let fx = 366.0;
            let hex_rect = Rect::from_min_size(at(fx, 40.0), Vec2::new(130.0, 30.0));
            let hex = ui.put(
                hex_rect,
                egui::TextEdit::singleline(&mut dlg.hex)
                    .font(fonts::regular(14.0))
                    .margin(Vec2::new(8.0, 6.0)),
            );
            if hex.changed() {
                if let Some(c) = Rgba::from_hex(&dlg.hex) {
                    let keep = dlg.hex.clone();
                    dlg.set_rgb(c);
                    dlg.hex = keep;
                }
            }
            if hex.lost_focus() {
                dlg.sync_hex();
            }
            let field_bg = ui.visuals().extreme_bg_color;
            let field = |ui: &egui::Ui, r: Rect| {
                ui.painter().rect(
                    r,
                    crate::theme::RADIUS,
                    field_bg,
                    Stroke::new(1.0, p.border),
                    StrokeKind::Inside,
                );
            };
            let mode_rect = Rect::from_min_size(at(fx, 84.0), Vec2::new(130.0, 30.0));
            field(ui, mode_rect);
            ui.scope_builder(egui::UiBuilder::new().max_rect(mode_rect), |ui| {
                egui::ComboBox::from_id_salt("cd_mode")
                    .width(122.0)
                    .selected_text(if dlg.hsv_mode { "HSV" } else { "RGB" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut dlg.hsv_mode, false, "RGB");
                        ui.selectable_value(&mut dlg.hsv_mode, true, "HSV");
                    });
            });
            let names = if dlg.hsv_mode { tr.hsv_names } else { tr.rgb_names };
            for (i, name) in names.iter().enumerate() {
                let y = 128.0 + i as f32 * 44.0;
                let rect = Rect::from_min_size(at(fx, y), Vec2::new(90.0, 30.0));
                field(ui, rect);
                if dlg.hsv_mode {
                    let (mut val, max) = match i {
                        0 => (dlg.h.round() as i32 % 360, 359),
                        1 => ((dlg.s * 100.0).round() as i32, 100),
                        _ => ((dlg.v * 100.0).round() as i32, 100),
                    };
                    if ui.put(rect, egui::DragValue::new(&mut val).range(0..=max)).changed() {
                        match i {
                            0 => dlg.h = val as f32,
                            1 => dlg.s = val as f32 / 100.0,
                            _ => dlg.v = val as f32 / 100.0,
                        }
                        dlg.sync_hex();
                    }
                } else {
                    let c = dlg.color();
                    let mut val = [c.r, c.g, c.b][i];
                    if ui.put(rect, egui::DragValue::new(&mut val).range(0..=255)).changed() {
                        let mut c = c;
                        match i {
                            0 => c.r = val,
                            1 => c.g = val,
                            _ => c.b = val,
                        }
                        dlg.set_rgb(c);
                    }
                }
                ui.painter().text(
                    at(fx + 100.0, y + 15.0),
                    Align2::LEFT_CENTER,
                    *name,
                    fonts::regular(13.0),
                    p.text,
                );
            }

            // Basic colors (12 × 4) and custom colors (6 × 4 + "+").
            ui.painter().text(
                at(0.0, 318.0),
                Align2::LEFT_CENTER,
                tr.basic_colors,
                fonts::medium(13.0),
                p.text,
            );
            let current = dlg.color();
            for (i, c) in basics.iter().enumerate() {
                let center = at(11.0 + (i % 12) as f32 * 28.0, 346.0 + (i / 12) as f32 * 28.0);
                let resp = swatch(ui, center, 10.0, Id::unique(("cd_basic", i)), Some(*c), false)
                    .on_hover_text(c.to_hex());
                if resp.clicked() {
                    dlg.set_rgb(*c);
                }
                if resp.double_clicked() {
                    ok = true;
                }
            }
            ui.painter().text(
                at(380.0, 318.0),
                Align2::LEFT_CENTER,
                tr.custom_colors,
                fonts::medium(13.0),
                p.text,
            );
            let next = self.dialog_custom_next;
            for i in 0..24 {
                let center = at(391.0 + (i % 6) as f32 * 28.0, 346.0 + (i / 6) as f32 * 28.0);
                let c = self.dialog_custom[i];
                let resp = swatch(ui, center, 10.0, Id::unique(("cd_custom", i)), c, i == next);
                if let Some(c) = c {
                    if resp.on_hover_text(c.to_hex()).clicked() {
                        dlg.set_rgb(c);
                    }
                }
            }
            let plus = Rect::from_center_size(at(580.0, 318.0), Vec2::splat(26.0));
            if crate::widgets::icon_button(
                ui,
                plus,
                Id::unique("cd_add"),
                crate::icons::PLUS,
                16.0,
                Default::default(),
            )
            .on_hover_text(tr.add_custom_color)
            .clicked()
            {
                self.dialog_custom[next] = Some(current);
                self.dialog_custom_next =
                    self.dialog_custom.iter().position(Option::is_none).unwrap_or((next + 1) % 24);
            }

            // OK (primary, left) · Cancel.
            let w = (size.x - 12.0) / 2.0;
            let ok_rect = Rect::from_min_size(at(0.0, size.y - 34.0), Vec2::new(w, 34.0));
            let cancel_rect = Rect::from_min_size(at(w + 12.0, size.y - 34.0), Vec2::new(w, 34.0));
            let okb = egui::Button::new(RichText::new(tr.ok).color(p.on_accent)).fill(p.accent);
            if ui.put(ok_rect, okb).clicked() {
                ok = true;
            }
            if ui.put(cancel_rect, egui::Button::new(tr.cancel).fill(p.hover)).clicked() {
                cancel = true;
            }
            if !hex.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                ok = true;
            }
        });
        if modal.should_close() {
            cancel = true;
        }
        if ok {
            let c = dlg.color();
            self.set_slot_color(dlg.slot, c);
            self.push_custom_color(c);
        } else if !cancel {
            self.color_dialog = Some(dlg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_colors_are_distinct_and_include_black_and_white() {
        let b = basic_colors();
        assert!(b.contains(&Rgba::BLACK) && b.contains(&Rgba::WHITE));
        for i in 0..b.len() {
            for j in i + 1..b.len() {
                assert_ne!(b[i], b[j], "{i} {j}");
            }
        }
    }

    #[test]
    fn gray_keeps_hue() {
        let mut d = ColorDialog::new(ColorSlot::One, Rgba::rgb(255, 0, 0));
        d.h = 200.0;
        d.set_rgb(Rgba::rgb(128, 128, 128));
        assert_eq!(d.h, 200.0);
        assert_eq!(d.color(), Rgba::rgb(128, 128, 128));
    }
}
