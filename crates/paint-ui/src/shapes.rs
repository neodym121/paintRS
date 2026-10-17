//! The shape tool (docs/behavior/shapes.md): drawing a shape, the editable draft with a dashed
//! frame, eight handles and a rotation handle, curve bends, polygon clicks, and committing the
//! draft to the layer as one history step.

use egui::{Align2, Color32, CursorIcon, Pos2, Rect, Stroke, StrokeKind, Vec2};
use paint_core::draw::snap_45;
use paint_core::shape::{px_center, render, PaintStyle, ShapeGeom, ShapeKind, ShapeStyle};
use paint_core::{IRect, PixelRecorder};

use crate::canvas::CanvasGeom;
use crate::editor::Editor;
use crate::{fonts, icons};

/// Outline/fill/width chosen in the ribbon (defaults as in the original).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeSettings {
    pub outline: PaintStyle,
    pub fill: PaintStyle,
    pub width: u32,
}

impl Default for ShapeSettings {
    fn default() -> Self {
        Self { outline: PaintStyle::Solid, fill: PaintStyle::None, width: 5 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    /// The first drag is in progress.
    Drawing,
    /// A curve waits for its first bend (drag anywhere).
    Bend1,
    /// A curve waits for its second bend.
    Bend2,
    /// A polygon takes vertices by clicks until a click near its start closes it.
    PolygonOpen,
    /// Finished drawing: the shape can be moved, resized and rotated until committed.
    Editing,
}

#[derive(Clone, Debug)]
enum Grab {
    Move {
        last: [f32; 2],
    },
    Handle {
        index: usize,
        start: ShapeGeom,
        frame: Frame,
    },
    Rotate {
        start: ShapeGeom,
        center: [f32; 2],
        from: f32,
    },
    /// Moving the newest polygon vertex or a curve control point.
    Point,
}

/// The draft frame: centre, half size and rotation, in image coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Frame {
    center: [f32; 2],
    half: [f32; 2],
    angle: f32,
}

impl Frame {
    fn to_world(self, l: [f32; 2]) -> [f32; 2] {
        let (s, c) = self.angle.sin_cos();
        [self.center[0] + l[0] * c - l[1] * s, self.center[1] + l[0] * s + l[1] * c]
    }

    fn to_local(self, p: [f32; 2]) -> [f32; 2] {
        let (s, c) = self.angle.sin_cos();
        let (dx, dy) = (p[0] - self.center[0], p[1] - self.center[1]);
        [dx * c + dy * s, -dx * s + dy * c]
    }

