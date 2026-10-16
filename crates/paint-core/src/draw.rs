//! Pixel tools of stage 2: pencil, eraser and bucket fill (docs/behavior/tools.md).
//! All of them are aliased, like the original.

use crate::color::Px;
use crate::geom::IRect;
use crate::raster::Raster;

/// Calls `f` for every pixel of the 8-connected line from (x0, y0) to (x1, y1), both ends included.
pub fn line_points(x0: i32, y0: i32, x1: i32, y1: i32, mut f: impl FnMut(i32, i32)) {
    let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    let (mut x, mut y, mut err) = (x0, y0, dx + dy);
    loop {
        f(x, y);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Snaps (x1, y1) so that the segment from (x0, y0) is at a multiple of 45° (Shift).
pub fn snap_45(x0: i32, y0: i32, x1: i32, y1: i32) -> (i32, i32) {
    let (dx, dy) = ((x1 - x0) as f32, (y1 - y0) as f32);
    let len = (dx * dx + dy * dy).sqrt();
    if len == 0.0 {
        return (x1, y1);
    }
    let a = dy.atan2(dx);
    let step = std::f32::consts::FRAC_PI_4;
    let snapped = (a / step).round() * step;
    let (c, s) = (snapped.cos(), snapped.sin());
    if c.abs() < 1e-3 {
        (x0, y1)
    } else if s.abs() < 1e-3 {
        (x1, y0)
    } else {
        // Diagonal: keep the larger of |dx|, |dy|.
        let m = dx.abs().max(dy.abs());
        (x0 + (m * c.signum()) as i32, y0 + (m * s.signum()) as i32)
    }
}

/// Pixels already painted by the current stroke, so semi-transparent strokes do not darken
/// where the stroke overlaps itself.
pub struct StrokeMask {
    w: u32,
    h: u32,
    bits: Vec<u64>,
}

impl StrokeMask {
    pub fn new(w: u32, h: u32) -> Self {
        Self { w, h, bits: vec![0; (w as usize * h as usize).div_ceil(64)] }
    }

    pub fn clear(&mut self) {
        self.bits.fill(0);
    }

    /// Marks the pixel; returns true if it was not marked before.
    #[inline]
    pub fn mark(&mut self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return false;
        }
        let i = y as usize * self.w as usize + x as usize;
        let (word, bit) = (i / 64, 1u64 << (i % 64));
        let fresh = self.bits[word] & bit == 0;
        self.bits[word] |= bit;
        fresh
    }

    #[inline]
    pub fn is_marked(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return true;
        }
        let i = y as usize * self.w as usize + x as usize;
        self.bits[i / 64] & (1u64 << (i % 64)) != 0
    }
}

/// Pixels of a round aliased stamp of diameter `size` centered on pixel (cx, cy).
pub fn disc_offsets(size: u32) -> Vec<(i32, i32)> {
    let r = size as f32 / 2.0;
    let reach = r.ceil() as i32;
    // Even sizes are centered on the pixel's corner, odd sizes on its center.
    let c = if size % 2 == 0 { 0.0 } else { 0.5 };
    let mut out = Vec::new();
    for y in -reach..=reach {
        for x in -reach..=reach {
            let (px, py) = (x as f32 + 0.5 - c, y as f32 + 0.5 - c);
            if px * px + py * py <= r * r {
                out.push((x, y));
            }
        }
    }
    out
}

/// A pencil stroke: 1 px Bresenham lines or a round stamp, composited once per pixel.
pub struct PencilStroke {
    color: Px,
    size: u32,
    stamp: Vec<(i32, i32)>,
    last: Option<(i32, i32)>,
    mask: StrokeMask,
}

impl PencilStroke {
    /// `color` is premultiplied and already includes the tool opacity.
    pub fn new(color: Px, size: u32, canvas: (u32, u32)) -> Self {
        let size = size.max(1);
        let stamp = if size > 1 { disc_offsets(size) } else { vec![(0, 0)] };
        Self { color, size, stamp, last: None, mask: StrokeMask::new(canvas.0, canvas.1) }
    }

    /// Forgets painted pixels (used when Shift redraws the stroke as a straight line).
    pub fn restart(&mut self) {
        self.mask.clear();
        self.last = None;
    }

    /// Bounding box of a segment from the last point to (x, y) including the stamp.
    pub fn segment_bounds(&self, x: i32, y: i32) -> IRect {
        let (lx, ly) = self.last.unwrap_or((x, y));
        let r = self.size as i32 / 2 + 1;
        IRect::from_corners(lx.min(x) - r, ly.min(y) - r, lx.max(x) + r + 1, ly.max(y) + r + 1)
    }

