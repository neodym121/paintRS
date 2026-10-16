//! Settings page (replaces the editor, like the original): theme, language, canvas smoothing.

use egui::{RichText, Ui};

use crate::editor::{Editor, Page};
use crate::i18n::Lang;
use crate::settings::Smoothing;
use crate::theme::ThemeMode;
use crate::{fonts, icons};

impl Editor {
    pub(crate) fn settings_page(&mut self, ui: &mut Ui) {
        let tr = self.tr();
        let p = self.palette(ui.ctx());
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(p.chrome).inner_margin(egui::Margin::symmetric(24, 16)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let back = egui::Button::new(
                        RichText::new(icons::ARROW_LEFT.to_string()).font(fonts::icon(16.0)),
                    );
                    if ui.add(back).on_hover_text(tr.back).clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        self.page = Page::Editor;
                    }
                    ui.label(RichText::new(tr.settings).font(fonts::semibold(22.0)).color(p.text));
                });
                ui.add_space(16.0);
                ui.set_max_width(640.0);

                section(ui, tr.appearance);
                ui.label(RichText::new(tr.theme).color(p.text_muted));
                ui.horizontal(|ui| {
                    for (mode, label) in [
                        (ThemeMode::Light, tr.theme_light),
                        (ThemeMode::Dark, tr.theme_dark),
                        (ThemeMode::System, tr.theme_system),
                    ] {
                        if ui.radio(self.settings.theme == mode, label).clicked() {
                            self.settings.theme = mode;
                            self.applied_theme = None;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new(tr.language).color(p.text_muted));
                ui.horizontal(|ui| {
                    for (lang, label) in [(Lang::Ru, "Русский"), (Lang::En, "English")] {
                        if ui.radio(self.settings.language == lang, label).clicked() {
                            self.settings.language = lang;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new(tr.smoothing).color(p.text_muted));
                ui.horizontal(|ui| {
                    for (mode, label) in [
                        (Smoothing::Auto, tr.smoothing_auto),
                        (Smoothing::Always, tr.smoothing_always),
                        (Smoothing::Never, tr.smoothing_never),
                    ] {
                        if ui.radio(self.settings.smoothing == mode, label).clicked() {
                            self.settings.smoothing = mode;
                        }
                    }
                });
                ui.label(RichText::new(tr.smoothing_hint).small().color(p.text_muted));

                ui.add_space(20.0);
                section(ui, tr.about);
                ui.label(RichText::new(format!("PaintRS {}", env!("CARGO_PKG_VERSION"))).color(p.text));
                ui.label(RichText::new(tr.about_body).color(p.text_muted));
            });
    }
}

fn section(ui: &mut Ui, title: &str) {
    ui.label(RichText::new(title).font(fonts::semibold(15.0)));
    ui.add_space(4.0);
}
