//! The 23 shapes of the gallery (order as in the original): geometry in a unit box (measured on
//! the original, docs/behavior/shapes.md), drawing with tiny-skia — aliased like the original or
//! anti-aliased on request — and the brush-like outline and fill styles.

use std::f32::consts::{FRAC_PI_2, TAU};

use tiny_skia::{FillRule, LineCap, LineJoin, Mask, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::brush::BrushStroke;
use crate::color::{mul_div255, Px, Rgba};
use crate::geom::IRect;
use crate::raster::Raster;
use crate::tool::BrushKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShapeKind {
    Line,
    Curve,
    Oval,
    Rectangle,
    RoundedRectangle,
    Polygon,
    Triangle,
    RightTriangle,
    Diamond,
    Pentagon,
    Hexagon,
    ArrowRight,
    ArrowLeft,
    ArrowUp,
    ArrowDown,
    Star4,
    Star5,
    Star6,
    CalloutRect,
    CalloutOval,
    CalloutCloud,
    Heart,
    Lightning,
}

impl ShapeKind {
    pub const ALL: [ShapeKind; 23] = [
        ShapeKind::Line,
        ShapeKind::Curve,
        ShapeKind::Oval,
        ShapeKind::Rectangle,
        ShapeKind::RoundedRectangle,
        ShapeKind::Polygon,
        ShapeKind::Triangle,
        ShapeKind::RightTriangle,
        ShapeKind::Diamond,
        ShapeKind::Pentagon,
        ShapeKind::Hexagon,
        ShapeKind::ArrowRight,
        ShapeKind::ArrowLeft,
        ShapeKind::ArrowUp,
        ShapeKind::ArrowDown,
        ShapeKind::Star4,
        ShapeKind::Star5,
        ShapeKind::Star6,
        ShapeKind::CalloutRect,
        ShapeKind::CalloutOval,
        ShapeKind::CalloutCloud,
        ShapeKind::Heart,
        ShapeKind::Lightning,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&k| k == self).unwrap_or(0)
    }

    /// Line and curve are open paths defined by points, not by a box.
    pub fn is_open(self) -> bool {
        matches!(self, ShapeKind::Line | ShapeKind::Curve)
    }
}

// ---------------------------------------------------------------- unit geometry

/// Path segment in unit-box or image coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Move([f32; 2]),
    Line([f32; 2]),
    Cubic([f32; 2], [f32; 2], [f32; 2]),
    Close,
}

/// Bézier constant for quarter circles.
const K: f32 = 0.552_284_8;

fn poly(points: &[[f32; 2]]) -> Vec<Seg> {
    let mut out = vec![Seg::Move(points[0])];
    out.extend(points[1..].iter().map(|&p| Seg::Line(p)));
    out.push(Seg::Close);
    out
}

fn ellipse_segs(c: [f32; 2], r: [f32; 2]) -> Vec<Seg> {
    let [cx, cy] = c;
    let [rx, ry] = r;
    vec![
        Seg::Move([cx + rx, cy]),
        Seg::Cubic([cx + rx, cy + ry * K], [cx + rx * K, cy + ry], [cx, cy + ry]),
        Seg::Cubic([cx - rx * K, cy + ry], [cx - rx, cy + ry * K], [cx - rx, cy]),
        Seg::Cubic([cx - rx, cy - ry * K], [cx - rx * K, cy - ry], [cx, cy - ry]),
        Seg::Cubic([cx + rx * K, cy - ry], [cx + rx, cy - ry * K], [cx + rx, cy]),
        Seg::Close,
    ]
}

/// Rounded rectangle from (0, 0) to (`x1`, `y1`), clockwise from the top edge; `tail` (left
/// base x, tip, right base x) is inserted into the bottom edge for the rectangular callout.
fn rounded_rect_segs(x1: f32, y1: f32, rx: f32, ry: f32, tail: Option<(f32, [f32; 2], f32)>) -> Vec<Seg> {
    let (x0, y0) = (0.0, 0.0);
    let mut s = vec![
        Seg::Move([x0 + rx, y0]),
        Seg::Line([x1 - rx, y0]),
        Seg::Cubic([x1 - rx + rx * K, y0], [x1, y0 + ry - ry * K], [x1, y0 + ry]),
        Seg::Line([x1, y1 - ry]),
        Seg::Cubic([x1, y1 - ry + ry * K], [x1 - rx + rx * K, y1], [x1 - rx, y1]),
    ];
    if let Some((left, tip, right)) = tail {
        s.extend([Seg::Line([right, y1]), Seg::Line(tip), Seg::Line([left, y1])]);
    }
    s.extend([
        Seg::Line([x0 + rx, y1]),
        Seg::Cubic([x0 + rx - rx * K, y1], [x0, y1 - ry + ry * K], [x0, y1 - ry]),
        Seg::Line([x0, y0 + ry]),
        Seg::Cubic([x0, y0 + ry - ry * K], [x0 + rx - rx * K, y0], [x0 + rx, y0]),
        Seg::Close,
    ]);
    s
}

/// Regular polygon or star (`inner` < 1) fitted to the unit box.
fn star_points(points: usize, inner: f32, start: f32) -> Vec<[f32; 2]> {
    let n = if inner >= 1.0 { points } else { points * 2 };
    let pts = (0..n)
        .map(|i| {
            let a = start + i as f32 / n as f32 * TAU;
            let r = if inner >= 1.0 || i % 2 == 0 { 1.0 } else { inner };
            [r * a.cos(), r * a.sin()]
        })
        .collect();
    fit_unit(pts)
}