    /// Local position of handle `i`: 0 top-left, then clockwise.
    fn handle_local(self, i: usize) -> [f32; 2] {
        let [hx, hy] = self.half;
        let (sx, sy) = HANDLE_SIGNS[i];
        [sx * hx, sy * hy]
    }
}

const HANDLE_SIGNS: [(f32, f32); 8] =
    [(-1.0, -1.0), (0.0, -1.0), (1.0, -1.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (-1.0, 1.0), (-1.0, 0.0)];

pub(crate) struct Draft {
    pub(crate) geom: ShapeGeom,
    /// Drawn with the right button: outline in color 2 and fill in color 1.
    secondary: bool,
    pub(crate) phase: Phase,
    grab: Option<Grab>,
    rec: PixelRecorder,
    rendered: Option<(ShapeGeom, ShapeStyle)>,
    seed: u64,
    /// Where the current drag started (Drawing).
    origin: [f32; 2],
}

/// Distance in points within which a click hits a handle.
const HIT: f32 = 7.0;
/// How far above the frame the rotation handle sits, in points.
const ROTATE_GAP: f32 = 26.0;

fn dist(a: Pos2, b: Pos2) -> f32 {
    (a - b).length()
}

impl Editor {
    pub(crate) fn shape_style(&self, secondary: bool) -> ShapeStyle {
        let (outline_color, fill_color) =
            if secondary { (self.color2, self.color1) } else { (self.color1, self.color2) };
        ShapeStyle {
            outline: self.shape_settings.outline,
            fill: self.shape_settings.fill,
            width: self.shape_settings.width,
            outline_color,
            fill_color,
            antialias: self.settings.antialias_shapes,
        }
    }

    fn draft_frame(&self) -> Option<Frame> {
        let d = self.draft.as_ref()?;
        if d.phase != Phase::Editing {
            return None;
        }
        Some(match &d.geom {
            ShapeGeom::Boxed { a, b, angle, .. } => Frame {
                center: [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0],
                half: [(a[0] - b[0]).abs() / 2.0, (a[1] - b[1]).abs() / 2.0],
                angle: *angle,
            },
            ShapeGeom::Line { .. } => return None,
            g => {
                let [x0, y0, x1, y1] = g.point_bounds();
                Frame {
                    center: [(x0 + x1) / 2.0, (y0 + y1) / 2.0],
                    half: [(x1 - x0) / 2.0, (y1 - y0) / 2.0],
                    angle: 0.0,
                }
            }
        })
    }

    /// Screen positions of the 8 handles and the rotation handle of the draft frame.
    fn frame_screen(&self, cg: &CanvasGeom, f: &Frame) -> ([Pos2; 8], Pos2, Pos2) {
        let to = |p: [f32; 2]| cg.to_screen(p[0], p[1]);
        let handles = std::array::from_fn(|i| to(f.to_world(f.handle_local(i))));
        let top = to(f.to_world([0.0, -f.half[1]]));
        let center = to(f.center);
        let up = (top - center).normalized();
        let up = if up.is_finite() && up.length() > 0.5 { up } else { Vec2::new(0.0, -1.0) };
        (handles, top, top + up * ROTATE_GAP)
    }

    /// Starts a new draft for the current shape kind at pixel `px`.
    fn begin_shape(&mut self, px: (i32, i32), secondary: bool) {
        let p = px_center(px.0, px.1);
        let geom = match self.shape {
            ShapeKind::Line => ShapeGeom::Line { a: p, b: p },
            ShapeKind::Curve => ShapeGeom::Curve { p: [p, p, p, p] },
            ShapeKind::Polygon => ShapeGeom::Polygon { pts: vec![p, p], closed: false },
            kind => ShapeGeom::Boxed { kind, a: p, b: p, angle: 0.0 },
        };
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(7, |d| d.as_nanos() as u64);
        self.draft = Some(Draft {
            geom,
            secondary,
            phase: Phase::Drawing,
            grab: None,
            rec: PixelRecorder::new(self.doc.active_id()),
            rendered: None,
            seed,
            origin: p,
        });
    }

    /// Pointer pressed on the canvas with the shape tool.
    pub(crate) fn shape_press(&mut self, cg: &CanvasGeom, pos: Pos2, px: (i32, i32), secondary: bool) {
        let p = px_center(px.0, px.1);
        let Some(phase) = self.draft.as_ref().map(|d| d.phase) else {
            self.begin_shape(px, secondary);
            return;
        };
        match phase {
            Phase::Drawing => return,
            Phase::Bend1 | Phase::Bend2 => {
                if let Some(d) = self.draft.as_mut() {
                    if let ShapeGeom::Curve { p: cp } = &mut d.geom {
                        if phase == Phase::Bend1 {
                            cp[1] = p;
                        }
                        cp[2] = p;
                    }
                    d.grab = Some(Grab::Point);
                }
                return;
            }
            Phase::PolygonOpen => {
                let close = self.draft.as_ref().is_some_and(|d| match &d.geom {
                    ShapeGeom::Polygon { pts, .. } => {
                        pts.len() >= 3 && dist(cg.to_screen(pts[0][0], pts[0][1]), pos) <= HIT
                    }
                    _ => false,
                });
                if let Some(d) = self.draft.as_mut() {
                    if let ShapeGeom::Polygon { pts, closed } = &mut d.geom {
                        if close {
                            *closed = true;
                        } else {
                            pts.push(p);
                            d.grab = Some(Grab::Point);
                        }
                    }
                }
                if close {
                    self.commit_draft();
                }
                return;
            }
            Phase::Editing => {}
        }
        match self.editing_grab(cg, pos, p) {
            Some(g) => {
                if let Some(d) = self.draft.as_mut() {
                    d.grab = Some(g);
                }
            }
            None => {
                // A press outside the draft commits it and starts a new shape.
                self.commit_draft();
                self.begin_shape(px, secondary);
            }
        }
    }

    /// What a press at `pos` grabs on the draft being edited (None: outside it).
    fn editing_grab(&self, cg: &CanvasGeom, pos: Pos2, p: [f32; 2]) -> Option<Grab> {
        let d = self.draft.as_ref()?;
        if let ShapeGeom::Line { a, b } = &d.geom {
            for (i, q) in [*a, *b].into_iter().enumerate() {
                if dist(cg.to_screen(q[0], q[1]), pos) <= HIT {
                    let frame = Frame { center: q, half: [0.0, 0.0], angle: 0.0 };
                    return Some(Grab::Handle { index: i, start: d.geom.clone(), frame });
                }
            }
            return line_hit(cg, *a, *b, pos, self.shape_settings.width).then_some(Grab::Move { last: p });
        }
        let f = self.draft_frame()?;
        let (handles, _, rot) = self.frame_screen(cg, &f);
        if dist(rot, pos) <= HIT + 3.0 {
            let from = (p[1] - f.center[1]).atan2(p[0] - f.center[0]);
            return Some(Grab::Rotate { start: d.geom.clone(), center: f.center, from });
        }
        if let Some(i) = (0..8).find(|&i| dist(handles[i], pos) <= HIT) {
            return Some(Grab::Handle { index: i, start: d.geom.clone(), frame: f });
        }
        let l = f.to_local(p);
        (l[0].abs() <= f.half[0] + 1.0 && l[1].abs() <= f.half[1] + 1.0).then_some(Grab::Move { last: p })
    }

    /// Pointer moved while the button is down.
    pub(crate) fn shape_drag(&mut self, px: (i32, i32), shift: bool) {
        let p = px_center(px.0, px.1);
        let Some(d) = self.draft.as_mut() else { return };
        if d.phase == Phase::Drawing {
            let o = d.origin;
            match &mut d.geom {
                ShapeGeom::Boxed { b, .. } => {
                    *b = if shift { square(o, p) } else { p };
                }
                ShapeGeom::Line { b, .. } => *b = if shift { snap(o, p) } else { p },
                ShapeGeom::Curve { p: cp } => {
                    let e = if shift { snap(o, p) } else { p };
                    cp[2] = e;
                    cp[3] = e;
                    cp[1] = o;
                }
                ShapeGeom::Polygon { pts, .. } => {
                    let e = if shift { snap(o, p) } else { p };
                    if let Some(last) = pts.last_mut() {
                        *last = e;
                    }
                }
            }
            return;
        }
        let Some(grab) = d.grab.clone() else { return };
        match grab {
            Grab::Point => match (&mut d.geom, d.phase) {
                (ShapeGeom::Curve { p: cp }, Phase::Bend1) => {
                    cp[1] = p;
                    cp[2] = p;
                }
                (ShapeGeom::Curve { p: cp }, _) => cp[2] = p,
                (ShapeGeom::Polygon { pts, .. }, _) => {
                    let prev = pts[pts.len().saturating_sub(2)];
                    if let Some(last) = pts.last_mut() {
                        *last = if shift { snap(prev, p) } else { p };
                    }
                }
                _ => {}
            },
            Grab::Move { last } => {
                d.geom.translate(p[0] - last[0], p[1] - last[1]);
                d.grab = Some(Grab::Move { last: p });
            }
            Grab::Rotate { start, center, from } => {
                let to = (p[1] - center[1]).atan2(p[0] - center[0]);
                d.geom = rotated(&start, center, to - from);
            }
            Grab::Handle { index, start, frame } => {
                d.geom = match &start {
                    ShapeGeom::Line { a, b } => {
                        let (fixed, moving) = if index == 0 { (*b, p) } else { (*a, p) };
                        let moving = if shift { snap(fixed, moving) } else { moving };
                        if index == 0 {
                            ShapeGeom::Line { a: moving, b: fixed }
                        } else {
                            ShapeGeom::Line { a: fixed, b: moving }
                        }
                    }
                    g => resized(g, &frame, index, frame.to_local(p)),
                };
            }
        }
    }

    /// Pointer released.
    pub(crate) fn shape_release(&mut self) {
        let Some((phase, degenerate, kind)) =
            self.draft.as_ref().map(|d| (d.phase, d.geom.is_degenerate(), d.geom.kind()))
        else {
            return;
        };
        if phase == Phase::Drawing && degenerate {
            // A click without a drag draws nothing.
            self.cancel_draft();
            return;
        }
        if let Some(d) = self.draft.as_mut() {
            d.grab = None;
            d.phase = match (phase, kind) {
                (Phase::Drawing, ShapeKind::Curve) => Phase::Bend1,
                (Phase::Drawing, ShapeKind::Polygon) => Phase::PolygonOpen,
                (Phase::Drawing, _) => Phase::Editing,
                (Phase::Bend1, _) => Phase::Bend2,
                (Phase::Bend2, _) => Phase::Editing,
                (other, _) => other,
            };
        }
    }

    /// Writes the draft into the layer as one history step.
    pub(crate) fn commit_draft(&mut self) {
        self.refresh_draft();
        if let Some(d) = self.draft.take() {
            self.history.push_pixels(&self.doc, "shape", d.rec);
        }
    }

    pub(crate) fn cancel_draft(&mut self) {
        if let Some(d) = self.draft.take() {
            d.rec.revert(&mut self.doc);
        }
    }

    /// Re-renders the draft when its geometry or style changed (colors, outline, fill and width
    /// apply to the draft live, like in the original).
    pub(crate) fn refresh_draft(&mut self) {
        let Some(secondary) = self.draft.as_ref().map(|d| d.secondary) else { return };
        let style = self.shape_style(secondary);
        let canvas = (self.doc.width(), self.doc.height());
        let Some(d) = self.draft.as_mut() else { return };
        if d.rendered.as_ref().is_some_and(|(g, s)| *g == d.geom && *s == style) {
            return;
        }
        d.rec.revert(&mut self.doc);
        if let Some((src, r)) = render(&d.geom, &style, canvas, d.seed) {
            d.rec.touch(&self.doc, r);
            if let Some(raster) = self.doc.raster_mut(d.rec.layer()) {
                raster.draw_over(&src, r.x, r.y, 255, IRect::new(0, 0, canvas.0, canvas.1));
            }
            self.doc.mark_dirty(r);
        }
        d.rendered = Some((d.geom.clone(), style));
    }

    /// Cursor for the shape tool at `pos` (handles and the inside of the draft frame).
    pub(crate) fn shape_cursor(&self, cg: &CanvasGeom, pos: Pos2) -> CursorIcon {
        if let Some(ShapeGeom::Line { a, b }) =
            self.draft.as_ref().filter(|d| d.phase == Phase::Editing).map(|d| &d.geom)
        {
            if [*a, *b].iter().any(|q| dist(cg.to_screen(q[0], q[1]), pos) <= HIT) {
                return CursorIcon::Crosshair;
            }
            if line_hit(cg, *a, *b, pos, self.shape_settings.width) {
                return CursorIcon::Move;
            }
        }
        let Some(f) = self.draft_frame() else { return CursorIcon::Crosshair };
        let (handles, _, rot) = self.frame_screen(cg, &f);
        if dist(rot, pos) <= HIT + 3.0 {
            return CursorIcon::Grab;
        }
        if let Some(i) = (0..8).find(|&i| dist(handles[i], pos) <= HIT) {
            // Pick the resize cursor closest to the handle's on-screen direction.
            let c = cg.to_screen(f.center[0], f.center[1]);
            let v = handles[i] - c;
            let a = v.y.atan2(v.x).to_degrees().rem_euclid(180.0);
            return match a {
                a if !(22.5..157.5).contains(&a) => CursorIcon::ResizeHorizontal,
                a if a < 67.5 => CursorIcon::ResizeNwSe,
                a if a < 112.5 => CursorIcon::ResizeVertical,
                _ => CursorIcon::ResizeNeSw,
            };
        }
        let l = f.to_local(cg.to_image_f(pos));
        if l[0].abs() <= f.half[0] && l[1].abs() <= f.half[1] {
            return CursorIcon::Move;
        }
        CursorIcon::Crosshair
    }

    /// Dashed frame, handles and rotation handle of the draft.
    pub(crate) fn draw_draft_overlay(&self, ui: &egui::Ui, cg: &CanvasGeom) {
        let Some(d) = self.draft.as_ref() else { return };
        let painter = ui.painter_at(cg.viewport);
        let p = self.palette(ui.ctx());
        let handle = |c: Pos2| {
            let r = Rect::from_center_size(c, Vec2::splat(7.0));
            painter.rect_filled(r, 1.0, Color32::WHITE);
            painter.rect_stroke(r, 1.0, Stroke::new(1.0, Color32::from_gray(50)), StrokeKind::Inside);
        };
        if let (ShapeGeom::Line { a, b }, Phase::Editing) = (&d.geom, d.phase) {
            handle(cg.to_screen(a[0], a[1]));
            handle(cg.to_screen(b[0], b[1]));
            return;
        }
        if let (ShapeGeom::Polygon { pts, .. }, Phase::PolygonOpen) = (&d.geom, d.phase) {
            // Show where a click closes the polygon.
            painter.circle_stroke(cg.to_screen(pts[0][0], pts[0][1]), HIT, Stroke::new(1.0, p.accent));
            return;
        }
        let Some(f) = self.draft_frame() else { return };
        let (handles, top, rot) = self.frame_screen(cg, &f);
        let corners = [handles[0], handles[2], handles[4], handles[6], handles[0]];
        for w in corners.windows(2) {
            painter.extend(egui::Shape::dashed_line(
                &[w[0], w[1]],
                Stroke::new(1.0, Color32::WHITE),
                4.0,
                4.0,
            ));
            painter.extend(egui::Shape::dashed_line_with_offset(
                &[w[0], w[1]],
                Stroke::new(1.0, Color32::from_gray(30)),
                &[4.0],
                &[4.0],
                4.0,
            ));
        }
        painter.line_segment([top, rot], Stroke::new(1.0, Color32::from_gray(90)));
        painter.circle_filled(rot, 10.0, p.panel);
        painter.circle_stroke(rot, 10.0, Stroke::new(1.0, p.border));
        painter.text(rot, Align2::CENTER_CENTER, icons::ROTATE_CW, fonts::icon(12.0), p.text);
        for h in handles {
            handle(h);
        }
    }
}

/// Square with the smaller side of the drag (Shift), keeping the drag direction.
fn square(o: [f32; 2], p: [f32; 2]) -> [f32; 2] {
    let (dx, dy) = (p[0] - o[0], p[1] - o[1]);
    let s = dx.abs().min(dy.abs());
    [o[0] + s * dx.signum(), o[1] + s * dy.signum()]
}

/// 45° snapping in pixel units.
fn snap(o: [f32; 2], p: [f32; 2]) -> [f32; 2] {
    let (x, y) = snap_45(o[0].floor() as i32, o[1].floor() as i32, p[0].floor() as i32, p[1].floor() as i32);
    px_center(x, y)
}

fn line_hit(cg: &CanvasGeom, a: [f32; 2], b: [f32; 2], pos: Pos2, width: u32) -> bool {
    let (sa, sb) = (cg.to_screen(a[0], a[1]), cg.to_screen(b[0], b[1]));
    let ab = sb - sa;
    let t = ((pos - sa).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    dist(sa + ab * t, pos) <= (width as f32 * cg.scale / 2.0).max(HIT)
}

fn rotated(g: &ShapeGeom, c: [f32; 2], delta: f32) -> ShapeGeom {
    let rot = |p: [f32; 2]| {
        let (s, co) = delta.sin_cos();
        let (dx, dy) = (p[0] - c[0], p[1] - c[1]);
        [c[0] + dx * co - dy * s, c[1] + dx * s + dy * co]
    };
    match g {
        ShapeGeom::Boxed { kind, a, b, angle } => {
            ShapeGeom::Boxed { kind: *kind, a: *a, b: *b, angle: angle + delta }
        }
        ShapeGeom::Line { a, b } => ShapeGeom::Line { a: rot(*a), b: rot(*b) },
        ShapeGeom::Curve { p } => ShapeGeom::Curve { p: p.map(rot) },
        ShapeGeom::Polygon { pts, closed } => {
            ShapeGeom::Polygon { pts: pts.iter().map(|&q| rot(q)).collect(), closed: *closed }
        }
    }
}

/// Geometry after dragging handle `index` of `frame` to local position `m`.
fn resized(g: &ShapeGeom, frame: &Frame, index: usize, m: [f32; 2]) -> ShapeGeom {
    let [hx, hy] = frame.half;
    let (sx, sy) = HANDLE_SIGNS[index];
    let (lx0, lx1) = match sx as i32 {
        -1 => (m[0], hx),
        1 => (-hx, m[0]),
        _ => (-hx, hx),
    };
    let (ly0, ly1) = match sy as i32 {
        -1 => (m[1], hy),
        1 => (-hy, m[1]),
        _ => (-hy, hy),
    };
    match g {
        ShapeGeom::Boxed { kind, angle, .. } => {
            let c = frame.to_world([(lx0 + lx1) / 2.0, (ly0 + ly1) / 2.0]);
            let (w, h) = ((lx1 - lx0).abs() / 2.0, (ly1 - ly0).abs() / 2.0);
            ShapeGeom::Boxed { kind: *kind, a: [c[0] - w, c[1] - h], b: [c[0] + w, c[1] + h], angle: *angle }
        }
        other => {
            // Point shapes: map every point from the old frame onto the new one.
            let map = |p: [f32; 2]| {
                let l = frame.to_local(p);
                let tx = if hx > 0.0 { (l[0] + hx) / (2.0 * hx) } else { 0.5 };
                let ty = if hy > 0.0 { (l[1] + hy) / (2.0 * hy) } else { 0.5 };
                frame.to_world([lx0 + tx * (lx1 - lx0), ly0 + ty * (ly1 - ly0)])
            };
            match other {
                ShapeGeom::Curve { p } => ShapeGeom::Curve { p: p.map(map) },
                ShapeGeom::Polygon { pts, closed } => {
                    ShapeGeom::Polygon { pts: pts.iter().map(|&q| map(q)).collect(), closed: *closed }
                }
                g => g.clone(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_square_uses_the_smaller_side() {
        assert_eq!(square([0.0, 0.0], [80.0, 30.0]), [30.0, 30.0]);
        assert_eq!(square([10.0, 10.0], [-20.0, 60.0]), [-20.0, 40.0]);
    }

    #[test]
    fn corner_handle_keeps_the_opposite_corner() {
        let g = ShapeGeom::Boxed { kind: ShapeKind::Rectangle, a: [0.0, 0.0], b: [100.0, 50.0], angle: 0.0 };
        let f = Frame { center: [50.0, 25.0], half: [50.0, 25.0], angle: 0.0 };
        // Drag the bottom-right handle to (150, 75).
        let r = resized(&g, &f, 4, f.to_local([150.0, 75.0]));
        let ShapeGeom::Boxed { a, b, .. } = r else { panic!() };
        assert_eq!((a, b), ([0.0, 0.0], [150.0, 75.0]));
        // Rotated frame: the opposite corner stays where it was on screen.
        let f = Frame { center: [50.0, 25.0], half: [50.0, 25.0], angle: 0.5 };
        let fixed = f.to_world(f.handle_local(0));
        let g = ShapeGeom::Boxed { kind: ShapeKind::Rectangle, a: [0.0, 0.0], b: [100.0, 50.0], angle: 0.5 };
        let r = resized(&g, &f, 4, [70.0, 40.0]);
        let ShapeGeom::Boxed { a, b, angle, .. } = r else { panic!() };
        let nf = Frame {
            center: [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0],
            half: [(b[0] - a[0]).abs() / 2.0, (b[1] - a[1]).abs() / 2.0],
            angle,
        };
        let now = nf.to_world(nf.handle_local(0));
        assert!((now[0] - fixed[0]).abs() < 1e-3 && (now[1] - fixed[1]).abs() < 1e-3);
    }
}
