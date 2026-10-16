//! Canvas input for the drawing tools (stage 2: pencil, eraser, fill, picker, magnifier) and the
//! floating side sliders (docs/behavior/tools.md).

use egui::{
    Align2, Color32, CursorIcon, Id, Key, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};
use paint_core::brush::BrushStroke;
use paint_core::draw::{flood_region, snap_45, EraseMode, EraserStroke, PencilStroke};
use paint_core::tool::{size_for_position, BrushKind, Tool, SIZE_POSITIONS};
use paint_core::{zoom, PixelRecorder, Rgba};

use crate::canvas::CanvasGeom;
use crate::editor::Editor;
use crate::ribbon::to_color32;
use crate::{fonts, icons};

/// Per-tool settings shown in the side sliders. Each tool and each brush keeps its own values,
/// like the original. Sizes are stored as slider positions (see [`size_for_position`]).
#[derive(Clone, Debug)]
pub struct ToolSettings {
    pub pencil_pos: u32,
    /// Percent, 0…100.
    pub pencil_opacity: u8,
    pub eraser_pos: u32,
    pub brush_pos: [u32; 9],
    pub brush_opacity: [u8; 9],
    /// Percent, 0…100.
    pub fill_tolerance: u8,
}

impl Default for ToolSettings {
    fn default() -> Self {
        // Defaults measured on the original: pencil 1 px, eraser 8 px, brush 3 px, opacity 100%.
        Self {
            pencil_pos: 1,
            pencil_opacity: 100,
            eraser_pos: 8,
            brush_pos: BrushKind::ALL.map(|k| k.min_size().max(3)),
            brush_opacity: [100; 9],
            fill_tolerance: 0,
        }
    }
}

impl ToolSettings {
    pub fn pencil_size(&self) -> u32 {
        size_for_position(self.pencil_pos)
    }

    pub fn eraser_size(&self) -> u32 {
        size_for_position(self.eraser_pos)
    }

    pub fn brush_size(&self, kind: BrushKind) -> u32 {
        size_for_position(self.brush_pos[kind.index()]).max(kind.min_size())
    }
}