/// Scales/translates points so their bounding box becomes the unit box.
fn fit_unit(pts: Vec<[f32; 2]>) -> Vec<[f32; 2]> {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in &pts {
        x0 = x0.min(p[0]);
        y0 = y0.min(p[1]);
        x1 = x1.max(p[0]);
        y1 = y1.max(p[1]);
    }
    let (w, h) = ((x1 - x0).max(1e-6), (y1 - y0).max(1e-6));
    pts.into_iter().map(|[x, y]| [(x - x0) / w, (y - y0) / h]).collect()
}

fn arrow_right() -> Vec<[f32; 2]> {
    vec![[0.0, 0.25], [0.5, 0.25], [0.5, 0.0], [1.0, 0.5], [0.5, 1.0], [0.5, 0.75], [0.0, 0.75]]
}

/// Ellipse with a wedge to `tip`; the wedge leaves the ellipse at `a0` and returns at `a1`
/// (angles in radians, y down, a0 > a1 on the lower half).
fn oval_callout(c: [f32; 2], r: [f32; 2], a0: f32, a1: f32, tip: [f32; 2]) -> Vec<Seg> {
    let n = 72;
    let span = a1 + TAU - a0;
    let at = |a: f32| [c[0] + r[0] * a.cos(), c[1] + r[1] * a.sin()];
    let mut s = vec![Seg::Move(at(a0))];
    for i in 1..=n {
        s.push(Seg::Line(at(a0 + span * i as f32 / n as f32)));
    }
    s.push(Seg::Line(tip));
    s.push(Seg::Close);
    s
}

/// Thought cloud body: elliptical arcs in a 43200-unit box, each `(start, radii, start angle,
/// sweep)` with angles in 1/60000 degree measured as seen on the ellipse (the cloud-callout
/// construction of ECMA-376, its numbers refitted to the original at 500×300 px).
const CLOUD_ARCS: [([f64; 2], [f64; 2], f64, f64); 11] = [
    ([4237.0, 14768.0], [6799.0, 10590.0], -10907808.0, 7238766.0),
    ([14005.0, 5202.0], [5613.0, 7227.0], -8646143.0, 4916714.0),
    ([22423.0, 3453.0], [4455.0, 6195.0], -8748475.0, 5803381.0),
    ([29870.0, 2474.0], [4857.0, 6815.0], -7859164.0, 7034504.0),
    ([38410.0, 5626.0], [5333.0, 7273.0], -4722533.0, 6661615.0),
    ([41911.0, 15510.0], [6665.0, 9140.0], -2776035.0, 7741140.0),
    ([37577.0, 30164.0], [5695.0, 7837.0], 37501.0, 6902000.0),
    ([28841.0, 36728.0], [6752.0, 9485.0], 1347096.0, 6910353.0),
    ([16700.0, 39217.0], [8210.0, 10613.0], 3734558.0, 4302661.0),
    ([5897.0, 35463.0], [4040.0, 5638.0], -16496525.0, 9179134.0),
    ([2336.0, 25644.0], [4445.0, 5895.0], -14809710.0, 9361131.0),
];

/// The short arcs the original draws inside the cloud where its bumps meet.
const CLOUD_SPURS: [([f64; 2], [f64; 2], f64, f64); 11] = [
    ([4693.0, 26177.0], [4345.0, 5945.0], 5204520.0, 1585770.0),
    ([6928.0, 34899.0], [4360.0, 5918.0], 4416628.0, 686848.0),
    ([16478.0, 39090.0], [6752.0, 9215.0], 8257449.0, 844866.0),
    ([28827.0, 34751.0], [6752.0, 9215.0], 387196.0, 959901.0),
    ([34129.0, 22954.0], [5785.0, 7867.0], -4217541.0, 4255042.0),
    ([41798.0, 15354.0], [5333.0, 7273.0], 1819082.0, 1665090.0),
    ([38324.0, 5426.0], [4857.0, 6595.0], -824660.0, 891534.0),
    ([29078.0, 3952.0], [4857.0, 6595.0], -8950887.0, 1091722.0),
    ([22141.0, 4720.0], [4365.0, 5945.0], -9809656.0, 1061181.0),
    ([14000.0, 5192.0], [6753.0, 9190.0], -4002417.0, 739161.0),
    ([4127.0, 15789.0], [6753.0, 9190.0], 9459261.0, 711490.0),
];

/// Points of an elliptical arc starting at `start` (its first point excluded).
fn arc_points(start: [f64; 2], r: [f64; 2], st: f64, sw: f64) -> Vec<[f64; 2]> {
    use std::f64::consts::TAU as TAU64;
    // A visual angle on the ellipse → its parametric angle.
    let param = |deg: f64| {
        let a = deg.to_radians();
        (a.sin() / r[1]).atan2(a.cos() / r[0])
    };
    let t0 = param(st / 60000.0);
    let mut d = param((st + sw) / 60000.0) - t0;
    if sw > 0.0 {
        while d <= 0.0 {
            d += TAU64;
        }
    } else {
        while d >= 0.0 {
            d -= TAU64;
        }
    }
    let c = [start[0] - r[0] * t0.cos(), start[1] - r[1] * t0.sin()];
    let n = ((d.abs() * 40.0) as usize).max(8);
    (1..=n)
        .map(|i| {
            let t = t0 + d * i as f64 / n as f64;
            [c[0] + r[0] * t.cos(), c[1] + r[1] * t.sin()]
        })
        .collect()
}

