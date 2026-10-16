//! Modal dialogs: "keep your changes?" and error messages.

use egui::{Align, Layout, RichText};

use crate::editor::{Editor, Modal, Pending};
use crate::fonts;

impl Editor {
    pub(crate) fn modals(&mut self, ctx: &egui::Context) {
        let Some(modal) = self.modal.clone() else { return };
        let tr = self.tr();
        let p = self.palette(ctx);
        let frame = egui::Frame::popup(&ctx.global_style())
            .fill(p.panel)
            .inner_margin(egui::Margin::same(20))
            .corner_radius(crate::theme::RADIUS_POPUP);
        let mut close = false;
        let mut then: Option<Pending> = None;
        let resp = egui::Modal::new(egui::Id::unique("modal")).frame(frame).show(ctx, |ui| {
            ui.set_width(380.0);
            match &modal {
                Modal::SaveChanges { then: pending } => {
                    ui.label(RichText::new(tr.save_changes_title).font(fonts::semibold(17.0)).color(p.text));
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(tr.save_changes_body.replace("{}", &self.document_name()))
                            .color(p.text_muted),
                    );
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        let w = (ui.available_width() - 2.0 * ui.spacing().item_spacing.x) / 3.0;
                        let size = egui::vec2(w, 30.0);
                        let save =
                            egui::Button::new(RichText::new(tr.save).color(p.on_accent)).fill(p.accent);
                        if ui.add_sized(size, save).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            close = true;
                            if self.save() {
                                then = Some(pending.clone());
                            }
                        }
                        if ui.add_sized(size, egui::Button::new(tr.dont_save)).clicked() {
                            close = true;
                            then = Some(pending.clone());
                        }
                        if ui.add_sized(size, egui::Button::new(tr.cancel)).clicked() {
                            close = true;
                        }
                    });
                }
                Modal::Message { title, text } => {
                    ui.label(RichText::new(title.as_str()).font(fonts::semibold(17.0)).color(p.text));
                    ui.add_space(6.0);
                    ui.label(RichText::new(text.as_str()).color(p.text_muted));
                    ui.add_space(16.0);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let ok = egui::Button::new(RichText::new(tr.ok).color(p.on_accent)).fill(p.accent);
                        if ui.add_sized(egui::vec2(110.0, 30.0), ok).clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            close = true;
                        }
                    });
                }
            }
        });
        if resp.should_close() {
            close = true;
        }
        if close {
            // `save()` may have replaced the modal with an error; keep that one.
            if matches!(self.modal, Some(ref m) if std::mem::discriminant(m) == std::mem::discriminant(&modal))
            {
                self.modal = None;
            }
            if let Some(action) = then {
                self.run_pending(action);
            }
        }
    }
}
