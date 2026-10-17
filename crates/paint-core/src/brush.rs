//! The nine brushes of the Brushes drop-down (docs/behavior/brushes.md).
//!
//! A stroke builds a coverage mask; the layer shows `original ⊕ color × mask × opacity`, where the
//! original pixels come from the stroke's [`PixelRecorder`]. Neighbouring segments of one pass are
//! combined by maximum, so a translucent stroke never darkens where its own segments meet. Brushes
//! that build up tone in the original (oil, crayon, marker, natural pencil) accumulate where the
//! stroke comes back to a place it left earlier, like the self-crossing "bow" of the reference.

use std::collections::HashMap;

use crate::color::{mul_div255, Px, Rgba};
use crate::geom::IRect;
use crate::history::{PixelRecorder, TILE};
use crate::raster::Raster;
use crate::tool::BrushKind;

const T: i32 = TILE as i32;
const TILE_PX: usize = (TILE * TILE) as usize;

/// Coverage of one 64×64 tile of the stroke.
struct MaskTile {
    /// Tone laid down by earlier passes.
    base: [u8; TILE_PX],
    /// Maximum coverage of the current pass.
    run: [u8; TILE_PX],
    /// Stroke distance at which the pixel was last written (to tell passes apart).
    when: [f32; TILE_PX],
}

impl MaskTile {
    fn new() -> Box<Self> {
        Box::new(Self { base: [0; TILE_PX], run: [0; TILE_PX], when: [0.0; TILE_PX] })
    }

    fn alpha(&self, i: usize) -> u8 {
        over(self.base[i], self.run[i])
    }
}

/// `a ⊕ b` for coverages.
fn over(a: u8, b: u8) -> u8 {
    a.saturating_add(mul_div255(b, 255 - a))
}

/// Small deterministic generator (xorshift64*), seeded per stroke.
#[derive(Clone, Debug)]
struct Rng(u64);

impl Rng {
    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    /// Uniform in [0, 1).
    fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// Hash of integer coordinates → [0, 1).
fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ seed.wrapping_mul(0xCB1A_B31F);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5BD1_E995);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Smooth value noise with cells of `cell` pixels → [0, 1).
fn value_noise(x: f32, y: f32, cell: f32, seed: u32) -> f32 {
    let (fx, fy) = (x / cell, y / cell);
    let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
    let (tx, ty) = (fx - ix as f32, fy - iy as f32);
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let a = hash2(ix, iy, seed);
    let b = hash2(ix + 1, iy, seed);
    let c = hash2(ix, iy + 1, seed);
    let d = hash2(ix + 1, iy + 1, seed);
    let top = a + (b - a) * sx;
    let bottom = c + (d - c) * sx;
    top + (bottom - top) * sy
}

/// 1-D value noise → [0, 1).
fn noise1(t: f32, seed: u32) -> f32 {
    value_noise(t, 0.5, 1.0, seed)
}

/// A brush stroke in progress.
pub struct BrushStroke {
    kind: BrushKind,
    size: u32,
    color: Px,
    opacity: u8,
    canvas: IRect,
    tiles: HashMap<(i32, i32), Box<MaskTile>>,
    rng: Rng,
    seed: u32,
    last: Option<(i32, i32)>,
    /// The point before `last` (direction of the final segment).
    prev: Option<(i32, i32)>,
    /// Brush centres of `last` and `prev` (sub-pixel for shape outlines).
    last_c: Option<(f32, f32)>,
    prev_c: Option<(f32, f32)>,
    /// Distance travelled along the stroke, in pixels.
    travelled: f32,
    /// Distance after which a revisited pixel counts as a new pass.
    window: f32,
    /// Never builds up tone on revisits (shape outlines: one even pass all around).
    single_pass: bool,
    /// Airbrush dots owed to the timer.
    spray_debt: f32,
    /// Oil/watercolor bristle intensities across the stroke.
    bristles: Vec<f32>,
}

