//! Canvas view: zoom/scroll state, the workspace around the canvas, GPU drawing and handles.

pub mod gpu;
pub mod mips;

use egui::{Color32, CursorIcon, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use paint_core::zoom;

use crate::settings::Smoothing;
use crate::theme::Palette;
use gpu::{CanvasCallback, Uniforms};

/// Free space around the canvas when it is larger than the viewport (keeps handles reachable).
pub const MARGIN: f32 = 12.0;
pub const SCROLLBAR: f32 = 12.0;
pub const HANDLE: f32 = 6.0;
const CHECKER_A: [f32; 4] = [0.82, 0.82, 0.82, 1.0];
const CHECKER_B: [f32; 4] = [0.71, 0.71, 0.71, 1.0];
const CHECKER_PT: f32 = 8.0;

#[derive(Clone, Debug)]
pub struct CanvasView {
    /// Image pixels → physical screen pixels (1.0 = 100%: one image pixel per device pixel).
    pub zoom: f32,
    /// Scroll offset of the content, in points.
    pub scroll: Vec2,
    /// Viewport of the last frame (for "fit to window" and keyboard zoom).
    pub last_viewport: Rect,
    pan_anchor: Option<(Pos2, Vec2)>,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self { zoom: 1.0, scroll: Vec2::ZERO, last_viewport: Rect::NOTHING, pan_anchor: None }
    }
}

/// Geometry of one frame: where the canvas is on screen.
#[derive(Clone, Copy, Debug)]
pub struct CanvasGeom {
    pub viewport: Rect,
    pub canvas: Rect,
    /// Points per image pixel.
    pub scale: f32,
    pub image: (u32, u32),
}

impl CanvasGeom {
    pub fn to_image(&self, p: Pos2) -> (f32, f32) {
        ((p.x - self.canvas.min.x) / self.scale, (p.y - self.canvas.min.y) / self.scale)
    }

    pub fn to_image_f(&self, p: Pos2) -> [f32; 2] {
        let (x, y) = self.to_image(p);
        [x, y]
    }

    pub fn to_screen(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(self.canvas.min.x + x * self.scale, self.canvas.min.y + y * self.scale)
    }

    /// Pixel under the point, if it is inside the image.
    pub fn pixel_at(&self, p: Pos2) -> Option<(i32, i32)> {
        let (x, y) = self.to_image(p);
        let (xi, yi) = (x.floor() as i32, y.floor() as i32);
        (xi >= 0 && yi >= 0 && xi < self.image.0 as i32 && yi < self.image.1 as i32).then_some((xi, yi))
    }
}

impl CanvasView {
    fn content_size(&self, image: (u32, u32), ppp: f32) -> Vec2 {
        let s = self.zoom / ppp;
        Vec2::new(image.0 as f32 * s, image.1 as f32 * s)
    }

    fn clamp_scroll(&mut self, viewport: Rect, image: (u32, u32), ppp: f32) {
        let c = self.content_size(image, ppp) + Vec2::splat(2.0 * MARGIN);
        let max = (c - viewport.size()).max(Vec2::ZERO);
        self.scroll = self.scroll.clamp(Vec2::ZERO, max);
    }

    pub fn geometry(&mut self, viewport: Rect, image: (u32, u32), ppp: f32) -> CanvasGeom {
        self.clamp_scroll(viewport, image, ppp);
        let size = self.content_size(image, ppp);
        let mut min = Pos2::ZERO;
        for axis in 0..2 {
            let room = viewport.size()[axis];
            let need = size[axis] + 2.0 * MARGIN;
            min[axis] = if need <= room {
                viewport.center()[axis] - size[axis] / 2.0
            } else {
                viewport.min[axis] + MARGIN - self.scroll[axis]
            };
            // Snap to device pixels so 100% is exactly 1:1.
            min[axis] = (min[axis] * ppp).round() / ppp;
        }
        CanvasGeom { viewport, canvas: Rect::from_min_size(min, size), scale: self.zoom / ppp, image }
    }

    /// Sets a new zoom keeping the image point under `anchor` (screen point) in place.
    pub fn zoom_to(&mut self, new_zoom: f32, anchor: Option<Pos2>, image: (u32, u32), ppp: f32) {
        let new_zoom = zoom::clamp(new_zoom);
        let viewport = self.last_viewport;
        if !viewport.is_positive() {
            self.zoom = new_zoom;
            return;
        }
        let anchor = anchor.filter(|a| viewport.contains(*a)).unwrap_or(viewport.center());
        let g = self.geometry(viewport, image, ppp);
        let (ix, iy) = g.to_image(anchor);
        self.zoom = new_zoom;
        let s = new_zoom / ppp;
        // origin = viewport.min + MARGIN - scroll  ⇒  scroll = viewport.min + MARGIN - (anchor - img*s)
        self.scroll = Vec2::new(
            viewport.min.x + MARGIN - (anchor.x - ix * s),
            viewport.min.y + MARGIN - (anchor.y - iy * s),
        );
        self.clamp_scroll(viewport, image, ppp);
    }

