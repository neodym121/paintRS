//! The ribbon. Group order, positions and sizes follow docs/behavior/layout.md (measured on
//! the original at 100%); the look is our own. Copilot is not included, so Layers moves into
//! its slot, and the "remove background" slot of the Image group holds "Invert colors".

use egui::{Align2, Color32, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use paint_core::shape::{unit_outline, ShapeKind};
use paint_core::tool::{BrushKind, SelectionKind, Tool};
use paint_core::Rgba;

use crate::actions::Action;
use crate::editor::{ColorSlot, Editor};
use crate::widgets::{self, BtnState};
use crate::{fonts, icons};

/// Group boundaries (x, relative to the ribbon's left edge).
const SEPARATORS: [f32; 7] = [74.0, 233.0, 370.0, 438.0, 727.0, 1080.0, 1145.0];
const LABEL_Y: f32 = 93.0;

/// Default palette: two rows of ten, our own color values (see DIFFS.md).
pub const PALETTE: [[Rgba; 10]; 2] = [
    [
        Rgba::rgb(0x00, 0x00, 0x00),
        Rgba::rgb(0x5C, 0x5C, 0x5C),
        Rgba::rgb(0x8E, 0x1B, 0x24),
        Rgba::rgb(0xE0, 0x35, 0x2B),
        Rgba::rgb(0xF2, 0x8A, 0x2E),
        Rgba::rgb(0xF6, 0xD0, 0x2F),
        Rgba::rgb(0x2E, 0x9E, 0x4F),
        Rgba::rgb(0x1E, 0x9A, 0xB0),
        Rgba::rgb(0x2F, 0x5B, 0xD3),
        Rgba::rgb(0x8A, 0x44, 0xB8),
    ],
    [
        Rgba::rgb(0xFF, 0xFF, 0xFF),
        Rgba::rgb(0xBD, 0xBD, 0xBD),
        Rgba::rgb(0x9A, 0x64, 0x40),
        Rgba::rgb(0xF4, 0xA3, 0xB5),
        Rgba::rgb(0xF5, 0xB8, 0x2E),
        Rgba::rgb(0xF7, 0xE9, 0xB0),
        Rgba::rgb(0xA8, 0xD8, 0x4E),
        Rgba::rgb(0x8F, 0xD3, 0xE8),
        Rgba::rgb(0x78, 0x91, 0xB8),
        Rgba::rgb(0xC4, 0xB3, 0xE0),
    ],
];

pub fn brush_icon(kind: BrushKind) -> char {
    match kind {
        BrushKind::Brush => icons::PAINTBRUSH,
        BrushKind::CalligraphyBrush => icons::FEATHER,
        BrushKind::CalligraphyPen => icons::PEN_LINE,
        BrushKind::Airbrush => icons::SPRAY_CAN,
        BrushKind::Oil => icons::PAINTBRUSH_VERTICAL,
        BrushKind::Crayon => icons::PENCIL_LINE,
        BrushKind::Marker => icons::HIGHLIGHTER,
        BrushKind::NaturalPencil => icons::PEN_TOOL,
        BrushKind::Watercolor => icons::DROPLETS,
    }
}

pub fn to_color32(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}

impl Editor {
    pub(crate) fn ribbon(&mut self, ui: &mut Ui) {
        let p = self.palette(ui.ctx());
        let tr = self.tr();
        let r = ui.max_rect();
        let o = r.min;
        let at = |x: f32, y: f32| Pos2::new(o.x + x, o.y + y);

        for x in SEPARATORS {
            widgets::vline(ui, o.x + x, o.y + 4.0, o.y + 103.0, p.separator);
        }
        let labels = [
            (37.0, tr.grp_selection),
            (153.5, tr.grp_image),
            (301.5, tr.grp_tools),
            (404.0, tr.grp_brushes),
            (582.5, tr.grp_shapes),
            (903.5, tr.grp_colors),
            (1112.5, tr.grp_layers),
        ];
        for (x, text) in labels {
            widgets::group_label(ui, at(x, LABEL_Y), text);
        }
        ui.painter().hline(r.x_range(), o.y + RIBBON_LINE, Stroke::new(1.0, p.separator));

        self.selection_group(ui, o);
        self.image_group(ui, o);
        self.tools_group(ui, o);
        self.brushes_group(ui, o);
        self.shapes_group(ui, o);
        self.colors_group(ui, o);

        // Layers (toggles the layers panel).
        let rect = widgets::centered(at(1112.5, 43.5), Vec2::splat(40.0));
        if widgets::icon_button(
            ui,
            rect,
            Id::unique("rb_layers"),
            icons::LAYERS,
            20.0,
            BtnState::selected(self.layers_panel),
        )
        .on_hover_text(tr.layers)
        .clicked()
        {
            self.layers_panel ^= true;
        }
    }

    fn selection_group(&mut self, ui: &mut Ui, o: Pos2) {
        let tr = self.tr();
        let p = self.palette(ui.ctx());
        let top = Rect::from_min_size(Pos2::new(o.x + 15.0, o.y + 10.0), Vec2::new(44.0, 40.0));
        let bottom = Rect::from_min_size(Pos2::new(o.x + 15.0, o.y + 50.0), Vec2::new(44.0, 22.0));
        let selected = self.tool == Tool::Select;
        let icon = match self.selection_kind {
            SelectionKind::Rectangle => icons::SQUARE_DASHED,
            SelectionKind::FreeForm => icons::LASSO,
        };
        let resp =
            widgets::icon_button(ui, top, Id::unique("rb_select"), icon, 22.0, BtnState::selected(selected))
                .on_hover_text(widgets::tip(tr.select, Some(&self.keymap.text(Action::ToolSelect))));
        if resp.clicked() {
            self.set_tool(Tool::Select);
        }
        let menu = ui.interact(bottom, Id::unique("rb_select_menu"), Sense::click());
        widgets::paint_button_bg(ui, bottom, &menu, BtnState::default());
        widgets::chevron(ui, bottom.center(), p.text);
        egui::Popup::menu(&menu).width(220.0).show(|ui| {
            if ui
                .add(egui::Button::selectable(
                    self.selection_kind == SelectionKind::Rectangle,
                    tr.sel_rectangle,
                ))
                .clicked()
            {
                self.selection_kind = SelectionKind::Rectangle;
                self.set_tool(Tool::Select);
            }
            if ui
                .add(egui::Button::selectable(
                    self.selection_kind == SelectionKind::FreeForm,
                    tr.sel_freeform,
                ))
                .clicked()
            {
                self.selection_kind = SelectionKind::FreeForm;
                self.set_tool(Tool::Select);
            }
            ui.separator();
            ui.add_enabled(
                false,
                egui::Button::new(tr.select_all).shortcut_text(self.keymap.text(Action::SelectAll)),
            );
            ui.add_enabled(false, egui::Button::new(tr.invert_selection));
            if ui
                .add(egui::Button::selectable(self.transparent_selection, tr.transparent_selection))
                .clicked()
            {
                self.transparent_selection ^= true;
            }
            ui.separator();
            ui.add_enabled(
                false,
                egui::Button::new(tr.delete).shortcut_text(self.keymap.text(Action::Delete)),
            );
        });
    }

    fn image_group(&mut self, ui: &mut Ui, o: Pos2) {
        let tr = self.tr();
        let p = self.palette(ui.ctx());
        let at = |x: f32, y: f32| Pos2::new(o.x + x, o.y + y);
        let off = BtnState::enabled(false);
        widgets::icon_button(
            ui,
            widgets::centered(at(102.5, 23.0), Vec2::splat(32.0)),
            Id::unique("rb_crop"),
            icons::CROP,
            16.0,
            off,
        )
        .on_hover_text(widgets::tip(tr.crop, Some(&self.keymap.text(Action::Crop))));
        widgets::icon_button(
            ui,
            widgets::centered(at(102.5, 63.0), Vec2::splat(32.0)),
            Id::unique("rb_invert"),
            icons::CONTRAST,
            16.0,
            off,
        )
        .on_hover_text(widgets::tip(tr.invert_colors, Some(&self.keymap.text(Action::InvertColors))));

        // Rotate ▾ and Flip ▾ split buttons.
        for (row_y, icon, id, title, items) in [
            (
                23.0,
                icons::ROTATE_CW,
                "rb_rotate",
                tr.rotate,
                vec![tr.rotate_right, tr.rotate_left, tr.rotate_180, tr.rotate_custom],
            ),
            (63.0, icons::FLIP_HORIZONTAL_2, "rb_flip", tr.flip, vec![tr.flip_vertical, tr.flip_horizontal]),
        ] {
            let rect = Rect::from_center_size(at(150.0, row_y), Vec2::new(44.0, 32.0));
            let resp = ui.interact(rect, Id::unique(id), Sense::click()).on_hover_text(title);
            widgets::paint_button_bg(ui, rect, &resp, BtnState::default());
            ui.painter().text(at(143.5, row_y), Align2::CENTER_CENTER, icon, fonts::icon(16.0), p.text);
            widgets::chevron(ui, at(165.5, row_y), p.text);
            egui::Popup::menu(&resp).width(240.0).show(|ui| {
                for item in items {
                    ui.add_enabled(false, egui::Button::new(item));
                }
            });
        }

        widgets::icon_button(
            ui,
            widgets::centered(at(204.5, 43.5), Vec2::splat(40.0)),
            Id::unique("rb_resize"),
            icons::SCALING,
            22.0,
            off,
        )
        .on_hover_text(widgets::tip(tr.resize_skew, Some(&self.keymap.text(Action::ResizeSkew))));
    }

    fn tools_group(&mut self, ui: &mut Ui, o: Pos2) {
        let tr = self.tr();
        let items = [
            (261.0, 23.0, Tool::Pencil, icons::PENCIL, tr.pencil, Action::ToolPencil),
            (301.0, 23.0, Tool::Fill, icons::PAINT_BUCKET, tr.fill, Action::ToolFill),
            (341.0, 23.0, Tool::Text, icons::TYPE, tr.text, Action::ToolText),
            (261.0, 63.0, Tool::Eraser, icons::ERASER, tr.eraser, Action::ToolEraser),
            (301.0, 63.0, Tool::Picker, icons::PIPETTE, tr.color_picker, Action::ToolPicker),
            (341.0, 63.0, Tool::Magnifier, icons::ZOOM_IN, tr.magnifier, Action::ToolMagnifier),
        ];
        for (x, y, tool, icon, name, action) in items {
            let rect = widgets::centered(Pos2::new(o.x + x, o.y + y), Vec2::splat(32.0));
            let resp = widgets::icon_button(
                ui,
                rect,
                Id::unique(("rb_tool", x as i32, y as i32)),
                icon,
                17.0,
                BtnState::selected(self.tool == tool),
            )
            .on_hover_text(widgets::tip(name, Some(&self.keymap.text(action))));
            if resp.clicked() {
                self.set_tool(tool);
            }
        }
    }

    fn brushes_group(&mut self, ui: &mut Ui, o: Pos2) {
        let tr = self.tr();
        let p = self.palette(ui.ctx());
        let top = Rect::from_min_size(Pos2::new(o.x + 385.0, o.y + 14.0), Vec2::new(40.0, 38.0));
        let bottom = Rect::from_min_size(Pos2::new(o.x + 385.0, o.y + 52.0), Vec2::new(40.0, 20.0));
        let selected = self.tool == Tool::Brush;
        let frame = top.union(bottom);
        // Outline around the whole split button (as in the original), accent when active.
        let stroke = if selected { Stroke::new(1.0, p.accent) } else { Stroke::new(1.0, p.border) };
        if selected {
            ui.painter().rect_filled(frame, crate::theme::RADIUS, p.selected);
        }
        ui.painter().rect_stroke(frame, crate::theme::RADIUS, stroke, StrokeKind::Inside);
        let resp = ui
            .interact(top, Id::unique("rb_brush"), Sense::click())
            .on_hover_text(tr.brushes[self.brush.index()]);
        if resp.hovered() && !selected {
            ui.painter().rect_filled(top.shrink(1.0), crate::theme::RADIUS, p.hover);
        }
        ui.painter().text(
            top.center(),
            Align2::CENTER_CENTER,
            brush_icon(self.brush),
            fonts::icon(22.0),
            p.text,
        );
        if resp.clicked() {
            self.set_tool(Tool::Brush);
        }
        let menu = ui.interact(bottom, Id::unique("rb_brush_menu"), Sense::click());
        if menu.hovered() {
            ui.painter().rect_filled(bottom.shrink(1.0), crate::theme::RADIUS, p.hover);
        }
        widgets::chevron(ui, bottom.center(), p.text);
        let previews = self.brush_previews(ui.ctx());
        egui::Popup::menu(&menu).width(300.0).show(|ui| {
            // Rows like the original: name on the left, a sample stroke on the right.
            for kind in BrushKind::ALL {
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(300.0, 44.0), Sense::click());
                let active = self.brush == kind;
                if resp.hovered() || active {
                    ui.painter().rect_filled(
                        rect,
                        crate::theme::RADIUS,
                        if active { p.selected } else { p.hover },
                    );
                }
                if active {
                    let bar = Rect::from_min_size(rect.min + Vec2::new(0.0, 12.0), Vec2::new(3.0, 20.0));
                    ui.painter().rect_filled(bar, 1.5, p.accent);
                }
                let painter = ui.painter();
                let cy = rect.center().y;
                painter.text(
                    Pos2::new(rect.min.x + 20.0, cy),
                    Align2::CENTER_CENTER,
                    brush_icon(kind),
                    fonts::icon(15.0),
                    p.text,
                );
                painter.text(
                    Pos2::new(rect.min.x + 36.0, cy),
                    Align2::LEFT_CENTER,
                    tr.brushes[kind.index()],
                    fonts::regular(13.0),
                    p.text,
                );
                let img = Rect::from_center_size(Pos2::new(rect.max.x - 44.0, cy), Vec2::new(64.0, 24.0));
                painter.image(
                    previews[kind.index()],
                    img,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    p.text,
                );
                if resp.clicked() {
                    self.brush = kind;
                    self.set_tool(Tool::Brush);
                    ui.close();
                }
            }
        });
    }

    /// Sample strokes for the brush menu, drawn once with the real brush engine.
    fn brush_previews(&mut self, ctx: &egui::Context) -> [egui::TextureId; 9] {
        let textures = self.brush_previews.get_or_insert_with(|| {
            BrushKind::ALL.map(|kind| {
                let (w, h) = (64usize, 24usize);
                let mut b = paint_core::brush::BrushStroke::new(
                    kind,
                    5,
                    Rgba::WHITE,
                    100,
                    (w as u32, h as u32),
                    0x5eed,
                );
                for i in 0..=30 {
                    let t = i as f32 / 30.0;
                    let x = 4.0 + t * 56.0;
                    let y = 12.0 - 6.0 * (t * std::f32::consts::TAU).sin();
                    b.move_to(x.round() as i32, y.round() as i32);
                }
                b.finish();
                let pixels = (0..h)
                    .flat_map(|y| (0..w).map(move |x| (x, y)))
                    .map(|(x, y)| Color32::from_white_alpha(b.coverage(x as i32, y as i32)))
                    .collect();
                ctx.load_texture(
                    format!("brush_preview_{}", kind.index()),
                    egui::ColorImage::new([w, h], pixels),
                    egui::TextureOptions::LINEAR,
                )
            })
        });
        std::array::from_fn(|i| textures[i].id())
    }

    fn shapes_group(&mut self, ui: &mut Ui, o: Pos2) {
        let tr = self.tr();
        let p = self.palette(ui.ctx());
        let frame = Rect::from_min_max(Pos2::new(o.x + 447.0, o.y + 4.0), Pos2::new(o.x + 618.0, o.y + 83.0));
        ui.painter().rect_stroke(frame, crate::theme::RADIUS, Stroke::new(1.0, p.border), StrokeKind::Inside);
        let rows = ShapeKind::ALL.len().div_ceil(7);
        let max_scroll = rows.saturating_sub(3);

        // Wheel over the gallery scrolls by rows.
        let gallery = ui.interact(frame, Id::unique("rb_shapes_area"), Sense::hover());
        if gallery.hovered() {
            let dy = ui.input(|i| crate::canvas::wheel_input(i, frame)).map_or(0.0, |(d, _)| d.y);
            if dy < 0.0 {
                self.shapes_scroll = (self.shapes_scroll + 1).min(max_scroll);
            } else if dy > 0.0 {
                self.shapes_scroll = self.shapes_scroll.saturating_sub(1);
            }
        }

        let clip =
            Rect::from_min_max(frame.min + Vec2::splat(1.0), Pos2::new(o.x + 603.0, frame.max.y - 1.0));
        let painter = ui.painter_at(clip);
        for (i, kind) in ShapeKind::ALL.into_iter().enumerate() {
            let row = (i / 7) as f32 - self.shapes_scroll as f32;
            let col = (i % 7) as f32;
            let c = Pos2::new(o.x + 462.0 + 22.0 * col, o.y + 20.0 + 22.0 * row);
            let cell = Rect::from_center_size(c, Vec2::splat(20.0));
            if !clip.intersects(cell) {
                continue;
            }
            let resp = ui
                .interact(cell.intersect(clip), Id::unique(("rb_shape", i)), Sense::click())
                .on_hover_text(tr.shapes[i]);
            let selected = self.tool == Tool::Shape && self.shape == kind;
            if selected {
                painter.rect_filled(cell, 3.0, p.selected);
                painter.rect_stroke(cell, 3.0, Stroke::new(1.0, p.accent), StrokeKind::Inside);
            } else if resp.hovered() {
                painter.rect_filled(cell, 3.0, p.hover);
            }
            draw_shape_icon(&painter, kind, Rect::from_center_size(c, Vec2::splat(13.0)), p.text);
            if resp.clicked() {
                self.shape = kind;
                self.set_tool(Tool::Shape);
            }
        }

        // Scroll column with ▲ ▼.
        let up = Rect::from_center_size(Pos2::new(o.x + 610.5, o.y + 11.0), Vec2::new(12.0, 12.0));
        let down = Rect::from_center_size(Pos2::new(o.x + 610.5, o.y + 76.0), Vec2::new(12.0, 12.0));
        if ui.interact(up, Id::unique("rb_shapes_up"), Sense::click()).clicked() {
            self.shapes_scroll = self.shapes_scroll.saturating_sub(1);
        }
        if ui.interact(down, Id::unique("rb_shapes_down"), Sense::click()).clicked() {
            self.shapes_scroll = (self.shapes_scroll + 1).min(max_scroll);
        }
        ui.painter().text(
            up.center(),
            Align2::CENTER_CENTER,
            icons::CHEVRON_UP,
            fonts::icon(10.0),
            p.text_muted,
        );
        ui.painter().text(
            down.center(),
            Align2::CENTER_CENTER,
            icons::CHEVRON_DOWN,
            fonts::icon(10.0),
            p.text_muted,
        );
        let track =
            Rect::from_min_max(Pos2::new(o.x + 608.0, o.y + 18.0), Pos2::new(o.x + 613.0, o.y + 69.0));
        let thumb_h = track.height() * 3.0 / rows as f32;
        let t = if max_scroll == 0 { 0.0 } else { self.shapes_scroll as f32 / max_scroll as f32 };
        let thumb = Rect::from_min_size(
            Pos2::new(track.min.x, track.min.y + t * (track.height() - thumb_h)),
            Vec2::new(track.width(), thumb_h),
        );
        ui.painter().rect_filled(thumb, 2.5, p.border);

        // Outline ▾ / Fill ▾ / Size ▾ (active only for shapes).
        let enabled = self.tool == Tool::Shape;
        let st = BtnState::enabled(enabled);
        let fg = widgets::fg(ui, st);
        for (row_y, icon, id, title) in [
            (24.0, icons::SQUARE_PEN, "rb_outline", tr.outline),
            (63.0, icons::PAINT_BUCKET, "rb_fill", tr.shape_fill),
        ] {
            let rect = Rect::from_center_size(Pos2::new(o.x + 648.5, o.y + row_y), Vec2::new(42.0, 30.0));
            let sense = if enabled { Sense::click() } else { Sense::hover() };
            let resp = ui.interact(rect, Id::unique(id), sense).on_hover_text(title);
            widgets::paint_button_bg(ui, rect, &resp, st);
            ui.painter().text(
                Pos2::new(o.x + 639.5, o.y + row_y),
                Align2::CENTER_CENTER,
                icon,
                fonts::icon(16.0),
                fg,
            );
            widgets::chevron(ui, Pos2::new(o.x + 661.5, o.y + row_y), fg);
        }
        let size_rect =
            Rect::from_min_max(Pos2::new(o.x + 680.0, o.y + 14.0), Pos2::new(o.x + 711.0, o.y + 72.0));
        let sense = if enabled { Sense::click() } else { Sense::hover() };
        let size_resp = ui.interact(size_rect, Id::unique("rb_size"), sense).on_hover_text(tr.size);
        widgets::paint_button_bg(ui, size_rect, &size_resp, st);
        for (k, w) in [1.0f32, 2.0, 3.5].into_iter().enumerate() {
            let y = o.y + 28.0 + k as f32 * 8.0;
            ui.painter()
                .line_segment([Pos2::new(o.x + 686.0, y), Pos2::new(o.x + 705.0, y)], Stroke::new(w, fg));
        }
        widgets::chevron(ui, Pos2::new(o.x + 695.5, o.y + 61.0), fg);
    }

    #[allow(clippy::needless_range_loop)]
    fn colors_group(&mut self, ui: &mut Ui, o: Pos2) {
        let tr = self.tr();
        let p = self.palette(ui.ctx());
        let at = |x: f32, y: f32| Pos2::new(o.x + x, o.y + y);

        // Color 1 / Color 2 slots; the active one is bigger and ringed.
        for (slot, y, color, name) in
            [(ColorSlot::One, 24.5, self.color1, tr.color1), (ColorSlot::Two, 62.5, self.color2, tr.color2)]
        {
            let c = at(755.5, y);
            let active = self.active_slot == slot;
            let radius = if active { 15.0 } else { 11.5 };
            let resp = ui
                .interact(
                    Rect::from_center_size(c, Vec2::splat(36.0)),
                    Id::unique(("rb_slot", y as i32)),
                    Sense::click(),
                )
                .on_hover_text(format!("{name}  {}", color.to_hex()));
            if resp.hovered() {
                ui.painter().circle_filled(c, radius + 4.0, p.hover);
            }
            ui.painter().circle_filled(c, radius, to_color32(color));
            ui.painter().circle_stroke(c, radius, Stroke::new(1.0, p.border));
            if active {
                ui.painter().circle_stroke(c, radius + 3.0, Stroke::new(1.5, p.text));
            }
            if resp.clicked() {
                self.active_slot = slot;
            }
        }

        // Palette: two preset rows and a row of custom colors.
        for row in 0..3 {
            for col in 0..10 {
                let c = at(792.0 + 24.0 * col as f32, 20.0 + 24.0 * row as f32);
                let color = if row < 2 { Some(PALETTE[row][col]) } else { self.custom_colors[col] };
                let rect = Rect::from_center_size(c, Vec2::splat(22.0));
                let resp = ui.interact(rect, Id::unique(("rb_pal", row, col)), Sense::click());
                if resp.hovered() {
                    ui.painter().circle_filled(c, 11.0, p.hover);
                }
                match color {
                    Some(color) => {
                        ui.painter().circle_filled(c, 8.0, to_color32(color));
                        ui.painter().circle_stroke(c, 8.0, Stroke::new(1.0, p.border));
                        let resp = resp.on_hover_text(color.to_hex());
                        let other = match self.active_slot {
                            ColorSlot::One => ColorSlot::Two,
                            ColorSlot::Two => ColorSlot::One,
                        };
                        if resp.clicked() {
                            self.set_slot_color(self.active_slot, color);
                        } else if resp.secondary_clicked() {
                            self.set_slot_color(other, color);
                        }
                    }
                    None => {
                        ui.painter().circle_stroke(c, 7.5, Stroke::new(1.0, p.border));
                    }
                }
            }
        }

        // Edit colors.
        let rect = widgets::centered(at(1047.5, 43.5), Vec2::splat(36.0));
        if widgets::icon_button(
            ui,
            rect,
            Id::unique("rb_edit_colors"),
            icons::PALETTE,
            20.0,
            BtnState::default(),
        )
        .on_hover_text(tr.edit_colors)
        .clicked()
        {
            self.open_color_dialog();
        }
    }
}

const RIBBON_LINE: f32 = 108.5;

/// Draws a gallery icon for `kind` inside `rect` using the core outline geometry.
pub fn draw_shape_icon(painter: &egui::Painter, kind: ShapeKind, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.2, color);
    for contour in unit_outline(kind) {
        let pts: Vec<Pos2> = contour
            .iter()
            .map(|[x, y]| Pos2::new(rect.min.x + x * rect.width(), rect.min.y + y * rect.height()))
            .collect();
        if kind.is_open() {
            painter.add(egui::Shape::line(pts, stroke));
        } else {
            painter.add(egui::Shape::closed_line(pts, stroke));
        }
    }
}