impl BrushStroke {
    /// `color` is straight sRGB (alpha ignored); `opacity` is the slider value in percent.
    pub fn new(kind: BrushKind, size: u32, color: Rgba, opacity: u8, canvas: (u32, u32), seed: u64) -> Self {
        let size = size.max(1);
        let mut rng = Rng(seed | 1);
        for _ in 0..4 {
            rng.next_u32();
        }
        let n = (size as usize * 4 / 5).clamp(4, 200);
        let (lo, hi) = match kind {
            BrushKind::Watercolor => (0.8, 1.0),
            _ => (0.68, 1.0),
        };
        let bristles = (0..n).map(|_| lo + (hi - lo) * rng.unit()).collect();
        let seed32 = rng.next_u32();
        Self {
            kind,
            size,
            color: Rgba { a: 255, ..color }.premultiplied(),
            opacity: ((opacity.min(100) as u32 * 255 + 50) / 100) as u8,
            canvas: IRect::new(0, 0, canvas.0, canvas.1),
            tiles: HashMap::new(),
            rng,
            seed: seed32,
            last: None,
            prev: None,
            last_c: None,
            prev_c: None,
            travelled: 0.0,
            window: size as f32 + 2.0,
            single_pass: false,
            spray_debt: 0.0,
            bristles,
        }
    }

    pub fn kind(&self) -> BrushKind {
        self.kind
    }

    /// True for brushes that keep painting while the pointer stands still (the airbrush).
    pub fn needs_ticks(&self) -> bool {
        self.kind == BrushKind::Airbrush
    }

    /// Forgets everything painted (used when Shift redraws the stroke as a straight line).
    pub fn restart(&mut self) {
        self.tiles.clear();
        self.last = None;
        self.prev = None;
        self.last_c = None;
        self.prev_c = None;
        self.travelled = 0.0;
        self.spray_debt = 0.0;
    }

    /// Lifts the brush: the next point starts a new sub-path of the same stroke, which builds
    /// up tone where it crosses earlier ones (used for shape outlines and fills).
    pub fn restart_path(&mut self) {
        self.last = None;
        self.prev = None;
        self.last_c = None;
        self.prev_c = None;
        self.travelled += self.window * 2.0;
    }

    /// Paints every pixel at most once: closing a shape outline over its start leaves no darker
    /// spot, like the original.
    pub fn set_single_pass(&mut self) {
        self.single_pass = true;
    }

    fn accumulates(&self) -> bool {
        !self.single_pass
            && matches!(
                self.kind,
                BrushKind::Oil | BrushKind::Crayon | BrushKind::Marker | BrushKind::NaturalPencil
            )
    }

    /// Centre of the brush for the pointer on pixel (x, y): odd sizes are centred on the pixel,
    /// even sizes on its top-left corner (measured on the original's round brush).
    fn center(&self, x: i32, y: i32) -> (f32, f32) {
        let o = if self.size % 2 == 1 { 0.5 } else { 0.0 };
        (x as f32 + o, y as f32 + o)
    }

    fn put(&mut self, x: i32, y: i32, c: u8) {
        self.put_at(x, y, c, self.travelled);
    }

    /// Writes coverage `c` for a pixel that lies `at` px along the stroke. A pixel last written
    /// more than one brush size earlier along the stroke is a new pass and builds up tone.
    fn put_at(&mut self, x: i32, y: i32, c: u8, at: f32) {
        if c == 0 || !self.canvas.contains(x, y) {
            return;
        }
        let acc = self.accumulates();
        let (t, window) = (at, self.window);
        let tile = self.tiles.entry((x.div_euclid(T), y.div_euclid(T))).or_insert_with(MaskTile::new);
        let i = (y.rem_euclid(T) * T + x.rem_euclid(T)) as usize;
        if acc && tile.run[i] != 0 && t - tile.when[i] > window {
            // The stroke came back here: the earlier pass becomes base tone.
            tile.base[i] = over(tile.base[i], tile.run[i]);
            tile.run[i] = c;
        } else {
            tile.run[i] = tile.run[i].max(c);
        }
        tile.when[i] = tile.when[i].max(t);
    }

    /// Stroke coverage of a pixel (before opacity), for tests and previews.
    pub fn coverage(&self, x: i32, y: i32) -> u8 {
        self.tiles
            .get(&(x.div_euclid(T), y.div_euclid(T)))
            .map_or(0, |t| t.alpha((y.rem_euclid(T) * T + x.rem_euclid(T)) as usize))
    }

    /// Pointer pressed on pixel (x, y). Returns the region whose coverage changed.
    pub fn press(&mut self, x: i32, y: i32) -> IRect {
        let c = self.center(x, y);
        self.press_at(x, y, c)
    }

