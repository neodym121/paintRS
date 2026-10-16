//! The 23 shapes of the gallery (order as in the original) and their outlines in a unit box
//! (`0..1` on both axes, y pointing down). Curves are flattened to polylines.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

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

pub type Contour = Vec<[f32; 2]>;

/// Outline contours of `kind` in the unit box. Closed shapes return closed contours
/// (the last point is not repeated). Line/curve/polygon return a representative icon path.
pub fn unit_outline(kind: ShapeKind) -> Vec<Contour> {
    use ShapeKind::*;
    match kind {
        Line => vec![vec![[0.0, 1.0], [1.0, 0.0]]],
        Curve => vec![bezier([0.0, 1.0], [0.1, 0.0], [0.6, 1.2], [1.0, 0.0], 24)],
        Oval => vec![ellipse([0.5, 0.5], [0.5, 0.5], 64)],
        Rectangle => vec![rect(0.0, 0.0, 1.0, 1.0)],
        RoundedRectangle => vec![rounded_rect(0.0, 0.0, 1.0, 1.0, 0.2)],
        Polygon => vec![vec![[0.3, 0.0], [1.0, 0.0], [0.75, 1.0], [0.0, 1.0], [0.4, 0.45]]],
        Triangle => vec![vec![[0.5, 0.0], [1.0, 1.0], [0.0, 1.0]]],
        RightTriangle => vec![vec![[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]]],
        Diamond => vec![vec![[0.5, 0.0], [1.0, 0.5], [0.5, 1.0], [0.0, 0.5]]],
        Pentagon => vec![regular(5, -FRAC_PI_2)],
        Hexagon => vec![vec![[0.25, 0.0], [0.75, 0.0], [1.0, 0.5], [0.75, 1.0], [0.25, 1.0], [0.0, 0.5]]],
        ArrowRight => vec![arrow_right()],
        ArrowLeft => vec![map(arrow_right(), |[x, y]| [1.0 - x, y])],
        ArrowUp => vec![map(arrow_right(), |[x, y]| [y, 1.0 - x])],
        ArrowDown => vec![map(arrow_right(), |[x, y]| [y, x])],
        Star4 => vec![star(4, 0.38, -FRAC_PI_2)],
        Star5 => vec![star(5, 0.382, -FRAC_PI_2)],
        Star6 => vec![star(6, 0.577, -FRAC_PI_2)],
        CalloutRect => {
            let mut c = rounded_rect(0.0, 0.0, 1.0, 0.72, 0.12);
            insert_tail(&mut c, [0.18, 0.72], [0.12, 1.0], [0.34, 0.72]);
            vec![c]
        }
        CalloutOval => {
            let body = ellipse([0.5, 0.38], [0.5, 0.38], 64);
            vec![with_tail_on_ellipse(body, [0.12, 1.0])]
        }
        CalloutCloud => cloud(),
        Heart => vec![heart()],
        Lightning => vec![vec![
            [0.38, 0.0],
            [0.62, 0.30],
            [0.53, 0.35],
            [0.80, 0.62],
            [0.70, 0.67],
            [1.0, 1.0],
            [0.45, 0.74],
            [0.56, 0.69],
            [0.18, 0.47],
            [0.30, 0.41],
            [0.0, 0.17],
        ]],
    }
}