    pub fn fit(&mut self, image: (u32, u32), ppp: f32) {
        let v = self.last_viewport;
        if !v.is_positive() {
            return;
        }
        let avail = (v.width() * ppp, v.height() * ppp);
        self.zoom = zoom::fit((image.0 as f32, image.1 as f32), avail, 50.0 * ppp);
        self.scroll = Vec2::ZERO;
    }
}

pub struct CanvasDraw<'a> {
    pub palette: &'a Palette,
    pub smoothing: Smoothing,
    pub srgb_target: bool,
    pub gpu_ready: bool,
}

/// Lays out and draws the workspace with the canvas; handles scrolling/zooming input.
/// Returns the geometry and the response covering the viewport (for tool input).
pub fn show(
    ui: &mut egui::Ui,
    view: &mut CanvasView,
    image: (u32, u32),
    draw: &CanvasDraw<'_>,
) -> (CanvasGeom, egui::Response) {
    let ppp = ui.ctx().pixels_per_point();
    let full = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(full, Sense::click_and_drag());
    let painter = ui.painter_at(full);
    painter.rect_filled(full, 0.0, draw.palette.workspace);

    // Room for scrollbars when the content does not fit.
    let content = view.content_size(image, ppp) + Vec2::splat(2.0 * MARGIN);
    let need_h = content.x > full.width();
    let need_v = content.y > full.height();
    let mut viewport = full;
    if need_h {
        viewport.max.y -= SCROLLBAR;
    }
    if need_v {
        viewport.max.x -= SCROLLBAR;
    }
    view.last_viewport = viewport;

    handle_navigation(ui, view, &response, viewport, image, ppp);
    let g = view.geometry(viewport, image, ppp);

    // Canvas pixels (GPU).
    let visible = g.canvas.intersect(viewport);
    if draw.gpu_ready && visible.is_positive() {
        let (nearest, grid) = match draw.smoothing {
            Smoothing::Auto => (view.zoom >= 8.0, view.zoom >= 8.0),
            Smoothing::Always => (false, false),
            Smoothing::Never => (true, view.zoom >= 8.0),
        };
        let grid_color = if draw.palette.dark { [0.5, 0.5, 0.5, 1.0] } else { [0.45, 0.45, 0.45, 1.0] };
        let uniforms = Uniforms {
            checker_a: CHECKER_A,
            checker_b: CHECKER_B,
            grid_color,
            canvas_origin: [g.canvas.min.x * ppp, g.canvas.min.y * ppp],
            canvas_size: [g.canvas.width() * ppp, g.canvas.height() * ppp],
            tex_size: [image.0 as f32, image.1 as f32],
            checker_px: (CHECKER_PT * ppp).round(),
            grid_alpha: if grid { 0.35 } else { 0.0 },
            srgb_target: if draw.srgb_target { 1.0 } else { 0.0 },
            _pad: [0.0; 3],
        };
        painter.add(egui_wgpu::Callback::new_paint_callback(visible, CanvasCallback { uniforms, nearest }));
    }

    // Frame and the eight resize handles (just outside the edges).
    let p = draw.palette;
    painter.rect_stroke(g.canvas.expand(0.5), 0.0, Stroke::new(1.0, p.canvas_frame), StrokeKind::Middle);
    for c in handle_centers(g.canvas) {
        let r = Rect::from_center_size(c, Vec2::splat(HANDLE));
        painter.rect_filled(r, 0.0, Color32::WHITE);
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::from_gray(40)), StrokeKind::Inside);
    }

    draw_scrollbars(ui, view, full, viewport, need_h, need_v, image, ppp, p);
    (g, response)
}

/// Centers of the 8 handles: corners and edge midpoints, offset outward by half a handle.
pub fn handle_centers(r: Rect) -> [Pos2; 8] {
    let o = HANDLE / 2.0;
    let (x0, x1, xm) = (r.min.x - o, r.max.x + o, r.center().x);
    let (y0, y1, ym) = (r.min.y - o, r.max.y + o, r.center().y);
    [
        Pos2::new(x0, y0),
        Pos2::new(xm, y0),
        Pos2::new(x1, y0),
        Pos2::new(x1, ym),
        Pos2::new(x1, y1),
        Pos2::new(xm, y1),
        Pos2::new(x0, y1),
        Pos2::new(x0, ym),
    ]
}