/// Thought cloud: the bumpy body (99% × 85% of the box) with its inner arcs, and three bubbles
/// towards the bottom left (measured).
fn cloud() -> Vec<Seg> {
    let f = |p: [f64; 2]| [(p[0] / 43200.0 * 0.991) as f32, (p[1] / 43200.0 * 0.853) as f32];
    // Each bump runs from where it meets the previous one to where it meets the next one.
    let mut arcs: Vec<Vec<[f64; 2]>> = CLOUD_ARCS
        .iter()
        .map(|&(start, r, st, sw)| {
            let mut pts = vec![start];
            pts.extend(arc_points(start, r, st, sw));
            pts
        })
        .collect();
    let n = arcs.len();
    let mut cuts = vec![(0usize, usize::MAX); n];
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, b) = (&arcs[i], &arcs[j]);
        let mut best = (f64::MAX, a.len() - 1, 0);
        for (ia, p) in a.iter().enumerate().skip(a.len() / 2) {
            for (ib, q) in b.iter().enumerate().take(b.len() / 2) {
                let d = (p[0] - q[0]).hypot(p[1] - q[1]);
                if d < best.0 {
                    best = (d, ia, ib);
                }
            }
        }
        cuts[i].1 = best.1;
        cuts[j].0 = best.2;
    }
    for (pts, &(from, to)) in arcs.iter_mut().zip(&cuts) {
        *pts = pts[from..=to.min(pts.len() - 1)].to_vec();
    }
    let mut s = Vec::new();
    for (i, pts) in arcs.iter().enumerate() {
        for (k, &p) in pts.iter().enumerate() {
            s.push(if i == 0 && k == 0 { Seg::Move(f(p)) } else { Seg::Line(f(p)) });
        }
    }
    s.push(Seg::Close);
    for &(start, r, st, sw) in &CLOUD_SPURS {
        s.push(Seg::Move(f(start)));
        s.extend(arc_points(start, r, st, sw).into_iter().map(|p| Seg::Line(f(p))));
    }
    s.extend(ellipse_segs([0.2053, 0.8596], [0.065, 0.0606]));
    s.extend(ellipse_segs([0.1453, 0.9348], [0.0354, 0.0359]));
    s.extend(ellipse_segs([0.1005, 0.9748], [0.0305, 0.0251]));
    s
}

/// Two cubics per side from the dip at the top to the bottom tip (fitted to the original at
/// 500×300 and checked at 200×130).
fn heart() -> Vec<Seg> {
    let m = |p: [f32; 2]| [1.0 - p[0], p[1]];
    let (dip, c1, c2, side, c3, c4, tip) = (
        [0.5, 0.256],
        [0.659, -0.151],
        [1.012, -0.0145],
        [0.997, 0.305],
        [0.995, 0.505],
        [0.849, 0.816],
        [0.5, 1.0],
    );
    vec![
        Seg::Move(dip),
        Seg::Cubic(c1, c2, side),
        Seg::Cubic(c3, c4, tip),
        Seg::Cubic(m(c4), m(c3), m(side)),
        Seg::Cubic(m(c2), m(c1), dip),
        Seg::Close,
    ]
}