fn map(c: Contour, f: impl Fn([f32; 2]) -> [f32; 2]) -> Contour {
    c.into_iter().map(f).collect()
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Contour {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

fn ellipse(c: [f32; 2], r: [f32; 2], n: usize) -> Contour {
    (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * TAU;
            [c[0] + r[0] * a.cos(), c[1] + r[1] * a.sin()]
        })
        .collect()
}

fn rounded_rect(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Contour {
    let mut out = Vec::new();
    let corners = [
        ([x1 - r, y0 + r], -FRAC_PI_2),
        ([x1 - r, y1 - r], 0.0),
        ([x0 + r, y1 - r], FRAC_PI_2),
        ([x0 + r, y0 + r], PI),
    ];
    for (c, start) in corners {
        for i in 0..=8 {
            let a = start + i as f32 / 8.0 * FRAC_PI_2;
            out.push([c[0] + r * a.cos(), c[1] + r * a.sin()]);
        }
    }
    out
}

/// Regular polygon inscribed in the unit box (stretched to fill it).
fn regular(n: usize, start: f32) -> Contour {
    let pts: Vec<[f32; 2]> = (0..n)
        .map(|i| {
            let a = start + i as f32 / n as f32 * TAU;
            [a.cos(), a.sin()]
        })
        .collect();
    fit_unit(pts)
}

fn star(points: usize, inner: f32, start: f32) -> Contour {
    let pts: Vec<[f32; 2]> = (0..points * 2)
        .map(|i| {
            let a = start + i as f32 / (points * 2) as f32 * TAU;
            let r = if i % 2 == 0 { 1.0 } else { inner };
            [r * a.cos(), r * a.sin()]
        })
        .collect();
    fit_unit(pts)
}

/// Scales/translates points so their bounding box becomes the unit box.
fn fit_unit(pts: Vec<[f32; 2]>) -> Contour {
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

fn arrow_right() -> Contour {
    vec![[0.0, 0.25], [0.5, 0.25], [0.5, 0.0], [1.0, 0.5], [0.5, 1.0], [0.5, 0.75], [0.0, 0.75]]
}

fn bezier(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], n: usize) -> Contour {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            [
                w[0] * p0[0] + w[1] * p1[0] + w[2] * p2[0] + w[3] * p3[0],
                w[0] * p0[1] + w[1] * p1[1] + w[2] * p2[1] + w[3] * p3[1],
            ]
        })
        .collect()
}

/// Inserts a triangular tail into a contour at the bottom edge, between `a` and `b`.
fn insert_tail(c: &mut Contour, a: [f32; 2], tip: [f32; 2], b: [f32; 2]) {
    // Find the bottom-edge point closest to `b` going clockwise; the rounded rect walks
    // top-right → bottom-right → bottom-left, so insert after the last point with x > b.x on y1.
    let idx = c
        .iter()
        .enumerate()
        .rev()
        .find(|(_, p)| (p[1] - a[1]).abs() < 1e-3 && p[0] > b[0])
        .map_or(c.len() / 2, |(i, _)| i);
    c.splice(idx + 1..idx + 1, [b, tip, a]);
}

fn with_tail_on_ellipse(body: Contour, tip: [f32; 2]) -> Contour {
    // Replace the arc between ~100° and ~130° (bottom-left) with a tail to `tip`.
    let n = body.len();
    let (i0, i1) = (n * 100 / 360, n * 125 / 360);
    let mut out = Vec::with_capacity(n + 1);
    out.extend_from_slice(&body[..=i0]);
    out.push(tip);
    out.extend_from_slice(&body[i1..]);
    out
}

fn cloud() -> Vec<Contour> {
    // Outer bumpy outline: a scalloped ellipse.
    let bumps = 9;
    let n = 180;
    let body: Contour = (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * TAU;
            let k = (a * bumps as f32).sin().abs();
            let r = 0.86 + 0.14 * k;
            [0.5 + 0.5 * r * a.cos(), 0.36 + 0.36 * r * a.sin()]
        })
        .collect();
    vec![body, ellipse([0.2, 0.83], [0.06, 0.05], 24), ellipse([0.1, 0.95], [0.035, 0.03], 16)]
}

fn heart() -> Contour {
    let n = 96;
    let pts: Vec<[f32; 2]> = (0..n)
        .map(|i| {
            let t = i as f32 / n as f32 * TAU;
            let x = 16.0 * t.sin().powi(3);
            let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            [x, -y]
        })
        .collect();
    fit_unit(pts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_outlines_fit_the_unit_box() {
        for kind in ShapeKind::ALL {
            for c in unit_outline(kind) {
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
}