    fn press_at(&mut self, x: i32, y: i32, c: (f32, f32)) -> IRect {
        self.last = Some((x, y));
        self.last_c = Some(c);
        match self.kind {
            BrushKind::Brush => self.round(c, c),
            BrushKind::Airbrush => self.spray(c, 20),
            // The other brushes only paint when the pointer moves.
            _ => IRect::EMPTY,
        }
    }

    /// Pointer moved to pixel (x, y).
    pub fn move_to(&mut self, x: i32, y: i32) -> IRect {
        if self.last == Some((x, y)) {
            return IRect::EMPTY;
        }
        let c = self.center(x, y);
        self.segment_to(x, y, c)
    }

    /// Moves the brush centre to (x, y) in image coordinates without snapping to pixels (shape
    /// outlines follow their curves smoothly this way).
    pub fn move_to_f(&mut self, x: f32, y: f32) -> IRect {
        if let Some((lx, ly)) = self.last_c {
            if (x - lx).abs() < 0.01 && (y - ly).abs() < 0.01 {
                return IRect::EMPTY;
            }
        }
        let o = if self.size % 2 == 1 { 0.5 } else { 0.0 };
        self.segment_to((x - o).round() as i32, (y - o).round() as i32, (x, y))
    }

    fn segment_to(&mut self, x: i32, y: i32, b: (f32, f32)) -> IRect {
        let (Some(last), Some(a)) = (self.last, self.last_c) else { return self.press_at(x, y, b) };
        self.prev = Some(last);
        self.prev_c = Some(a);
        self.last = Some((x, y));
        self.last_c = Some(b);
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let dirty = match self.kind {
            BrushKind::Brush => self.round(a, b),
            BrushKind::CalligraphyBrush => self.nib(last, (x, y), false),
            BrushKind::CalligraphyPen => self.nib(last, (x, y), true),
            // The original sprays ~10 dots per mouse event, i.e. ~2.5 dots per pixel of travel at
            // the usual event rate; spread them along the segment so coalesced events look the same.
            BrushKind::Airbrush => self.spray_along(a, b, ((len * 2.5).round() as u32).max(10)),
            BrushKind::Marker
            | BrushKind::NaturalPencil
            | BrushKind::Crayon
            | BrushKind::Oil
            | BrushKind::Watercolor => self.band(a, b),
        };
        self.travelled += len;
        dirty
    }

    /// Time passes with the button held at the last position (airbrush only).
    pub fn tick(&mut self, dt: f32) -> IRect {
        if self.kind != BrushKind::Airbrush {
            return IRect::EMPTY;
        }
        let Some((x, y)) = self.last else { return IRect::EMPTY };
        // About 300 dots per second, as measured on the original.
        self.spray_debt += 300.0 * dt.clamp(0.0, 0.25);
        let n = self.spray_debt.floor();
        self.spray_debt -= n;
        let c = self.center(x, y);
        self.spray(c, n as u32)
    }

    // ---------------------------------------------------------------- shapes

    /// Round brush: anti-aliased disc swept along the segment.
    fn round(&mut self, a: (f32, f32), b: (f32, f32)) -> IRect {
        let s = self.size as f32;
        let (r, aa) = if self.size <= 2 {
            (s / 2.0, false)
        } else {
            ((s - 1.0) / 2.0 + 0.08 * (10.0 - s).max(0.0), true)
        };
        let reach = r + 1.5;
        let bounds = IRect::from_corners(
            (a.0.min(b.0) - reach).floor() as i32,
            (a.1.min(b.1) - reach).floor() as i32,
            (a.0.max(b.0) + reach).ceil() as i32,
            (a.1.max(b.1) + reach).ceil() as i32,
        )
        .intersect(&self.canvas);
        for y in bounds.y..bounds.bottom() {
            for x in bounds.x..bounds.right() {
                let d = dist_to_segment((x as f32 + 0.5, y as f32 + 0.5), a, b);
                let c = if aa {
                    (r - d + 0.5).clamp(0.0, 1.0)
                } else if d <= r {
                    1.0
                } else {
                    0.0
                };
                self.put(x, y, (c * 255.0).round() as u8);
            }
        }
        bounds
    }