/// Percent (0…100) to an alpha byte.
pub fn opacity_alpha(percent: u8) -> u8 {
    ((percent.min(100) as u32 * 255 + 50) / 100) as u8
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SliderId {
    Size,
    Opacity,
    Tolerance,
}

enum StrokeTool {
    Pencil(PencilStroke),
    Eraser(EraserStroke),
    Brush(Box<BrushStroke>),
}

pub(crate) struct ActiveStroke {
    tool: StrokeTool,
    rec: PixelRecorder,
    button: PointerButton,
    start: (i32, i32),
    last: (i32, i32),
    label: &'static str,
    /// Time of the last airbrush tick.
    ticked: std::time::Instant,
}

/// Pointer events of one frame in the order they happened, plus the state at the end of it.
struct PointerFrame {
    events: Vec<PointerEvent>,
    hover: Option<Pos2>,
    shift: bool,
    space: bool,
    primary_down: bool,
    secondary_down: bool,
}

enum PointerEvent {
    Move(Pos2),
    Press(Pos2, PointerButton),
    Release(PointerButton),
}

impl Editor {
    fn image_pixel(geom: &CanvasGeom, p: Pos2) -> (i32, i32) {
        let (x, y) = geom.to_image(p);
        (x.floor() as i32, y.floor() as i32)
    }

    /// Handles pointer input on the canvas for the active tool. `blocked` are floating panels
    /// over the canvas; `popup_was_open` is true when a menu was open at the start of the frame
    /// (the click that closes a menu must not draw). Events are handled in the order they came,
    /// so a release and the next press within one frame make two separate strokes.
    pub(crate) fn canvas_input(
        &mut self,
        ui: &egui::Ui,
        resp: &egui::Response,
        geom: &CanvasGeom,
        blocked: &[Rect],
        popup_was_open: bool,
    ) {
        let f = ui.input(|i| PointerFrame {
            events: i
                .events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::PointerMoved(p) => Some(PointerEvent::Move(*p)),
                    egui::Event::PointerButton { pos, button, pressed: true, .. } => {
                        Some(PointerEvent::Press(*pos, *button))
                    }
                    egui::Event::PointerButton { button, pressed: false, .. } => {
                        Some(PointerEvent::Release(*button))
                    }
                    _ => None,
                })
                .collect(),
            hover: i.pointer.hover_pos(),
            shift: i.modifiers.shift,
            space: i.key_down(Key::Space),
            primary_down: i.pointer.button_down(PointerButton::Primary),
            secondary_down: i.pointer.button_down(PointerButton::Secondary),
        });
        let free = |p: Pos2| geom.viewport.contains(p) && !blocked.iter().any(|r| r.contains(p));
        let over_viewport = f.hover.is_some_and(free) && resp.contains_pointer();
        let dialog_open = self.modal.is_some() || self.color_dialog.is_some();

        if over_viewport && !f.space {
            let icon = match self.tool {
                Tool::Eraser => CursorIcon::None,
                Tool::Text => CursorIcon::Text,
                Tool::Magnifier => CursorIcon::ZoomIn,
                _ => CursorIcon::Crosshair,
            };
            ui.ctx().set_cursor_icon(icon);
        }

        for e in &f.events {
            match *e {
                PointerEvent::Move(p) => {
                    if self.stroke.is_some() {
                        self.continue_stroke(Self::image_pixel(geom, p), f.shift);
                    }
                }
                PointerEvent::Press(pos, button) => {
                    if !free(pos) || dialog_open {
                        continue;
                    }
                    // A press on the canvas takes keyboard focus away from the side sliders.
                    self.focused_slider = None;
                    if f.space || popup_was_open || self.stroke.is_some() {
                        continue;
                    }
                    let px = Self::image_pixel(geom, pos);
                    let secondary = button == PointerButton::Secondary;
                    if !matches!(button, PointerButton::Primary | PointerButton::Secondary) {
                        continue;
                    }
                    match self.tool {
                        Tool::Pencil | Tool::Eraser | Tool::Brush => self.begin_stroke(px, button),
                        Tool::Fill => self.fill_at(px, secondary),
                        Tool::Picker => self.pick_at(px, secondary),
                        Tool::Magnifier => {
                            let image = (self.doc.width(), self.doc.height());
                            let ppp = ui.ctx().pixels_per_point();
                            let z = if secondary {
                                zoom::zoom_out(self.view.zoom)
                            } else {
                                zoom::zoom_in(self.view.zoom)
                            };
                            self.view.zoom_to(z, Some(pos), image, ppp);
                        }
                        _ => {}
                    }
                }
                PointerEvent::Release(button) => {
                    if self.stroke.as_ref().is_some_and(|s| s.button == button) {
                        self.end_stroke();
                    }
                }
            }
        }

        // Safety net: the button went up without us seeing the event (e.g. focus loss).
        if let Some(button) = self.stroke.as_ref().map(|s| s.button) {
            let still_down = match button {
                PointerButton::Secondary => f.secondary_down,
                _ => f.primary_down,
            };
            if still_down {
                self.tick_stroke(ui.ctx());
            } else {
                self.end_stroke();
            }
        }

        if over_viewport && !f.space {
            if let Some(pos) = f.hover {
                self.draw_tool_preview(ui, geom, pos);
            }
        }
    }

    /// What the original draws under the pointer: a dab of the pencil in color 1 at the tool
    /// opacity, or the square outline of the eraser.
    fn draw_tool_preview(&self, ui: &egui::Ui, geom: &CanvasGeom, pos: Pos2) {
        let (x, y) = Self::image_pixel(geom, pos);
        let painter = ui.painter_at(geom.viewport.intersect(geom.canvas));
        match self.tool {
            Tool::Pencil => {
                let size = self.tool_settings.pencil_size() as f32;
                // Odd sizes are centered on the pixel, even sizes on its top-left corner.
                let c = if size as u32 % 2 == 1 { 0.5 } else { 0.0 };
                let center = geom.to_screen(x as f32 + c, y as f32 + c);
                let mut color = self.color1;
                color.a = opacity_alpha(self.tool_settings.pencil_opacity);
                let r = size / 2.0 * geom.scale;
                if size <= 1.0 || r < 1.0 {
                    let a = geom.to_screen(x as f32, y as f32);
                    let rect = Rect::from_min_size(a, Vec2::splat(geom.scale.max(1.0)));
                    painter.rect_filled(rect, 0.0, to_color32(color));
                } else {
                    painter.circle_filled(center, r, to_color32(color));
                }
            }
            Tool::Brush => {
                let kind = self.brush;
                let size = self.tool_settings.brush_size(kind) as f32;
                let c = if size as u32 % 2 == 1 { 0.5 } else { 0.0 };
                let center = geom.to_screen(x as f32 + c, y as f32 + c);
                let r = (size / 2.0 * geom.scale).max(1.5);
                match kind {
                    BrushKind::Brush => {
                        // Like the pencil: a dab of color 1 at the brush opacity.
                        let mut color = self.color1;
                        color.a = opacity_alpha(self.tool_settings.brush_opacity[kind.index()]);
                        let r = ((size - 1.0).max(1.0) / 2.0 * geom.scale).max(0.75);
                        painter.circle_filled(center, r, to_color32(color));
                    }
                    BrushKind::CalligraphyBrush | BrushKind::CalligraphyPen => {
                        let d = size / 2.0 * std::f32::consts::FRAC_1_SQRT_2 * geom.scale;
                        let v = if kind == BrushKind::CalligraphyPen {
                            Vec2::new(d, d)
                        } else {
                            Vec2::new(d, -d)
                        };
                        painter.line_segment([center - v, center + v], Stroke::new(3.0, Color32::WHITE));
                        painter.line_segment([center - v, center + v], Stroke::new(1.0, Color32::BLACK));
                    }
                    _ => {
                        // The airbrush shows the outline of its spray area.
                        painter.circle_stroke(center, r, Stroke::new(3.0, Color32::from_white_alpha(160)));
                        painter.circle_stroke(center, r, Stroke::new(1.0, Color32::BLACK));
                    }
                }
            }
            Tool::Eraser => {
                let s = self.tool_settings.eraser_size() as i32;
                let x0 = x - (s - 1) / 2;
                let y0 = y - (s - 1) / 2;
                let a = geom.to_screen(x0 as f32, y0 as f32);
                let b = geom.to_screen((x0 + s) as f32, (y0 + s) as f32);
                let r = Rect::from_min_max(a, b);
                let painter = ui.painter_at(geom.viewport);
                painter.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Outside);
                painter.rect_stroke(
                    r.expand(1.0),
                    0.0,
                    Stroke::new(1.0, Color32::WHITE),
                    StrokeKind::Outside,
                );
            }
            _ => {}
        }
    }

    fn tool_color(&self, secondary: bool) -> Rgba {
        if secondary {
            self.color2
        } else {
            self.color1
        }
    }

    fn begin_stroke(&mut self, px: (i32, i32), button: PointerButton) {
        let secondary = button == PointerButton::Secondary;
        let layer = self.doc.active_id();
        let canvas = (self.doc.width(), self.doc.height());
        let (tool, label) = match self.tool {
            Tool::Brush => {
                let b = self.brush;
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(1, |d| d.as_nanos() as u64);
                let s = BrushStroke::new(
                    b,
                    self.tool_settings.brush_size(b),
                    self.tool_color(secondary),
                    self.tool_settings.brush_opacity[b.index()],
                    canvas,
                    seed,
                );
                (StrokeTool::Brush(Box::new(s)), "brush")
            }
            Tool::Pencil => {
                let mut c = self.tool_color(secondary);
                c.a = opacity_alpha(self.tool_settings.pencil_opacity);
                let s = PencilStroke::new(c.premultiplied(), self.tool_settings.pencil_size(), canvas);
                (StrokeTool::Pencil(s), "pencil")
            }
            _ => {
                // Left button erases to transparency; right button replaces color 1 with color 2.
                let mode = if secondary {
                    EraseMode::Replace { from: self.color1.premultiplied(), to: self.color2.premultiplied() }
                } else {
                    EraseMode::Clear
                };
                (StrokeTool::Eraser(EraserStroke::new(self.tool_settings.eraser_size(), mode)), "eraser")
            }
        };
        self.stroke = Some(ActiveStroke {
            tool,
            rec: PixelRecorder::new(layer),
            button,
            start: px,
            last: px,
            label,
            ticked: std::time::Instant::now(),
        });
        self.paint_to(px);
    }

    fn paint_to(&mut self, (x, y): (i32, i32)) {
        let Some(stroke) = self.stroke.as_mut() else { return };
        let id = stroke.rec.layer();
        let bounds = match &mut stroke.tool {
            StrokeTool::Pencil(s) => {
                let bounds = s.segment_bounds(x, y);
                stroke.rec.touch(&self.doc, bounds);
                if let Some(raster) = self.doc.raster_mut(id) {
                    s.line_to(raster, x, y);
                }
                bounds
            }
            StrokeTool::Eraser(s) => {
                let bounds = s.segment_bounds(x, y);
                stroke.rec.touch(&self.doc, bounds);
                if let Some(raster) = self.doc.raster_mut(id) {
                    s.line_to(raster, x, y);
                }
                bounds
            }
            StrokeTool::Brush(b) => {
                // The brush keeps its own coverage; the layer is re-rendered from the
                // original pixels wherever the coverage changed.
                let bounds = b.move_to(x, y);
                stroke.rec.touch(&self.doc, bounds);
                if let Some(raster) = self.doc.raster_mut(id) {
                    b.composite(raster, &stroke.rec, bounds);
                }
                bounds
            }
        };
        stroke.last = (x, y);
        self.doc.mark_dirty(bounds);
    }

    /// Time-based painting (the airbrush keeps spraying while the button is held).
    fn tick_stroke(&mut self, ctx: &egui::Context) {
        let Some(stroke) = self.stroke.as_mut() else { return };
        let StrokeTool::Brush(b) = &mut stroke.tool else { return };
        if !b.needs_ticks() {
            return;
        }
        let now = std::time::Instant::now();
        let dt = now.duration_since(stroke.ticked).as_secs_f32();
        stroke.ticked = now;
        let bounds = b.tick(dt);
        let id = stroke.rec.layer();
        stroke.rec.touch(&self.doc, bounds);
        if let Some(raster) = self.doc.raster_mut(id) {
            b.composite(raster, &stroke.rec, bounds);
        }
        self.doc.mark_dirty(bounds);
        ctx.request_repaint();
    }

    fn continue_stroke(&mut self, px: (i32, i32), shift: bool) {
        let Some(stroke) = self.stroke.as_mut() else { return };
        if shift {
            // Shift turns the whole stroke into one straight line from the press point,
            // snapped to a multiple of 45°: undo what was painted and redraw.
            let end = snap_45(stroke.start.0, stroke.start.1, px.0, px.1);
            if end == stroke.last {
                return;
            }
            stroke.rec.revert(&mut self.doc);
            match &mut stroke.tool {
                StrokeTool::Pencil(s) => s.restart(),
                StrokeTool::Eraser(s) => s.restart(),
                StrokeTool::Brush(b) => b.restart(),
            }
            let start = stroke.start;
            self.paint_to(start);
            self.paint_to(end);
        } else if px != stroke.last {
            self.paint_to(px);
        }
    }

    /// Commits the stroke in progress (if any) as one history entry.
    pub(crate) fn end_stroke(&mut self) {
        if let Some(mut stroke) = self.stroke.take() {
            if let StrokeTool::Brush(b) = &mut stroke.tool {
                let bounds = b.finish();
                stroke.rec.touch(&self.doc, bounds);
                if let Some(raster) = self.doc.raster_mut(stroke.rec.layer()) {
                    b.composite(raster, &stroke.rec, bounds);
                }
                self.doc.mark_dirty(bounds);
            }
            self.history.push_pixels(&self.doc, stroke.label, stroke.rec);
        }
    }

    fn fill_at(&mut self, (x, y): (i32, i32), secondary: bool) {
        if !self.doc.bounds().contains(x, y) {
            return;
        }
        let id = self.doc.active_id();
        let color = self.tool_color(secondary).premultiplied();
        let tolerance = self.tool_settings.fill_tolerance;
        let Some(region) = self.doc.layer(id).map(|l| flood_region(&l.raster, x, y, tolerance)) else {
            return;
        };
        let mut rec = PixelRecorder::new(id);
        rec.touch(&self.doc, region.bounds);
        if let Some(raster) = self.doc.raster_mut(id) {
            region.paint(raster, color);
        }
        self.doc.mark_dirty(region.bounds);
        self.history.push_pixels(&self.doc, "fill", rec);
    }

    fn pick_at(&mut self, (x, y): (i32, i32), secondary: bool) {
        if !self.doc.bounds().contains(x, y) {
            return;
        }
        let Some(mips) = &self.mips else { return };
        if let Some(px) = mips.level(0).get(x, y) {
            let c = px.unpremultiplied();
            // A fully transparent pixel gives the background color.
            let c = if c.a == 0 { self.doc.background.color } else { Rgba { a: 255, ..c } };
            if secondary {
                self.color2 = c;
            } else {
                self.color1 = c;
            }
        }
        // The original returns to the previous tool after one pick.
        self.set_tool(self.prev_tool);
    }

    // ---------------------------------------------------------------- side sliders

    /// Sliders shown for the current tool, top to bottom.
    pub(crate) fn slider_ids(&self) -> &'static [SliderId] {
        match self.tool {
            Tool::Pencil | Tool::Brush => &[SliderId::Size, SliderId::Opacity],
            Tool::Eraser => &[SliderId::Size],
            Tool::Fill => &[SliderId::Tolerance],
            _ => &[],
        }
    }

    fn slider_range(&self, id: SliderId) -> (u32, u32) {
        match id {
            // Brushes with a 4 px minimum start their scale at position 4 (= 4 px).
            SliderId::Size if self.tool == Tool::Brush => (self.brush.min_size(), SIZE_POSITIONS),
            SliderId::Size => (1, SIZE_POSITIONS),
            SliderId::Opacity | SliderId::Tolerance => (0, 100),
        }
    }

    fn slider_value(&self, id: SliderId) -> u32 {
        let t = &self.tool_settings;
        let b = self.brush.index();
        match (id, self.tool) {
            (SliderId::Size, Tool::Pencil) => t.pencil_pos,
            (SliderId::Size, Tool::Eraser) => t.eraser_pos,
            (SliderId::Size, _) => t.brush_pos[b],
            (SliderId::Opacity, Tool::Pencil) => t.pencil_opacity as u32,
            (SliderId::Opacity, _) => t.brush_opacity[b] as u32,
            (SliderId::Tolerance, _) => t.fill_tolerance as u32,
        }
    }

    fn set_slider_value(&mut self, id: SliderId, v: u32) {
        let (lo, hi) = self.slider_range(id);
        let v = v.clamp(lo, hi);
        let b = self.brush.index();
        let t = &mut self.tool_settings;
        match (id, self.tool) {
            (SliderId::Size, Tool::Pencil) => t.pencil_pos = v,
            (SliderId::Size, Tool::Eraser) => t.eraser_pos = v,
            (SliderId::Size, _) => t.brush_pos[b] = v,
            (SliderId::Opacity, Tool::Pencil) => t.pencil_opacity = v as u8,
            (SliderId::Opacity, _) => t.brush_opacity[b] = v as u8,
            (SliderId::Tolerance, _) => t.fill_tolerance = v as u8,
        }
    }

    fn slider_label(&self, id: SliderId) -> String {
        let v = self.slider_value(id);
        match id {
            SliderId::Size => {
                let size = match self.tool {
                    Tool::Brush => self.tool_settings.brush_size(self.brush),
                    _ => size_for_position(v),
                };
                format!("{size}px")
            }
            _ => format!("{v}%"),
        }
    }

    /// Draws the side sliders for the current tool and handles their input.
    pub(crate) fn side_sliders(&mut self, ui: &mut egui::Ui, panels: &[Rect]) {
        let ids = self.slider_ids();
        if !ids.contains(&self.focused_slider.unwrap_or(SliderId::Size)) {
            self.focused_slider = None;
        }
        // Any press outside the focused slider takes the focus away.
        let pressed_at = ui.input(|i| if i.pointer.any_pressed() { i.pointer.interact_pos() } else { None });
        if let Some(p) = pressed_at {
            if !panels.iter().any(|r| r.contains(p)) {
                self.focused_slider = None;
            }
        }
        for (&id, &rect) in ids.iter().zip(panels) {
            self.side_slider(ui, id, rect);
        }
    }

    fn side_slider(&mut self, ui: &mut egui::Ui, id: SliderId, rect: Rect) {
        let p = self.palette(ui.ctx());
        let (lo, hi) = self.slider_range(id);
        let k = rect.height() / PANEL_H;
        let (top, bottom) = (rect.min.y + 49.5 * k, rect.max.y - 21.5 * k);
        let cx = rect.center().x;
        let resp = ui.interact(rect, Id::unique(("side_slider", id)), Sense::click_and_drag());

        if resp.is_pointer_button_down_on() || resp.dragged() || resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let t = ((bottom - pos.y) / (bottom - top)).clamp(0.0, 1.0);
                self.set_slider_value(id, lo + (t * (hi - lo) as f32).round() as u32);
                self.focused_slider = Some(id);
            }
        }

        let focused = self.focused_slider == Some(id);
        if focused {
            let v = self.slider_value(id);
            let (up, down, home, end) = ui.input_mut(|i| {
                let n = egui::Modifiers::NONE;
                (
                    i.count_and_consume_key(n, Key::ArrowUp) + i.count_and_consume_key(n, Key::ArrowRight),
                    i.count_and_consume_key(n, Key::ArrowDown) + i.count_and_consume_key(n, Key::ArrowLeft),
                    i.consume_key(n, Key::Home),
                    i.consume_key(n, Key::End),
                )
            });
            let mut nv = (v + up as u32).saturating_sub(down as u32).max(lo);
            if home {
                nv = lo;
            }
            if end {
                nv = hi;
            }
            if nv != v {
                self.set_slider_value(id, nv);
            }
        }

        // Look: our own panel, same layout as the original (icon on top, track, round knob).
        let painter = ui.painter();
        painter.rect(rect, 6.0, p.panel, Stroke::new(1.0, p.border), StrokeKind::Inside);
        if focused {
            painter.rect_stroke(rect.shrink(2.0), 5.0, Stroke::new(1.5, p.accent), StrokeKind::Inside);
        }
        let icon_c = Pos2::new(cx, rect.min.y + 20.0 * k);
        match id {
            SliderId::Size => {
                for (i, w) in [1.0f32, 2.0, 3.0].into_iter().enumerate() {
                    let y = icon_c.y - 5.0 + i as f32 * 5.0;
                    painter.line_segment(
                        [Pos2::new(cx - 7.0, y), Pos2::new(cx + 7.0, y)],
                        Stroke::new(w, p.text),
                    );
                }
            }
            SliderId::Opacity => {
                painter.text(icon_c, Align2::CENTER_CENTER, icons::DROPLET, fonts::icon(15.0), p.text);
            }
            SliderId::Tolerance => {
                painter.text(icon_c, Align2::CENTER_CENTER, icons::PAINT_BUCKET, fonts::icon(15.0), p.text);
            }
        }
        painter.line_segment(
            [Pos2::new(cx, top - 7.0 * k), Pos2::new(cx, bottom + 8.0 * k)],
            Stroke::new(2.0, p.text_muted),
        );
        let v = self.slider_value(id);
        let t = (v - lo) as f32 / (hi - lo) as f32;
        let knob = Pos2::new(cx, bottom - t * (bottom - top));
        let active = resp.hovered() || resp.dragged();
        if active || focused {
            painter.circle_filled(knob, 10.0, p.hover);
        }
        painter.circle_filled(knob, 6.0, if active { p.accent } else { p.text });

        if active {
            let text = self.slider_label(id);
            let galley = painter.layout_no_wrap(text, fonts::regular(12.0), p.text);
            let size = galley.size() + Vec2::new(12.0, 6.0);
            let r = Rect::from_min_size(Pos2::new(rect.max.x + 6.0, knob.y - size.y / 2.0), size);
            let tip = ui
                .painter()
                .clone()
                .with_layer_id(egui::LayerId::new(egui::Order::Tooltip, Id::unique("slider_tip")));
            tip.rect(r, 4.0, p.panel, Stroke::new(1.0, p.border), StrokeKind::Inside);
            tip.galley(r.min + Vec2::new(6.0, 3.0), galley, p.text);
        }
    }
}

