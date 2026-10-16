//! Menu bar: File / Edit / View, quick buttons (Save, Undo, Redo) and Settings on the right.

use egui::{Button, Id, Pos2, Rect, Sense, Ui, Vec2};
use paint_core::io::Format;

use crate::actions::Action;
use crate::editor::{Editor, Page, Pending};
use crate::icons;
use crate::widgets::{self, BtnState};

impl Editor {
    pub(crate) fn menu_bar(&mut self, ui: &mut Ui) {
        let tr = self.tr();
        let p = self.palette(ui.ctx());
        let bar = ui.max_rect();
        let cy = bar.center().y;

        // Text menus.
        let mut x = bar.min.x + 5.0;
        let menus = [(tr.menu_file, 0u8), (tr.menu_edit, 1), (tr.menu_view, 2)];
        for (label, which) in menus {
            let galley = ui.painter().layout_no_wrap(label.to_owned(), crate::fonts::regular(14.0), p.text);
            let w = galley.size().x + 20.0;
            let rect = Rect::from_min_size(Pos2::new(x, cy - 15.0), Vec2::new(w, 30.0));
            let resp = ui.interact(rect, Id::unique(("menu", which)), Sense::click());
            widgets::paint_button_bg(ui, rect, &resp, BtnState::default());
            ui.painter().galley(rect.center() - galley.size() / 2.0, galley, p.text);
            egui::Popup::menu(&resp).width(260.0).show(|ui| match which {
                0 => self.file_menu(ui),
                1 => self.edit_menu(ui),
                _ => self.view_menu(ui),
            });
            x += w + 4.0;
        }

        // Quick buttons (after the menus: the original has them at x≈160…320 with English labels).
        let sep = |ui: &Ui, x: f32| widgets::vline(ui, x, cy - 10.0, cy + 10.0, p.separator);
        let qx = (x + 6.0).max(bar.min.x + 160.0) - bar.min.x - 160.0;
        let bar = Rect::from_min_max(Pos2::new(bar.min.x + qx, bar.min.y), Pos2::new(bar.max.x, bar.max.y));
        sep(ui, bar.min.x + 160.0);
        let btn = Vec2::splat(32.0);
        let save_rect = Rect::from_center_size(Pos2::new(bar.min.x + 183.5, cy), btn);
        if widgets::icon_button(ui, save_rect, Id::unique("qb_save"), icons::SAVE, 16.0, BtnState::default())
            .on_hover_text(widgets::tip(tr.save, Some(&self.keymap.text(Action::Save))))
            .clicked()
        {
            self.save();
        }
        sep(ui, bar.min.x + 206.0);
        let undo_rect = Rect::from_center_size(Pos2::new(bar.min.x + 228.0, cy), btn);
        if widgets::icon_button(
            ui,
            undo_rect,
            Id::unique("qb_undo"),
            icons::UNDO_2,
            16.0,
            BtnState::enabled(self.history.can_undo()),
        )
        .on_hover_text(widgets::tip(tr.undo, Some(&self.keymap.text(Action::Undo))))
        .clicked()
        {
            self.run_action(ui.ctx(), Action::Undo);
        }
        let redo_rect = Rect::from_center_size(Pos2::new(bar.min.x + 267.0, cy), btn);
        if widgets::icon_button(
            ui,
            redo_rect,
            Id::unique("qb_redo"),
            icons::REDO_2,
            16.0,
            BtnState::enabled(self.history.can_redo()),
        )
        .on_hover_text(widgets::tip(tr.redo, Some(&self.keymap.text(Action::Redo))))
        .clicked()
        {
            self.run_action(ui.ctx(), Action::Redo);
        }

        // Settings on the right.
        let gear = Rect::from_center_size(Pos2::new(bar.max.x - 28.0, cy), btn);
        if widgets::icon_button(
            ui,
            gear,
            Id::unique("qb_settings"),
            icons::SETTINGS,
            16.0,
            BtnState::default(),
        )
        .on_hover_text(tr.settings)
        .clicked()
        {
            self.page = Page::Settings;
        }

        // Bottom hairline between the menu bar and the ribbon.
        let full = ui.max_rect();
        ui.painter().hline(full.x_range(), full.max.y - 0.5, egui::Stroke::new(1.0, p.separator));
    }

    fn item(&self, ui: &mut Ui, text: &str, action: Option<Action>, enabled: bool) -> bool {
        let mut b = Button::new(text);
        if let Some(a) = action {
            b = b.shortcut_text(self.keymap.text(a));
        }
        ui.add_enabled(enabled, b).clicked()
    }