fn handle_navigation(
    ui: &egui::Ui,
    view: &mut CanvasView,
    response: &egui::Response,
    viewport: Rect,
    image: (u32, u32),
    ppp: f32,
) {
    let hovered = response.hovered() || response.contains_pointer();
    if hovered {
        let (wheel, pinch, pointer) = ui.input(|i| {
            (wheel_input(i, viewport), i.multi_touch().map(|t| t.zoom_delta), i.pointer.hover_pos())
        });
        if let Some(z) = pinch.filter(|z| (z - 1.0).abs() > f32::EPSILON) {
            // Touchpad / touch screen pinch.
            view.zoom_to(view.zoom * z, pointer, image, ppp);
        } else if let Some((delta, mods)) = wheel {
            if mods.command {
                // Ctrl + wheel: one zoom step per notch, anchored at the cursor.
                let z = if delta.y > 0.0 { zoom::zoom_in(view.zoom) } else { zoom::zoom_out(view.zoom) };
                if delta.y != 0.0 {
                    view.zoom_to(z, pointer, image, ppp);
                }
            } else {
                // Wheel scrolls vertically, Shift + wheel horizontally.
                let d = if mods.shift && delta.x == 0.0 { Vec2::new(delta.y, 0.0) } else { delta };
                view.scroll -= d;
                view.clamp_scroll(viewport, image, ppp);
            }
        }
    }

    // Middle-button drag or Space + drag pans the view.
    let space = ui.input(|i| i.key_down(egui::Key::Space));
    let pan_button = response.dragged_by(egui::PointerButton::Middle)
        || (space && response.dragged_by(egui::PointerButton::Primary));
    if pan_button {
        if let Some(pos) = response.interact_pointer_pos() {
            let (start, scroll0) = *view.pan_anchor.get_or_insert((pos, view.scroll));
            view.scroll = scroll0 - (pos - start);
            view.clamp_scroll(viewport, image, ppp);
        }
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    } else {
        view.pan_anchor = None;
        if space && hovered {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }
    }
}

/// Mouse wheel movement of this frame in points (lines/pages converted) and its modifiers.
pub fn wheel_input(i: &egui::InputState, viewport: Rect) -> Option<(Vec2, egui::Modifiers)> {
    let mut total = Vec2::ZERO;
    let mut mods = None;
    for e in &i.events {
        if let egui::Event::MouseWheel { unit, delta, modifiers, .. } = e {
            let k = match unit {
                egui::MouseWheelUnit::Point => Vec2::splat(1.0),
                egui::MouseWheelUnit::Line => Vec2::splat(48.0),
                egui::MouseWheelUnit::Page => viewport.size(),
            };
            total += *delta * k;
            // Modifier state of the event, or of the keyboard right now (injected input can
            // deliver the wheel event before the modifier change is applied to it).
            mods = Some(*modifiers | i.modifiers);
        }
    }
    mods.map(|m| (total, m))
}

#[allow(clippy::too_many_arguments)]
fn draw_scrollbars(
    ui: &mut egui::Ui,
    view: &mut CanvasView,
    full: Rect,
    viewport: Rect,
    need_h: bool,
    need_v: bool,
    image: (u32, u32),
    ppp: f32,
    p: &Palette,
) {
    let content = view.content_size(image, ppp) + Vec2::splat(2.0 * MARGIN);
    for axis in 0..2 {
        let needed = if axis == 0 { need_h } else { need_v };
        if !needed {
            continue;
        }
        let track = if axis == 0 {
            Rect::from_min_max(Pos2::new(full.min.x, viewport.max.y), Pos2::new(viewport.max.x, full.max.y))
        } else {
            Rect::from_min_max(Pos2::new(viewport.max.x, full.min.y), Pos2::new(full.max.x, viewport.max.y))
        };
        let len = track.size()[axis];
        let visible = viewport.size()[axis];
        let total = content[axis].max(visible);
        let thumb_len = (len * visible / total).max(24.0).min(len);
        let max_scroll = (total - visible).max(1.0);
        let pos = (view.scroll[axis] / max_scroll).clamp(0.0, 1.0) * (len - thumb_len);
        let thumb = if axis == 0 {
            Rect::from_min_size(
                Pos2::new(track.min.x + pos, track.min.y + 3.0),
                Vec2::new(thumb_len, track.height() - 6.0),
            )
        } else {
            Rect::from_min_size(
                Pos2::new(track.min.x + 3.0, track.min.y + pos),
                Vec2::new(track.width() - 6.0, thumb_len),
            )
        };
        let id = egui::Id::unique(("canvas_scrollbar", axis));
        let resp = ui.interact(track, id, Sense::click_and_drag());
        if resp.dragged() {
            let d = resp.drag_delta()[axis];
            view.scroll[axis] += d * total / len;
        } else if resp.clicked() {
            if let Some(ptr) = resp.interact_pointer_pos() {
                let page = visible * 0.9;
                view.scroll[axis] += if ptr[axis] < thumb.center()[axis] { -page } else { page };
            }
        }
        view.clamp_scroll(viewport, image, ppp);
        let painter = ui.painter();
        painter.rect_filled(track, 0.0, p.chrome);
        let color = if resp.hovered() || resp.dragged() { p.text_muted } else { p.border };
        painter.rect_filled(thumb, 3.0, color);
    }
}