/// Size of a side-slider panel in the original (points at 100%).
pub const PANEL_W: f32 = 40.0;
pub const PANEL_H: f32 = 360.0;
/// Gap between two panels, stacked or side by side.
pub const PANEL_GAP: f32 = 8.0;

/// Screen rects of `count` side-slider panels at the left edge of the workspace: stacked
/// vertically when there is room, side by side otherwise (as the original does).
pub fn slider_panels(workspace: Rect, count: usize) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    let n = count as f32;
    let left = workspace.min.x + 12.0;
    let stacked_h = PANEL_H * n + PANEL_GAP * (n - 1.0);
    if count > 1 && workspace.height() >= stacked_h + 48.0 {
        let top = workspace.center().y - stacked_h / 2.0;
        return (0..count)
            .map(|i| {
                Rect::from_min_size(
                    Pos2::new(left, top + i as f32 * (PANEL_H + PANEL_GAP)),
                    Vec2::new(PANEL_W, PANEL_H),
                )
            })
            .collect();
    }
    let h = PANEL_H.min(workspace.height() - 16.0).max(120.0);
    let top = workspace.center().y - h / 2.0;
    (0..count)
        .map(|i| {
            Rect::from_min_size(
                Pos2::new(left + i as f32 * (PANEL_W + PANEL_GAP), top),
                Vec2::new(PANEL_W, h),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_percent_to_alpha() {
        assert_eq!(opacity_alpha(100), 255);
        assert_eq!(opacity_alpha(0), 0);
        assert_eq!(opacity_alpha(50), 128);
    }

    #[test]
    fn panels_stack_when_tall_enough() {
        let tall = Rect::from_min_size(Pos2::ZERO, Vec2::new(1900.0, 808.0));
        let r = slider_panels(tall, 2);
        assert_eq!(r[0].min.x, r[1].min.x);
        assert!(r[1].min.y > r[0].max.y);
        let short = Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 600.0));
        let r = slider_panels(short, 2);
        assert_eq!(r[0].min.y, r[1].min.y);
        assert_eq!(r[1].min.x - r[0].max.x, 8.0);
    }

    #[test]
    fn defaults_match_the_original() {
        let t = ToolSettings::default();
        assert_eq!(t.pencil_size(), 1);
        assert_eq!(t.eraser_size(), 8);
        assert_eq!(t.brush_size(BrushKind::Brush), 3);
    }
}
