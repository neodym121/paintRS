//! Status bar: cursor position, selection size, image size, file size; zoom controls on the right.

use egui::{Align2, Id, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use paint_core::zoom;

use crate::actions::Action;
use crate::editor::Editor;
use crate::widgets::{self, BtnState};
use crate::{fonts, icons};

impl Editor {
    pub(crate) fn status_bar(&mut self, ui: &mut Ui) {
        let tr = self.tr();
        let p = self.palette(ui.ctx());
        let bar = ui.max_rect();
        let cy = bar.center().y;
        let x0 = bar.min.x;
        ui.painter().hline(bar.x_range(), bar.min.y + 0.5, Stroke::new(1.0, p.separator));

        let item = |ui: &Ui, x: f32, icon: char, text: String| {
            ui.painter().text(
                Pos2::new(x0 + x, cy),
                Align2::CENTER_CENTER,
                icon,
                fonts::icon(14.0),
                p.text_muted,
            );
            if !text.is_empty() {
                ui.painter().text(
                    Pos2::new(x0 + x + 18.0, cy),
                    Align2::LEFT_CENTER,
                    text,
                    fonts::regular(12.0),
                    p.text,
                );
            }
        };
        let pos = self.hover_pixel.map(|(x, y)| format!("{x}, {y}px")).unwrap_or_default();
        item(ui, 18.0, icons::MOUSE_POINTER_2, pos);
        widgets::vline(ui, x0 + 134.0, cy - 9.0, cy + 9.0, p.separator);
        item(ui, 150.0, icons::SQUARE_DASHED, String::new());
        widgets::vline(ui, x0 + 266.0, cy - 9.0, cy + 9.0, p.separator);
        item(ui, 282.0, icons::FRAME, format!("{} × {}px", self.doc.width(), self.doc.height()));
        if let Some(size) = self.file_size {
            widgets::vline(ui, x0 + 398.0, cy - 9.0, cy + 9.0, p.separator);
            item(ui, 414.0, icons::HARD_DRIVE, format!("{}: {}", tr.file_size, human_size(size)));
        }

        // Right side: fit · [zoom ▾] · − slider +
        let image = (self.doc.width(), self.doc.height());
        let ppp = ui.ctx().pixels_per_point();
        let right = bar.max.x;
        let btn = Vec2::splat(26.0);
        let fit = Rect::from_center_size(Pos2::new(right - 334.0, cy), btn);
        if widgets::icon_button(ui, fit, Id::unique("sb_fit"), icons::SCAN, 16.0, BtnState::default())
            .on_hover_text(widgets::tip(tr.fit_to_window, Some(&self.keymap.text(Action::FitToWindow))))
            .clicked()
        {
            self.view.fit(image, ppp);
        }

        // Editable zoom field with presets.
        let field =
            Rect::from_min_max(Pos2::new(right - 306.0, cy - 13.0), Pos2::new(right - 215.0, cy + 13.0));
        ui.painter().rect_filled(field, crate::theme::RADIUS, p.hover);
        let text_rect = Rect::from_min_max(
            field.min + Vec2::new(6.0, 3.0),
            Pos2::new(field.max.x - 24.0, field.max.y - 3.0),
        );
        let mut text = self.zoom_text.clone().unwrap_or_else(|| zoom::label(self.view.zoom));
        let edit = ui.put(
            text_rect,
            egui::TextEdit::singleline(&mut text).frame(egui::Frame::NONE).font(fonts::regular(12.0)),
        );
        if edit.has_focus() {
            self.zoom_text = Some(text.clone());
        }
        if edit.lost_focus() {
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some(z) = zoom::parse(&text) {
                    self.view.zoom_to(z, None, image, ppp);
                }
            }
            self.zoom_text = None;
        }
        let drop = Rect::from_min_max(Pos2::new(field.max.x - 24.0, field.min.y), field.max);
        let drop_resp = ui.interact(drop, Id::unique("sb_zoom_presets"), Sense::click());
        widgets::chevron(ui, drop.center(), p.text);
        egui::Popup::menu(&drop_resp).width(110.0).show(|ui| {
            for z in zoom::PRESETS {
                let selected = (self.view.zoom - z).abs() < 1e-4;
                if ui.add(egui::Button::selectable(selected, zoom::label(z))).clicked() {
                    self.view.zoom_to(z, None, image, ppp);
                }
            }
        });

        let out_rect = Rect::from_center_size(Pos2::new(right - 190.0, cy), btn);
        if widgets::icon_button(
            ui,
            out_rect,
            Id::unique("sb_zoom_out"),
            icons::ZOOM_OUT,
            15.0,
            BtnState::default(),
        )
        .on_hover_text(widgets::tip(tr.zoom_out, Some(&self.keymap.text(Action::ZoomOut))))
        .clicked()
        {
            self.view.zoom_to(zoom::zoom_out(self.view.zoom), None, image, ppp);
        }
        let in_rect = Rect::from_center_size(Pos2::new(right - 20.0, cy), btn);
        if widgets::icon_button(
            ui,
            in_rect,
            Id::unique("sb_zoom_in"),
            icons::ZOOM_IN,
            15.0,
            BtnState::default(),
        )
        .on_hover_text(widgets::tip(tr.zoom_in, Some(&self.keymap.text(Action::ZoomIn))))
        .clicked()
        {
            self.view.zoom_to(zoom::zoom_in(self.view.zoom), None, image, ppp);
        }

        // Logarithmic zoom slider 10%…1000%.
        let track = Rect::from_min_max(Pos2::new(right - 170.0, cy - 8.0), Pos2::new(right - 41.0, cy + 8.0));
        let resp = ui.interact(track, Id::unique("sb_zoom_slider"), Sense::click_and_drag());
        let (lo, hi) = (zoom::MIN.ln(), zoom::MAX.ln());
        if let Some(ptr) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            let t = ((ptr.x - track.min.x) / track.width()).clamp(0.0, 1.0);
            let z = (lo + t * (hi - lo)).exp();
            // Snap near 100% for convenience.
            let z = if (z - 1.0).abs() < 0.04 { 1.0 } else { (z * 1000.0).round() / 1000.0 };
            self.view.zoom_to(z, None, image, ppp);
        }
        let t = (self.view.zoom.ln() - lo) / (hi - lo);
        ui.painter().line_segment(
            [Pos2::new(track.min.x, cy), Pos2::new(track.max.x, cy)],
            Stroke::new(2.0, p.border),
        );
        let knob = Pos2::new(track.min.x + t * track.width(), cy);
        ui.painter().circle_filled(
            knob,
            6.0,
            if resp.hovered() || resp.dragged() { p.accent } else { p.text_muted },
        );
    }
}

pub fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < KB * KB {
        format!("{:.1}KB", b / KB)
    } else {
        format!("{:.1}MB", b / KB / KB)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sizes() {
        assert_eq!(super::human_size(512), "512 B");
        assert_eq!(super::human_size(27_750), "27.1KB");
        assert_eq!(super::human_size(5 * 1024 * 1024), "5.0MB");
    }
}