    /// Extends the stroke to (x, y) and paints into `raster`.
    pub fn line_to(&mut self, raster: &mut Raster, x: i32, y: i32) {
        let (lx, ly) = self.last.unwrap_or((x, y));
        let color = self.color;
        let (stamp, mask) = (&self.stamp, &mut self.mask);
        line_points(lx, ly, x, y, |px, py| {
            for &(ox, oy) in stamp {
                let (qx, qy) = (px + ox, py + oy);
                if mask.mark(qx, qy) {
                    if let Some(old) = raster.get(qx, qy) {
                        raster.put(qx, qy, old.over(color));
                    }
                }
            }
        });
        self.last = Some((x, y));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EraseMode {
    /// Left button: make pixels transparent.
    Clear,
    /// Right button: replace pixels equal to `from` with `to` (both premultiplied).
    Replace { from: Px, to: Px },
}

/// Square eraser swept along the pointer path.
pub struct EraserStroke {
    size: u32,
    mode: EraseMode,
    last: Option<(i32, i32)>,
}

impl EraserStroke {
    pub fn new(size: u32, mode: EraseMode) -> Self {
        Self { size: size.max(1), mode, last: None }
    }

    pub fn mode(&self) -> EraseMode {
        self.mode
    }

    /// Forgets the last point (used when Shift redraws the stroke as a straight line).
    pub fn restart(&mut self) {
        self.last = None;
    }

    /// Square covered when the pointer is on pixel (x, y).
    pub fn square_at(&self, x: i32, y: i32) -> IRect {
        let s = self.size as i32;
        IRect::new(x - (s - 1) / 2, y - (s - 1) / 2, self.size, self.size)
    }

    pub fn segment_bounds(&self, x: i32, y: i32) -> IRect {
        let (lx, ly) = self.last.unwrap_or((x, y));
        self.square_at(lx, ly).union(&self.square_at(x, y))
    }

    pub fn line_to(&mut self, raster: &mut Raster, x: i32, y: i32) {
        let (lx, ly) = self.last.unwrap_or((x, y));
        let mode = self.mode;
        let size = self.size as i32;
        line_points(lx, ly, x, y, |px, py| {
            let sq = IRect::new(px - (size - 1) / 2, py - (size - 1) / 2, size as u32, size as u32)
                .intersect(&raster.bounds());
            match mode {
                EraseMode::Clear => raster.fill_rect(sq, Px::TRANSPARENT),
                EraseMode::Replace { from, to } => {
                    for yy in sq.y..sq.bottom() {
                        for p in &mut raster.row_mut(yy as u32)[sq.x as usize..sq.right() as usize] {
                            if *p == from {
                                *p = to;
                            }
                        }
                    }
                }
            }
        });
        self.last = Some((x, y));
    }
}

/// Pixels of a bucket fill: horizontal spans `(y, x_first, x_last)` and their bounds.
#[derive(Clone, Debug, Default)]
pub struct FillRegion {
    pub spans: Vec<(i32, i32, i32)>,
    pub bounds: IRect,
}

impl FillRegion {
    pub fn paint(&self, raster: &mut Raster, color: Px) {
        for &(y, l, r) in &self.spans {
            raster.row_mut(y as u32)[l as usize..=r as usize].fill(color);
        }
    }
}

/// Finds the 4-connected region around (x, y) whose pixels differ from the seed pixel by at most
/// `tolerance` (0…100 %, max channel difference). Scanline algorithm, no recursion.
pub fn flood_region(raster: &Raster, x: i32, y: i32, tolerance: u8) -> FillRegion {
    let Some(seed) = raster.get(x, y) else { return FillRegion::default() };
    let threshold = (tolerance.min(100) as u32 * 255 + 50) / 100;
    let matches = |p: Px| {
        p.0.iter().zip(seed.0.iter()).all(|(a, b)| (*a as i32 - *b as i32).unsigned_abs() <= threshold)
    };
    let (w, h) = (raster.width() as i32, raster.height() as i32);
    let mut visited = StrokeMask::new(w as u32, h as u32);
    let mut stack = vec![(x, y)];
    let mut spans = Vec::new();
    let (mut x0, mut y0, mut x1, mut y1) = (x, y, x, y);
    while let Some((sx, sy)) = stack.pop() {
        if !visited.mark(sx, sy) {
            continue;
        }
        let row = raster.row(sy as u32);
        if !matches(row[sx as usize]) {
            continue;
        }
        // Extend the span left and right.
        let mut l = sx;
        while l > 0 && matches(row[(l - 1) as usize]) {
            l -= 1;
            visited.mark(l, sy);
        }
        let mut r = sx;
        while r + 1 < w && matches(row[(r + 1) as usize]) {
            r += 1;
            visited.mark(r, sy);
        }
        spans.push((sy, l, r));
        x0 = x0.min(l);
        x1 = x1.max(r);
        y0 = y0.min(sy);
        y1 = y1.max(sy);
        // Seed the rows above and below: one seed per run of matching, unvisited pixels.
        for ny in [sy - 1, sy + 1] {
            if ny < 0 || ny >= h {
                continue;
            }
            let nrow = raster.row(ny as u32);
            let mut in_run = false;
            for nx in l..=r {
                let ok = !visited.is_marked(nx, ny) && matches(nrow[nx as usize]);
                if ok && !in_run {
                    stack.push((nx, ny));
                }
                in_run = ok;
            }
        }
    }
    FillRegion { spans, bounds: IRect::from_corners(x0, y0, x1 + 1, y1 + 1) }
}

/// Bucket fill (see [`flood_region`]). Returns the changed bounds.
pub fn flood_fill(raster: &mut Raster, x: i32, y: i32, color: Px, tolerance: u8) -> IRect {
    if tolerance == 0 && raster.get(x, y) == Some(color) {
        return IRect::EMPTY;
    }
    let region = flood_region(raster, x, y, tolerance);
    region.paint(raster, color);
    region.bounds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba;

    fn count(r: &Raster, px: Px) -> usize {
        r.pixels().iter().filter(|p| **p == px).count()
    }

    #[test]
    fn bresenham_is_8_connected_without_extra_pixels() {
        let mut pts = Vec::new();
        line_points(0, 0, 10, 4, |x, y| pts.push((x, y)));
        assert_eq!(pts.len(), 11);
        for w in pts.windows(2) {
            assert!((w[1].0 - w[0].0).abs() <= 1 && (w[1].1 - w[0].1).abs() <= 1);
        }
        let mut diag = Vec::new();
        line_points(0, 0, 5, 5, |x, y| diag.push((x, y)));
        assert_eq!(diag, (0..=5).map(|i| (i, i)).collect::<Vec<_>>());
    }

    #[test]
    fn snap_to_45() {
        assert_eq!(snap_45(0, 0, 100, 17), (100, 0));
        assert_eq!(snap_45(0, 0, 3, 90), (0, 90));
        assert_eq!(snap_45(0, 0, 100, 90), (100, 100));
        assert_eq!(snap_45(10, 10, 0, 0), (0, 0));
    }

    #[test]
    fn pencil_1px_line_is_exact() {
        let mut r = Raster::new(20, 20);
        let black = Rgba::BLACK.premultiplied();
        let mut s = PencilStroke::new(black, 1, (20, 20));
        s.line_to(&mut r, 2, 2);
        s.line_to(&mut r, 12, 7);
        assert_eq!(count(&r, black), 11);
    }

    #[test]
    fn translucent_pencil_does_not_darken_on_overlap() {
        let mut r = Raster::new(10, 10);
        let half = Rgba::new(0, 0, 0, 128).premultiplied();
        let mut s = PencilStroke::new(half, 1, (10, 10));
        s.line_to(&mut r, 0, 5);
        s.line_to(&mut r, 9, 5);
        s.line_to(&mut r, 0, 5);
        assert!(r.row(5).iter().all(|p| *p == half));
    }

    #[test]
    fn disc_sizes() {
        assert_eq!(disc_offsets(1).len(), 1);
        assert_eq!(disc_offsets(5).len(), 21);
        let d = disc_offsets(10);
        let area = std::f32::consts::PI * 25.0;
        assert!((d.len() as f32 - area).abs() < area * 0.15, "{}", d.len());
    }

    #[test]
    fn eraser_clears_square_and_replaces_colors() {
        let black = Rgba::BLACK.premultiplied();
        let red = Rgba::rgb(255, 0, 0).premultiplied();
        let mut r = Raster::filled(10, 10, black);
        r.put(5, 5, red);
        let mut e = EraserStroke::new(3, EraseMode::Replace { from: black, to: red });
        e.line_to(&mut r, 5, 5);
        assert_eq!(count(&r, red), 9);
        let mut c = EraserStroke::new(3, EraseMode::Clear);
        c.line_to(&mut r, 0, 0);
        assert_eq!(count(&r, Px::TRANSPARENT), 4);
    }

    #[test]
    fn fill_respects_borders_and_4_connectivity() {
        let black = Rgba::BLACK.premultiplied();
        let green = Rgba::rgb(0, 200, 0).premultiplied();
        let mut r = Raster::new(10, 10);
        // A diagonal 1-px wall is passable for 8-connected fills but not for 4-connected ones.
        line_points(0, 9, 9, 0, |x, y| r.put(x, y, black));
        let dirty = flood_fill(&mut r, 0, 0, green, 0);
        assert_eq!(count(&r, green), 45);
        assert_eq!(dirty, IRect::new(0, 0, 9, 9));
        assert_eq!(r.get(9, 9), Some(Px::TRANSPARENT));
    }

    #[test]
    fn fill_tolerance() {
        let mut r = Raster::filled(4, 1, Rgba::rgb(100, 100, 100).premultiplied());
        r.put(2, 0, Rgba::rgb(110, 100, 100).premultiplied());
        let red = Rgba::rgb(255, 0, 0).premultiplied();
        let mut a = r.clone();
        flood_fill(&mut a, 0, 0, red, 0);
        assert_eq!(count(&a, red), 2);
        flood_fill(&mut r, 0, 0, red, 5);
        assert_eq!(count(&r, red), 4);
    }

    #[test]
    fn fill_large_canvas_is_fast() {
        let mut r = Raster::new(4000, 4000);
        let t = std::time::Instant::now();
        flood_fill(&mut r, 10, 10, Rgba::BLACK.premultiplied(), 0);
        let ms = t.elapsed().as_millis();
        // Generous bound for debug builds; release is far below the 200 ms target.
        assert!(ms < 3000, "{ms} ms");
    }
}