    /// Calligraphy: a flat nib (Euclidean length = size) at 45°, stamped along a 4-connected path
    /// so that the swept band has no holes. `pen` mirrors the nib ("\" instead of "/").
    fn nib(&mut self, from: (i32, i32), to: (i32, i32), pen: bool) -> IRect {
        let m = ((self.size as f32 / std::f32::consts::SQRT_2).round() as i32).max(1);
        let lo = -(m / 2);
        let offsets: Vec<(i32, i32)> = (lo..lo + m).map(|j| if pen { (j, j) } else { (j, -j) }).collect();
        let mut dirty = IRect::EMPTY;
        let mut path = Vec::new();
        walk4(from, to, |x, y| path.push((x, y)));
        for (x, y) in path {
            for &(ox, oy) in &offsets {
                self.put(x + ox, y + oy, 255);
            }
            dirty = dirty.union(&IRect::from_corners(x + lo - 1, y + lo - 1, x - lo + 2, y - lo + 2));
        }
        dirty.intersect(&self.canvas)
    }

    /// Airbrush: `n` opaque single pixels uniformly spread over a disc of the brush size.
    fn spray(&mut self, c: (f32, f32), n: u32) -> IRect {
        self.spray_along(c, c, n)
    }

    fn spray_along(&mut self, a: (f32, f32), b: (f32, f32), n: u32) -> IRect {
        let r = self.size as f32 / 2.0;
        for _ in 0..n {
            let k = self.rng.unit();
            let c = (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
            let (dx, dy) = loop {
                let dx = (self.rng.unit() * 2.0 - 1.0) * r;
                let dy = (self.rng.unit() * 2.0 - 1.0) * r;
                if dx * dx + dy * dy <= r * r {
                    break (dx, dy);
                }
            };
            self.put((c.0 + dx).floor() as i32, (c.1 + dy).floor() as i32, 255);
        }
        let reach = r.ceil() as i32 + 1;
        IRect::from_corners(
            a.0.min(b.0).floor() as i32 - reach,
            a.1.min(b.1).floor() as i32 - reach,
            a.0.max(b.0).floor() as i32 + reach + 1,
            a.1.max(b.1).floor() as i32 + reach + 1,
        )
        .intersect(&self.canvas)
    }

    /// Flat-ended band along the segment (marker, natural pencil, crayon, oil, watercolor).
    /// Pixel centres with `0 ≤ t < len` along and `-w/2 ≤ u < w/2` across are inside, so
    /// consecutive segments of a straight stroke neither overlap nor leave gaps.
    fn band(&mut self, a: (f32, f32), b: (f32, f32)) -> IRect {
        self.band_ex(a, b, None)
    }

    /// Band pass; with `fade = Some(f)` the band runs `f` px past `b` with fading coverage
    /// instead (the soft end of oil and watercolor strokes).
    fn band_ex(&mut self, a: (f32, f32), b: (f32, f32), fade: Option<f32>) -> IRect {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt();
        if len == 0.0 {
            return IRect::EMPTY;
        }
        let (ux, uy) = (dx / len, dy / len);
        // Left-hand normal of the direction of travel.
        let (nx, ny) = (uy, -ux);
        let s = self.size as f32;
        let t0 = self.travelled;
        // Reach back over the joint with the previous segment so that turns leave no wedge
        // gaps; inside one pass coverage combines by maximum, so the overlap adds no tone.
        let (t_min, t_max) = match fade {
            Some(f) => (len, len + f),
            None if self.travelled > 0.0 => (-(s / 2.0).min(self.travelled), len),
            None => (0.0, len),
        };
        let reach = s / 2.0 + 1.0 + fade.unwrap_or(0.0);
        let bounds = IRect::from_corners(
            (a.0.min(b.0) - reach).floor() as i32,
            (a.1.min(b.1) - reach).floor() as i32,
            (a.0.max(b.0) + reach).ceil() as i32 + 1,
            (a.1.max(b.1) + reach).ceil() as i32 + 1,
        )
        .intersect(&self.canvas);
        for y in bounds.y..bounds.bottom() {
            for x in bounds.x..bounds.right() {
                let (px, py) = (x as f32 + 0.5 - a.0, y as f32 + 0.5 - a.1);
                let t = px * ux + py * uy;
                if !(t_min..t_max).contains(&t) {
                    continue;
                }
                let u = px * nx + py * ny;
                let mut c = self.band_coverage(x, y, u, t0 + t.min(len));
                if let Some(f) = fade {
                    c = (c as f32 * (1.0 - (t - len) / f)).round() as u8;
                }
                self.put_at(x, y, c, t0 + t);
            }
        }
        bounds
    }

    /// Ends the stroke: oil and watercolor lift off softly past the last point.
    pub fn finish(&mut self) -> IRect {
        if !matches!(self.kind, BrushKind::Oil | BrushKind::Watercolor) {
            return IRect::EMPTY;
        }
        let (Some(b), Some(a)) = (self.last_c, self.prev_c) else { return IRect::EMPTY };
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        self.travelled -= len;
        let r = self.band_ex(a, b, Some((self.size as f32 * 0.15).max(2.0)));
        self.travelled += len;
        r
    }

    /// Coverage of a band pixel `u` pixels across the centre line, `dist` pixels into the stroke.
    fn band_coverage(&self, x: i32, y: i32, u: f32, dist: f32) -> u8 {
        let s = self.size as f32;
        let half = s / 2.0;
        let inside = |h: f32| u >= -h && u < h;
        let c = match self.kind {
            BrushKind::Marker => {
                if !inside(half) {
                    return 0;
                }
                0.75
            }
            BrushKind::NaturalPencil => {
                if !inside(half) {
                    return 0;
                }
                // Light graphite on a paper grain fixed to the canvas; the grain gets coarser
                // and the tone a bit darker with the size (measured: 1 px ≈ 9%, 5–20 px ≈ 31%,
                // 36 px ≈ 39%).
                let base = match self.size {
                    1 => 0.1,
                    2 => 0.2,
                    _ => 0.31 + 0.08 * ((s - 20.0) / 16.0).clamp(0.0, 1.0),
                };
                let fine = hash2(x, y, self.seed) + hash2(x + 7, y + 3, self.seed) - 1.0;
                let coarse = value_noise(x as f32, y as f32, (s / 10.0).max(1.5), self.seed ^ 0x55) - 0.5;
                base + 0.3 * coarse + 0.14 * fine
            }
            BrushKind::Crayon => {
                // Waxy blotches with soft, ragged edges.
                let side = if u < 0.0 { 1.0 } else { 2.0 };
                let hw = half * (0.88 + 0.12 * noise1(dist / 3.0 + side * 17.0, self.seed)) + 0.5;
                let soft = (half * 0.45).clamp(0.6, 6.0);
                let e = ((hw - u.abs()) / soft).clamp(0.0, 1.0);
                if e <= 0.0 {
                    return 0;
                }
                // The wax texture scales with the brush: fine and even when small, blotchy when big.
                let k = ((s - 20.0) / 16.0).clamp(0.0, 1.0);
                let coarse = value_noise(x as f32, y as f32, (s / 10.0).max(1.0), self.seed ^ 0x77);
                let fine = hash2(x, y, self.seed ^ 0x99);
                let a = 0.47 + 0.3 * k + (1.1 + 0.6 * k) * (0.7 * coarse + 0.3 * fine - 0.5);
                a.clamp(0.0, 1.0) * e
            }
            // Both brushes load up: they start narrow and widen over a distance that is longer
            // for small sizes (≈ 600 / size px for oil). The right-hand edge (relative to the
            // direction of travel) stays in place and the left one grows, as in the original.
            BrushKind::Oil => {
                let w = s * (1.0 - 0.8 * (-dist / (600.0 / s).min(60.0)).exp());
                self.bristle(u + 0.5 * (half - w / 2.0), w, dist, 0.92, (w * 0.15).clamp(0.5, 1.6))
            }
            BrushKind::Watercolor => {
                let w = 0.75 * s * (1.0 - 0.85 * (-dist / (1800.0 / s).min(60.0)).exp());
                self.bristle(u + 0.8 * (half - w / 2.0), w, dist, 0.8, (w * 0.2).clamp(0.5, 2.5))
            }
            _ => 0.0,
        };
        (c.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    /// Streaky bristle profile of width `w` with soft edges `soft` px wide.
    fn bristle(&self, u: f32, w: f32, dist: f32, strength: f32, soft: f32) -> f32 {
        let half = w / 2.0;
        let edge = ((half - u.abs()) / soft + 0.5).clamp(0.0, 1.0);
        if edge <= 0.0 {
            return 0.0;
        }
        // Fine bristles across the stroke, each with its own tone that drifts along the stroke.
        let n = self.bristles.len();
        let pos = ((u / w.max(1.0) + 0.5) * (n - 1) as f32).clamp(0.0, (n - 1) as f32);
        let i = pos.floor() as usize;
        let f = pos - i as f32;
        let j = (i + 1).min(n - 1);
        let tone =
            |k: usize| self.bristles[k] * (0.82 + 0.18 * noise1(dist / 24.0 + k as f32 * 7.31, self.seed));
        let b = tone(i) * (1.0 - f) + tone(j) * f;
        strength * b * edge
    }

    // ---------------------------------------------------------------- output

    /// Writes `rect` of the stroke into `raster`: original pixels from `rec` with the brush color
    /// on top. Every tile of `rect` must have been recorded with [`PixelRecorder::touch`].
    pub fn composite(&self, raster: &mut Raster, rec: &PixelRecorder, rect: IRect) {
        let r = rect.intersect(&self.canvas);
        if r.is_empty() {
            return;
        }
        for ty in r.y.div_euclid(T)..=(r.bottom() - 1).div_euclid(T) {
            for tx in r.x.div_euclid(T)..=(r.right() - 1).div_euclid(T) {
                let Some(orig) = rec.original_tile(tx as u32, ty as u32) else { continue };
                let mask = self.tiles.get(&(tx, ty));
                let tr = IRect::new(tx * T, ty * T, TILE, TILE).intersect(&r);
                for y in tr.y..tr.bottom() {
                    for x in tr.x..tr.right() {
                        let Some(base) = orig.get(x - tx * T, y - ty * T) else { continue };
                        let i = ((y - ty * T) * T + (x - tx * T)) as usize;
                        let a = mask.map_or(0, |m| mul_div255(m.alpha(i), self.opacity));
                        let px = if a == 0 { base } else { base.over(self.color.scaled(a)) };
                        raster.put(x, y, px);
                    }
                }
            }
        }
    }
}

fn dist_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let (apx, apy) = (p.0 - a.0, p.1 - a.1);
    let l2 = abx * abx + aby * aby;
    let t = if l2 == 0.0 { 0.0 } else { ((apx * abx + apy * aby) / l2).clamp(0.0, 1.0) };
    let (qx, qy) = (a.0 + abx * t - p.0, a.1 + aby * t - p.1);
    (qx * qx + qy * qy).sqrt()
}

/// 4-connected line walk (every step changes x or y, never both), both ends included.
fn walk4(from: (i32, i32), to: (i32, i32), mut f: impl FnMut(i32, i32)) {
    let mut prev: Option<(i32, i32)> = None;
    crate::draw::line_points(from.0, from.1, to.0, to.1, |x, y| {
        if let Some((px, py)) = prev {
            if px != x && py != y {
                f(x, py);
            }
        }
        f(x, y);
        prev = Some((x, y));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    fn stroke(kind: BrushKind, size: u32, pts: &[(i32, i32)]) -> BrushStroke {
        let mut b = BrushStroke::new(kind, size, Rgba::BLACK, 100, (200, 200), 7);
        b.press(pts[0].0, pts[0].1);
        for p in &pts[1..] {
            b.move_to(p.0, p.1);
        }
        b
    }

    #[test]
    fn round_brush_matches_measured_profile() {
        // Size 10 dab measured on the original: 9 px across with ≈ 50% edges.
        let b = stroke(BrushKind::Brush, 10, &[(50, 50)]);
        assert_eq!(b.coverage(50, 50), 255);
        assert_eq!(b.coverage(46, 50), 255);
        let edge = b.coverage(45, 50);
        assert!((100..160).contains(&edge), "{edge}");
        assert_eq!(b.coverage(44, 50), 0);
        // A horizontal line has 50% edge rows (measured: 127).
        let l = stroke(BrushKind::Brush, 10, &[(20, 50), (120, 50)]);
        assert!((120..135).contains(&l.coverage(70, 45)));
        assert_eq!(l.coverage(70, 46), 255);
    }

    #[test]
    fn small_round_brushes_are_solid() {
        let b = stroke(BrushKind::Brush, 2, &[(10, 10)]);
        let n = (0..20)
            .flat_map(|y| (0..20).map(move |x| (x, y)))
            .filter(|&(x, y)| b.coverage(x, y) == 255)
            .count();
        assert_eq!(n, 4);
    }

    #[test]
    fn marker_is_flat_75_percent_and_builds_up_on_crossing() {
        // Horizontal, size 20: 20 rows centred on the pointer's corner, flat ends.
        let b = stroke(BrushKind::Marker, 20, &[(60, 30), (100, 30), (140, 30)]);
        assert_eq!(b.coverage(80, 20), 191);
        assert_eq!(b.coverage(80, 39), 191);
        assert_eq!(b.coverage(80, 19), 0);
        assert_eq!(b.coverage(80, 40), 0);
        assert_eq!(b.coverage(59, 30), 0);
        assert_eq!(b.coverage(139, 30), 191);
        assert_eq!(b.coverage(140, 30), 0);
        // Out and back over the start: two layers of 75% (measured 16 on white).
        let x = stroke(BrushKind::Marker, 10, &[(20, 50), (80, 50), (80, 120), (50, 120), (50, 20)]);
        let c = x.coverage(50, 50);
        assert!(c >= 238, "{c}");
        assert_eq!(x.coverage(30, 50), 191);
    }

    #[test]
    fn calligraphy_band_height_and_thin_direction() {
        // Size 20 moving horizontally: 14 rows (20 / √2).
        let b = stroke(BrushKind::CalligraphyBrush, 20, &[(40, 100), (120, 100)]);
        let rows = (60..140).filter(|&y| b.coverage(80, y) > 0).count();
        assert_eq!(rows, 14);
        // Moving along the nib draws a thin line, a click draws nothing.
        let t = stroke(BrushKind::CalligraphyBrush, 20, &[(40, 100), (80, 60)]);
        let across = (40..130).filter(|&x| t.coverage(x, 80) > 0).count();
        assert!(across <= 2, "{across}");
        let click = stroke(BrushKind::CalligraphyBrush, 20, &[(40, 100)]);
        assert_eq!(click.coverage(40, 100), 0);
    }

    #[test]
    fn airbrush_sprays_binary_dots_inside_the_disc() {
        let mut b = stroke(BrushKind::Airbrush, 36, &[(100, 100)]);
        b.tick(2.0);
        let mut n = 0;
        for y in 60..140 {
            for x in 60..140 {
                let c = b.coverage(x, y);
                assert!(c == 0 || c == 255);
                if c == 255 {
                    n += 1;
                    let d = (((x as f32 + 0.5) - 100.0).powi(2) + ((y as f32 + 0.5) - 100.0).powi(2)).sqrt();
                    assert!(d <= 19.0);
                }
            }
        }
        // 20 at the press + 75 for 0.25 s (dt is capped per tick).
        assert!(n > 60 && n <= 95, "{n}");
    }

    #[test]
    fn oil_starts_narrow() {
        let b = stroke(BrushKind::Oil, 36, &[(10, 100), (190, 100)]);
        let width = |x: i32| (60..140).filter(|&y| b.coverage(x, y) > 20).count();
        assert!((3..15).contains(&width(12)), "{}", width(12));
        assert!(width(150) >= 32, "{}", width(150));
    }

    #[test]
    fn composite_uses_original_pixels() {
        let mut doc = Document::new(64, 64, "L");
        let id = doc.active_id();
        let red = Rgba::rgb(255, 0, 0).premultiplied();
        doc.raster_mut(id).unwrap().fill(red);
        let mut rec = PixelRecorder::new(id);
        let mut b = BrushStroke::new(BrushKind::Marker, 10, Rgba::BLACK, 100, (64, 64), 1);
        b.press(5, 30);
        let r = b.move_to(50, 30);
        rec.touch(&doc, r);
        b.composite(doc.raster_mut(id).unwrap(), &rec, r);
        // Compositing twice gives the same result (no build-up from re-rendering).
        b.composite(doc.raster_mut(id).unwrap(), &rec, r);
        let p = doc.layer(id).unwrap().raster.get(20, 30).unwrap().unpremultiplied();
        assert_eq!(p.a, 255);
        assert!((60..68).contains(&p.r) && p.g == 0, "{p:?}");
    }
}