    fn file_menu(&mut self, ui: &mut Ui) {
        let tr = self.tr();
        let ctx = ui.ctx().clone();
        if self.item(ui, tr.new, Some(Action::New), true) {
            self.run_action(&ctx, Action::New);
        }
        if self.item(ui, tr.open, Some(Action::Open), true) {
            self.run_action(&ctx, Action::Open);
        }
        ui.menu_button(tr.import_to_canvas, |ui| {
            ui.add_enabled(false, Button::new(tr.from_file));
        });
        ui.menu_button(tr.recent, |ui| {
            if self.settings.recent_files.is_empty() {
                ui.add_enabled(false, Button::new(tr.recent_empty));
            }
            let mut chosen = None;
            for path in &self.settings.recent_files {
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                if ui.button(name).on_hover_text(path.display().to_string()).clicked() {
                    chosen = Some(path.clone());
                }
            }
            if let Some(path) = chosen {
                self.run_or_ask(Pending::Open(Some(path)));
            }
        });
        ui.separator();
        if self.item(ui, tr.save, Some(Action::Save), true) {
            self.run_action(&ctx, Action::Save);
        }
        ui.menu_button(tr.save_as, |ui| {
            let formats = [
                (tr.save_as_png, Some(Format::Png)),
                (tr.save_as_jpeg, Some(Format::Jpeg)),
                (tr.save_as_bmp, Some(Format::Bmp)),
                (tr.save_as_gif, Some(Format::Gif)),
                (tr.save_as_other, None),
            ];
            for (label, fmt) in formats {
                let enabled = fmt == Some(Format::Png);
                let action = (fmt == Some(Format::Png)).then_some(Action::SaveAs);
                if self.item(ui, label, action, enabled) {
                    self.save_as(fmt.unwrap_or(Format::Png));
                }
            }
        });
        ui.add_enabled(false, Button::new(tr.save_as_project));
        ui.separator();
        ui.menu_button(tr.print, |ui| {
            self.item(ui, tr.print, Some(Action::Print), false);
            ui.add_enabled(false, Button::new(tr.page_setup));
            ui.add_enabled(false, Button::new(tr.print_preview));
        });
        ui.separator();
        self.item(ui, tr.image_properties, Some(Action::ImageProperties), false);
        ui.separator();
        if ui.button(tr.exit).clicked() {
            self.request_exit();
        }
    }

    fn edit_menu(&mut self, ui: &mut Ui) {
        let tr = self.tr();
        let ctx = ui.ctx().clone();
        if self.item(ui, tr.undo, Some(Action::Undo), self.history.can_undo()) {
            self.run_action(&ctx, Action::Undo);
        }
        if self.item(ui, tr.redo, Some(Action::Redo), self.history.can_redo()) {
            self.run_action(&ctx, Action::Redo);
        }
        ui.separator();
        self.item(ui, tr.cut, Some(Action::Cut), false);
        self.item(ui, tr.copy, Some(Action::Copy), false);
        self.item(ui, tr.paste, Some(Action::Paste), false);
        ui.separator();
        self.item(ui, tr.copy_visible_layers, Some(Action::CopyVisibleLayers), false);
    }

    fn view_menu(&mut self, ui: &mut Ui) {
        let tr = self.tr();
        let ctx = ui.ctx().clone();
        ui.menu_button(tr.zoom, |ui| {
            for (label, a) in [
                (tr.zoom_100, Action::Zoom100),
                (tr.zoom_in, Action::ZoomIn),
                (tr.zoom_out, Action::ZoomOut),
                (tr.fit_to_window, Action::FitToWindow),
            ] {
                let enabled = !(a == Action::Zoom100 && (self.view.zoom - 1.0).abs() < 1e-4);
                if self.item(ui, label, Some(a), enabled) {
                    self.run_action(&ctx, a);
                }
            }
        });
        ui.separator();
        let keymap = self.keymap.clone();
        let toggle = |ui: &mut Ui, label: &str, value: bool, action: Option<Action>| {
            let mut b = Button::selectable(value, label);
            if let Some(a) = action {
                b = b.shortcut_text(keymap.text(a));
            }
            ui.add(b).clicked()
        };
        if toggle(ui, tr.rulers, self.settings.show_rulers, Some(Action::ToggleRulers)) {
            self.run_action(&ctx, Action::ToggleRulers);
        }
        if toggle(ui, tr.gridlines, self.settings.show_grid, Some(Action::ToggleGrid)) {
            self.run_action(&ctx, Action::ToggleGrid);
        }
        if toggle(ui, tr.status_bar, self.settings.show_status_bar, None) {
            self.settings.show_status_bar ^= true;
        }
        ui.separator();
        if toggle(ui, tr.full_screen, self.fullscreen, Some(Action::FullScreen)) {
            self.run_action(&ctx, Action::FullScreen);
        }
        ui.add_enabled(false, Button::new(tr.thumbnail));
    }
}