/// Path of `kind` in the unit box (y down). Line, curve and polygon return an icon path; when
/// drawn they use [`ShapeGeom::Line`] etc.
pub fn unit_path(kind: ShapeKind) -> Vec<Seg> {
    use ShapeKind::*;
    match kind {
        Line => vec![Seg::Move([0.0, 1.0]), Seg::Line([1.0, 0.0])],
        Curve => vec![Seg::Move([0.0, 1.0]), Seg::Cubic([0.1, 0.0], [0.6, 1.2], [1.0, 0.0])],
        Oval => ellipse_segs([0.5, 0.5], [0.5, 0.5]),
        Rectangle => poly(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        // The original draws it over 99% of the box, corner radii 16% of the width and of the
        // height (measured at 20…575 px).
        RoundedRectangle => rounded_rect_segs(0.99, 0.99, 0.16, 0.16, None),
        Polygon => poly(&[[0.3, 0.0], [1.0, 0.0], [0.75, 1.0], [0.0, 1.0], [0.4, 0.45]]),
        Triangle => poly(&[[0.5, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        RightTriangle => poly(&[[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        Diamond => poly(&[[0.5, 0.0], [1.0, 0.5], [0.5, 1.0], [0.0, 0.5]]),
        Pentagon => poly(&star_points(5, 1.0, -FRAC_PI_2)),
        // Pointy top and bottom (measured).
        Hexagon => poly(&star_points(6, 1.0, -FRAC_PI_2)),
        ArrowRight => poly(&arrow_right()),
        ArrowLeft => poly(&arrow_right().into_iter().map(|[x, y]| [1.0 - x, y]).collect::<Vec<_>>()),
        ArrowUp => poly(&arrow_right().into_iter().map(|[x, y]| [y, 1.0 - x]).collect::<Vec<_>>()),
        ArrowDown => poly(&arrow_right().into_iter().map(|[x, y]| [y, x]).collect::<Vec<_>>()),
        Star4 => poly(&star_points(4, 0.34, -FRAC_PI_2)),
        // The original's stars are not regular (measured vertices).
        Star5 => poly(&[
            [0.5, 0.0],
            [0.62, 0.38],
            [1.0, 0.3904],
            [0.6995, 0.62],
            [0.8098, 1.0],
            [0.5, 0.7797],
            [0.1902, 1.0],
            [0.3005, 0.62],
            [0.0, 0.3904],
            [0.38, 0.38],
        ]),
        Star6 => poly(&[
            [0.5, 0.0],
            [0.68, 0.25],
            [1.0, 0.25],
            [0.84, 0.5],
            [1.0, 0.75],
            [0.68, 0.75],
            [0.5, 1.0],
            [0.32, 0.75],
            [0.0, 0.75],
            [0.16, 0.5],
            [0.0, 0.25],
            [0.32, 0.25],
        ]),
        // Callout bodies take 88% of the height, the tail reaches the bottom (measured).
        CalloutRect => rounded_rect_segs(1.0, 0.88, 0.16, 0.16, Some((0.158, [0.28, 1.0], 0.401))),
        CalloutOval => oval_callout([0.5, 0.44], [0.5, 0.44], 2.214, 1.7747, [0.22, 1.0]),
        CalloutCloud => cloud(),
        Heart => heart(),
        // Measured vertices.
        Lightning => poly(&[
            [0.39, 0.0],
            [0.589, 0.28],
            [0.521, 0.321],
            [0.758, 0.549],
            [0.679, 0.59],
            [1.0, 1.0],
            [0.4726, 0.6913],
            [0.56, 0.64],
            [0.2386, 0.4493],
            [0.351, 0.3912],
            [0.0, 0.19],
        ]),
    }
}

pub type Contour = Vec<[f32; 2]>;

/// Flattens segments into contours (with a "closed" flag); cubics use `steps` lines.
pub fn flatten(segs: &[Seg], steps: usize) -> Vec<(Contour, bool)> {
    flatten_with(segs, |_, _, _, _| steps)
}

/// Flattens segments so no chord strays more than `tol` px from its cubic.
pub fn flatten_tol(segs: &[Seg], tol: f32) -> Vec<(Contour, bool)> {
    flatten_with(segs, |p0, c1, c2, p| {
        // Wang's formula: n = sqrt(3/4 · max |second difference| / tol).
        let d = |a: [f32; 2], b: [f32; 2], c: [f32; 2]| {
            (a[0] - 2.0 * b[0] + c[0]).hypot(a[1] - 2.0 * b[1] + c[1])
        };
        let m = d(p0, c1, c2).max(d(c1, c2, p));
        ((0.75 * m / tol).sqrt().ceil() as usize).clamp(1, 1024)
    })
}

fn flatten_with(
    segs: &[Seg],
    steps: impl Fn([f32; 2], [f32; 2], [f32; 2], [f32; 2]) -> usize,
) -> Vec<(Contour, bool)> {
    let mut out: Vec<(Contour, bool)> = Vec::new();
    let mut cur: Contour = Vec::new();
    for s in segs {
        match *s {
            Seg::Move(p) => {
                if cur.len() > 1 {
                    out.push((std::mem::take(&mut cur), false));
                }
                cur = vec![p];
            }
            Seg::Line(p) => cur.push(p),
            Seg::Cubic(c1, c2, p) => {
                let p0 = *cur.last().unwrap_or(&c1);
                let steps = steps(p0, c1, c2, p);
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    let u = 1.0 - t;
                    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                    cur.push([
                        w[0] * p0[0] + w[1] * c1[0] + w[2] * c2[0] + w[3] * p[0],
                        w[0] * p0[1] + w[1] * c1[1] + w[2] * c2[1] + w[3] * p[1],
                    ]);
                }
            }
            Seg::Close => {
                if cur.len() > 1 {
                    out.push((std::mem::take(&mut cur), true));
                }
            }
        }
    }
    if cur.len() > 1 {
        out.push((cur, false));
    }
    out
}

/// Outline contours of `kind` in the unit box (for gallery icons).
/// Outline contours of `kind` in the unit box (for gallery icons), with a "closed" flag and
/// points at least 1/100 apart.
pub fn unit_outline(kind: ShapeKind) -> Vec<(Contour, bool)> {
    flatten(&unit_path(kind), 12)
        .into_iter()
        .map(|(c, closed)| {
            let mut out: Contour = Vec::with_capacity(c.len());
            for p in c {
                if out.last().is_none_or(|q: &[f32; 2]| (p[0] - q[0]).hypot(p[1] - q[1]) >= 0.01) {
                    out.push(p);
                }
            }
            (out, closed)
        })
        .collect()
}

// ---------------------------------------------------------------- shapes on the canvas

/// How an outline or a fill is painted (the Outline ▾ / Fill ▾ menus).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PaintStyle {
    None,
    #[default]
    Solid,
    Crayon,
    Marker,
    Oil,
    NaturalPencil,
    Watercolor,
}

impl PaintStyle {
    pub const ALL: [PaintStyle; 7] = [
        PaintStyle::None,
        PaintStyle::Solid,
        PaintStyle::Crayon,
        PaintStyle::Marker,
        PaintStyle::Oil,
        PaintStyle::NaturalPencil,
        PaintStyle::Watercolor,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }

    /// The brush whose texture this style uses.
    pub fn brush(self) -> Option<BrushKind> {
        match self {
            PaintStyle::Crayon => Some(BrushKind::Crayon),
            PaintStyle::Marker => Some(BrushKind::Marker),
            PaintStyle::Oil => Some(BrushKind::Oil),
            PaintStyle::NaturalPencil => Some(BrushKind::NaturalPencil),
            PaintStyle::Watercolor => Some(BrushKind::Watercolor),
            _ => None,
        }
    }
}

/// Outline widths of the Size ▾ menu (5 px is the default).
pub const WIDTHS: [u32; 4] = [1, 3, 5, 8];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeStyle {
    pub outline: PaintStyle,
    pub fill: PaintStyle,
    pub width: u32,
    pub outline_color: Rgba,
    pub fill_color: Rgba,
    pub antialias: bool,
}

/// A shape placed on the canvas. Coordinates are image coordinates where pixel (x, y) has its
/// centre at (x + 0.5, y + 0.5): a 1 px outline of a box dragged from pixel A to pixel B
/// covers both A and B, like the original.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeGeom {
    /// A gallery shape stretched over the box between corners `a` and `b` (any order; shapes
    /// are never mirrored) and rotated by `angle` radians around the box centre.
    Boxed {
        kind: ShapeKind,
        a: [f32; 2],
        b: [f32; 2],
        angle: f32,
    },
    Line {
        a: [f32; 2],
        b: [f32; 2],
    },
    /// Cubic curve from `p[0]` to `p[3]` with control points `p[1]`, `p[2]`.
    Curve {
        p: [[f32; 2]; 4],
    },
    Polygon {
        pts: Vec<[f32; 2]>,
        closed: bool,
    },
}

/// Pixel `(x, y)` → its centre in shape coordinates.
pub fn px_center(x: i32, y: i32) -> [f32; 2] {
    [x as f32 + 0.5, y as f32 + 0.5]
}

fn rotate(p: [f32; 2], c: [f32; 2], angle: f32) -> [f32; 2] {
    let (s, co) = angle.sin_cos();
    let (dx, dy) = (p[0] - c[0], p[1] - c[1]);
    [c[0] + dx * co - dy * s, c[1] + dx * s + dy * co]
}

impl ShapeGeom {
    pub fn kind(&self) -> ShapeKind {
        match self {
            ShapeGeom::Boxed { kind, .. } => *kind,
            ShapeGeom::Line { .. } => ShapeKind::Line,
            ShapeGeom::Curve { .. } => ShapeKind::Curve,
            ShapeGeom::Polygon { .. } => ShapeKind::Polygon,
        }
    }

    /// True if the shape has no area to draw (a click without a drag).
    pub fn is_degenerate(&self) -> bool {
        match self {
            ShapeGeom::Boxed { a, b, .. } => (a[0] - b[0]).abs() < 0.5 || (a[1] - b[1]).abs() < 0.5,
            ShapeGeom::Line { a, b } => a == b,
            ShapeGeom::Curve { p } => p.iter().all(|q| *q == p[0]),
            ShapeGeom::Polygon { pts, .. } => pts.len() < 2 || pts.iter().all(|q| *q == pts[0]),
        }
    }

    /// Whether the outline is an open path (lines, curves, unfinished polygons).
    fn is_open(&self) -> bool {
        match self {
            ShapeGeom::Line { .. } | ShapeGeom::Curve { .. } => true,
            ShapeGeom::Polygon { closed, .. } => !closed,
            ShapeGeom::Boxed { .. } => false,
        }
    }

    /// Path in image coordinates.
    pub fn segs(&self) -> Vec<Seg> {
        match self {
            ShapeGeom::Boxed { kind, a, b, angle } => {
                let (x0, y0) = (a[0].min(b[0]), a[1].min(b[1]));
                let (w, h) = ((a[0] - b[0]).abs(), (a[1] - b[1]).abs());
                let c = [x0 + w / 2.0, y0 + h / 2.0];
                let f = |p: [f32; 2]| rotate([x0 + p[0] * w, y0 + p[1] * h], c, *angle);
                unit_path(*kind)
                    .into_iter()
                    .map(|s| match s {
                        Seg::Move(p) => Seg::Move(f(p)),
                        Seg::Line(p) => Seg::Line(f(p)),
                        Seg::Cubic(p1, p2, p3) => Seg::Cubic(f(p1), f(p2), f(p3)),
                        Seg::Close => Seg::Close,
                    })
                    .collect()
            }
            ShapeGeom::Line { a, b } => vec![Seg::Move(*a), Seg::Line(*b)],
            ShapeGeom::Curve { p } => vec![Seg::Move(p[0]), Seg::Cubic(p[1], p[2], p[3])],
            ShapeGeom::Polygon { pts, closed } => {
                let mut s = vec![Seg::Move(pts[0])];
                s.extend(pts[1..].iter().map(|&p| Seg::Line(p)));
                if *closed {
                    s.push(Seg::Close);
                }
                s
            }
        }
    }

    /// Bounding box of the path's points (control points included).
    pub fn point_bounds(&self) -> [f32; 4] {
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for s in self.segs() {
            let pts: Vec<[f32; 2]> = match s {
                Seg::Move(p) | Seg::Line(p) => vec![p],
                Seg::Cubic(a, b, c) => vec![a, b, c],
                Seg::Close => Vec::new(),
            };
            for p in pts {
                x0 = x0.min(p[0]);
                y0 = y0.min(p[1]);
                x1 = x1.max(p[0]);
                y1 = y1.max(p[1]);
            }
        }
        [x0, y0, x1, y1]
    }

    /// Pixels the shape can touch with an outline of `width`.
    pub fn bounds(&self, width: u32) -> IRect {
        let [x0, y0, x1, y1] = self.point_bounds();
        // Miter joins reach up to 4 widths out; brush outlines add their own fringe.
        let m = width as f32 * 4.0 + 4.0;
        IRect::from_corners(
            (x0 - m).floor() as i32,
            (y0 - m).floor() as i32,
            (x1 + m).ceil() as i32 + 1,
            (y1 + m).ceil() as i32 + 1,
        )
    }

    /// Moves the shape by (dx, dy).
    pub fn translate(&mut self, dx: f32, dy: f32) {
        let t = |p: &mut [f32; 2]| {
            p[0] += dx;
            p[1] += dy;
        };
        match self {
            ShapeGeom::Boxed { a, b, .. } | ShapeGeom::Line { a, b } => {
                t(a);
                t(b);
            }
            ShapeGeom::Curve { p } => p.iter_mut().for_each(t),
            ShapeGeom::Polygon { pts, .. } => pts.iter_mut().for_each(t),
        }
    }
}

fn to_skia_path(segs: &[Seg], dx: f32, dy: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    for s in segs {
        match *s {
            Seg::Move(p) => pb.move_to(p[0] + dx, p[1] + dy),
            Seg::Line(p) => pb.line_to(p[0] + dx, p[1] + dy),
            Seg::Cubic(a, b, c) => {
                pb.cubic_to(a[0] + dx, a[1] + dy, b[0] + dx, b[1] + dy, c[0] + dx, c[1] + dy)
            }
            Seg::Close => pb.close(),
        }
    }
    pb.finish()
}

/// Curves of 1 px outlines are drawn as chords at most this far from the curve: the original's
/// ellipses match pixel for pixel with it.
const HAIR_TOLERANCE: f32 = 0.16;

fn skia_color(c: Rgba) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.r, c.g, c.b, 255)
}

/// Draws the shape. Returns its pixels (premultiplied, transparent around the shape) and the
/// canvas rectangle they belong to, or `None` when nothing would be drawn.
pub fn render(
    geom: &ShapeGeom,
    style: &ShapeStyle,
    canvas: (u32, u32),
    seed: u64,
) -> Option<(Raster, IRect)> {
    let canvas_rect = IRect::new(0, 0, canvas.0, canvas.1);
    let r = geom.bounds(style.width).intersect(&canvas_rect);
    if r.is_empty() || geom.is_degenerate() {
        return None;
    }
    let mut pm = Pixmap::new(r.w, r.h)?;
    // Even outline widths are centred on the pixel corner, like the original (8 px reaches 4 px
    // out on the top/left side and 3 px on the bottom/right one).
    let parity = if style.outline != PaintStyle::None && style.width % 2 == 0 { 0.5 } else { 0.0 };
    let (dx, dy) = (-r.x as f32 - parity, -r.y as f32 - parity);
    let segs = geom.segs();
    let path = to_skia_path(&segs, dx, dy)?;

    let open = geom.is_open();

    // Fill first (color 2), outline on top (color 1).
    if !open && style.fill != PaintStyle::None {
        if style.fill == PaintStyle::Solid {
            let mut paint = Paint::default();
            paint.set_color(skia_color(style.fill_color));
            paint.anti_alias = style.antialias;
            pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        } else if let Some(mut mask) = Mask::new(r.w, r.h) {
            mask.fill_path(&path, FillRule::Winding, style.antialias, Transform::identity());
            let cov = brush_fill(style.fill, r.w, r.h, seed);
            paint_coverage(&mut pm, style.fill_color, |i| mul_div255(mask.data()[i], cov[i]));
        }
    }
    if style.outline != PaintStyle::None {
        let w = style.width.max(1) as f32;
        if style.outline == PaintStyle::Solid && w <= 1.0 && !style.antialias {
            // A 1 px aliased outline: thin connected lines through pixel centres, both ends
            // included, like the original.
            let px = Rgba { a: 255, ..style.outline_color }.premultiplied().0;
            let (pw, ph) = (r.w as i32, r.h as i32);
            let data = pm.data_mut();
            for (contour, closed) in flatten_tol(&segs, HAIR_TOLERANCE) {
                let mut pts: Vec<[f32; 2]> = contour.iter().map(|p| [p[0] + dx, p[1] + dy]).collect();
                if closed {
                    pts.push(pts[0]);
                }
                for seg in pts.windows(2) {
                    hair_segment(seg[0], seg[1], |x, y| {
                        if x >= 0 && y >= 0 && x < pw && y < ph {
                            let i = (y * pw + x) as usize * 4;
                            data[i..i + 4].copy_from_slice(&px);
                        }
                    });
                }
            }
        } else if style.outline == PaintStyle::Solid {
            let mut paint = Paint::default();
            paint.set_color(skia_color(style.outline_color));
            paint.anti_alias = style.antialias;
            let stroke = Stroke {
                width: w,
                miter_limit: 4.0,
                line_cap: if open { LineCap::Round } else { LineCap::Butt },
                line_join: if matches!(geom, ShapeGeom::Curve { .. } | ShapeGeom::Line { .. }) {
                    LineJoin::Round
                } else {
                    LineJoin::Miter
                },
                dash: None,
            };
            pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        } else if let Some(kind) = style.outline.brush() {
            let size = style.width.max(kind.min_size());
            let mut b = BrushStroke::new(kind, size, style.outline_color, 100, (r.w, r.h), seed ^ 0xA5A5);
            b.set_single_pass();
            for (contour, closed) in flatten_tol(&segs, 0.25) {
                let mut pts: Vec<[f32; 2]> = contour.iter().map(|p| [p[0] + dx, p[1] + dy]).collect();
                if closed {
                    pts.push(pts[0]);
                }
                b.restart_path();
                walk_polyline(&pts, OUTLINE_STEP, |x, y| {
                    b.move_to_f(x, y);
                });
            }
            b.finish();
            let w = r.w as i32;
            paint_coverage(&mut pm, style.outline_color, |i| b.coverage(i as i32 % w, i as i32 / w));
        }
    }

    let data: Vec<Px> = pm.data().chunks_exact(4).map(|c| Px([c[0], c[1], c[2], c[3]])).collect();
    Some((Raster::from_pixels(r.w, r.h, data), r))
}

/// Visits the pixels of an aliased 1 px line from `a` to `b` (pixel-centre coordinates): one
/// pixel per column (or row, for steep lines) whose centre lies within the segment, ends
/// included. A centre exactly between two pixels goes to the one nearer the left (top) end,
/// as in the original.
fn hair_segment(a: [f32; 2], b: [f32; 2], mut f: impl FnMut(i32, i32)) {
    let (a, b) = ([a[0] as f64, a[1] as f64], [b[0] as f64, b[1] as f64]);
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    // Swaps axes for steep lines so the walk is always along the major axis.
    let steep = dy.abs() > dx.abs();
    let (mut p, mut q) = if steep { ([a[1], a[0]], [b[1], b[0]]) } else { (a, b) };
    if p[0] > q[0] {
        std::mem::swap(&mut p, &mut q);
    }
    let len = q[0] - p[0];
    let slope = if len > 0.0 { (q[1] - p[1]) / len } else { 0.0 };
    let first = (p[0] - 0.5 - 1e-6).ceil() as i64;
    let last = (q[0] - 0.5 + 1e-6).floor() as i64;
    for i in first..=last {
        let c = i as f64 + 0.5;
        let v = p[1] + slope * (c - p[0]) - slope.signum() * 1e-5;
        let j = v.floor() as i64;
        if steep {
            f(j as i32, i as i32);
        } else {
            f(i as i32, j as i32);
        }
    }
    if first > last {
        // Shorter than a pixel: still marks the pixel it lies in.
        let m = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
        f(m[0].floor() as i32, m[1].floor() as i32);
    }
}

/// Brush outlines follow the path in steps of this many pixels: pixel-by-pixel steps would
/// turn curves into stairs that the brush footprint shows as bumps.
const OUTLINE_STEP: f32 = 3.0;

/// Visits points along a polyline (pixel-centre coordinates) about `step` px apart, always
/// including its vertices.
fn walk_polyline(pts: &[[f32; 2]], step: f32, mut f: impl FnMut(f32, f32)) {
    let mut visit = |p: [f32; 2]| f(p[0], p[1]);
    if let Some(&p) = pts.first() {
        visit(p);
    }
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = (b[0] - a[0]).hypot(b[1] - a[1]);
        let mut t = step;
        while t < len - step * 0.5 {
            visit([a[0] + (b[0] - a[0]) * t / len, a[1] + (b[1] - a[1]) * t / len]);
            t += step;
        }
        visit(b);
    }
}

/// Paints `color` with per-pixel coverage under what is already in the pixmap... over it.
fn paint_coverage(pm: &mut Pixmap, color: Rgba, cov: impl Fn(usize) -> u8) {
    let src = Rgba { a: 255, ..color }.premultiplied();
    let data = pm.data_mut();
    for (i, px) in data.chunks_exact_mut(4).enumerate() {
        let a = cov(i);
        if a == 0 {
            continue;
        }
        let under = Px([px[0], px[1], px[2], px[3]]);
        let out = under.over(src.scaled(a));
        px.copy_from_slice(&out.0);
    }
}

/// Texture of a brush-style fill: horizontal strokes of the brush across the box, like the
/// original's fills (marker is flat).
fn brush_fill(style: PaintStyle, w: u32, h: u32, seed: u64) -> Vec<u8> {
    let n = (w * h) as usize;
    let Some(kind) = style.brush() else { return vec![255; n] };
    if kind == BrushKind::Marker {
        return vec![191; n];
    }
    // Brush size and row pitch of the horizontal strokes (looked up on the original's fills).
    let (size, pitch) = match kind {
        BrushKind::Crayon => (10, 7),
        BrushKind::Oil => (16, 8),
        BrushKind::NaturalPencil => (6, 4),
        _ => (16, 10),
    };
    let mut b = BrushStroke::new(kind, size, Rgba::BLACK, 100, (w, h), seed);
    let mut rng = seed as u32 | 1;
    let mut rand = move |m: u32| {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        rng % m.max(1)
    };
    let mut y = pitch / 2;
    while y < h as i32 + pitch {
        // Each row is two strokes: one from a random place in the left part to the right
        // edge (its loading start shows, as on the original) and one covering the rest.
        let lead = -(size as i32 * 2);
        let split = lead + rand(w / 3 + size * 2) as i32;
        for (from, to) in [(split, w as i32 + size as i32), (lead, split + size as i32)] {
            b.restart_path();
            let mut x = from;
            while x <= to {
                b.move_to(x, y);
                x += 3;
            }
        }
        y += pitch;
    }
    b.finish();
    (0..n).map(|i| b.coverage(i as i32 % w as i32, i as i32 / w as i32)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(width: u32) -> ShapeStyle {
        ShapeStyle {
            outline: PaintStyle::Solid,
            fill: PaintStyle::None,
            width,
            outline_color: Rgba::BLACK,
            fill_color: Rgba::rgb(255, 0, 0),
            antialias: false,
        }
    }

    fn boxed(kind: ShapeKind, x0: i32, y0: i32, x1: i32, y1: i32) -> ShapeGeom {
        ShapeGeom::Boxed { kind, a: px_center(x0, y0), b: px_center(x1, y1), angle: 0.0 }
    }

    /// Ink pixels of a rendered shape on a canvas of the given size.
    fn ink(geom: &ShapeGeom, st: &ShapeStyle) -> Raster {
        let mut canvas = Raster::new(400, 300);
        if let Some((src, r)) = render(geom, st, (400, 300), 1) {
            canvas.draw_over(&src, r.x, r.y, 255, canvas.bounds());
        }
        canvas
    }

    fn extent(r: &Raster) -> (i32, i32, i32, i32) {
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for y in 0..r.height() as i32 {
            for x in 0..r.width() as i32 {
                if r.get(x, y).is_some_and(|p| p.alpha() > 0) {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        (x0, y0, x1, y1)
    }

    #[test]
    fn all_outlines_fit_the_unit_box() {
        for kind in ShapeKind::ALL {
            for (c, _) in unit_outline(kind) {
                assert!(c.len() >= 2, "{kind:?}");
                for p in c {
                    assert!(
                        (-0.01..=1.01).contains(&p[0]) && (-0.01..=1.01).contains(&p[1]),
                        "{kind:?} {p:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn order_matches_gallery() {
        assert_eq!(ShapeKind::ALL[0], ShapeKind::Line);
        assert_eq!(ShapeKind::ALL[7], ShapeKind::RightTriangle);
        assert_eq!(ShapeKind::ALL[22], ShapeKind::Lightning);
        assert_eq!(ShapeKind::Star5.index(), 16);
    }

    #[test]
    fn rectangle_1px_covers_both_drag_points() {
        // Measured: dragging from (50, 40) to (250, 170) outlines pixels 50..=250 × 40..=170.
        let r = ink(&boxed(ShapeKind::Rectangle, 50, 40, 250, 170), &style(1));
        assert_eq!(extent(&r), (50, 40, 250, 170));
        assert_eq!(r.get(150, 40).map(|p| p.alpha()), Some(255));
        assert_eq!(r.get(150, 41).map(|p| p.alpha()), Some(0));
        let pixels: Vec<u8> = r.pixels().iter().map(|p| p.alpha()).collect();
        assert!(pixels.iter().all(|a| *a == 0 || *a == 255), "aliased");
    }

    #[test]
    fn hairlines_pick_pixels_like_the_original() {
        // Measured: a 1 px line from (0, 0) to (200, 130) has column 10 (exactly between rows 6
        // and 7) in row 6, and both ends.
        let line = ShapeGeom::Line { a: px_center(0, 0), b: px_center(200, 130) };
        let r = ink(&line, &style(1));
        let on = |x, y| r.get(x, y).is_some_and(|p| p.alpha() == 255);
        assert!(on(0, 0) && on(200, 130));
        assert!(on(10, 6) && !on(10, 7));
        assert!(on(1, 1) && on(2, 1) && on(3, 2));
        // One pixel per column: no doubled pixels.
        for x in 0..=200 {
            assert_eq!((0..300).filter(|&y| on(x, y)).count(), 1, "column {x}");
        }
        // Sharp vertices are drawn too.
        let t = ink(&boxed(ShapeKind::Triangle, 0, 0, 200, 130), &style(1));
        assert_eq!(extent(&t), (0, 0, 200, 130));
        assert!(t.get(100, 0).is_some_and(|p| p.alpha() == 255));
    }

    #[test]
    fn rounded_rectangle_uses_99_percent_of_the_box() {
        // Measured: a 200×130 drag gives a rounded rectangle over 0..=198 × 0..=129.
        let r = ink(&boxed(ShapeKind::RoundedRectangle, 0, 0, 200, 130), &style(1));
        assert_eq!(extent(&r), (0, 0, 198, 129));
    }

    #[test]
    fn thick_outlines_are_centred_like_the_original() {
        // Measured extents for a box from (0,0) to (80,60): 3 px → -1..81, 5 px → -2..82,
        // 8 px → -4..83.
        for (w, lo, hi) in [(3, -1, 81), (5, -2, 82), (8, -4, 83)] {
            let r = ink(&boxed(ShapeKind::Rectangle, 100, 100, 180, 160), &style(w));
            let (x0, y0, x1, y1) = extent(&r);
            assert_eq!((x0 - 100, x1 - 100), (lo, hi), "width {w}");
            assert_eq!((y0 - 100, y1 - 160), (lo, hi - 80), "width {w}");
        }
    }

    #[test]
    fn fill_uses_fill_color_inside() {
        let mut st = style(1);
        st.fill = PaintStyle::Solid;
        let r = ink(&boxed(ShapeKind::Oval, 10, 10, 110, 90), &st);
        assert_eq!(r.get(60, 50), Some(Rgba::rgb(255, 0, 0).premultiplied()));
        assert_eq!(r.get(60, 10), Some(Rgba::BLACK.premultiplied()));
    }

    #[test]
    fn lines_have_round_caps() {
        let r = ink(&ShapeGeom::Line { a: px_center(100, 100), b: px_center(200, 100) }, &style(8));
        let (x0, _, x1, _) = extent(&r);
        assert!(x0 <= 97 && x1 >= 203, "{x0} {x1}");
    }

    #[test]
    fn shapes_are_not_mirrored_by_the_drag_direction() {
        let a = ink(&boxed(ShapeKind::ArrowRight, 20, 20, 120, 80), &style(1));
        let b = ink(&boxed(ShapeKind::ArrowRight, 120, 80, 20, 20), &style(1));
        assert!(a == b);
    }

    #[test]
    fn brush_styles_render_something() {
        for s in [
            PaintStyle::Crayon,
            PaintStyle::Marker,
            PaintStyle::Oil,
            PaintStyle::NaturalPencil,
            PaintStyle::Watercolor,
        ] {
            let mut st = style(8);
            st.outline = s;
            st.fill = s;
            let r = ink(&boxed(ShapeKind::Rectangle, 50, 50, 200, 150), &st);
            let inked = r.pixels().iter().filter(|p| p.alpha() > 0).count();
            assert!(inked > 3000, "{s:?}: {inked}");
        }
    }
}
