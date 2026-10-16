//! Colors. Tool colors are straight (non-premultiplied) sRGB; raster pixels are premultiplied.

/// Straight-alpha sRGB color, as shown to the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    /// Premultiplied pixel for this color.
    pub fn premultiplied(self) -> Px {
        Px([mul_div255(self.r, self.a), mul_div255(self.g, self.a), mul_div255(self.b, self.a), self.a])
    }

    /// `#RRGGBB` (alpha is ignored).
    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Parses `#RGB`, `#RRGGBB` or the same without `#`.
    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('#');
        let v = |i: usize, n: usize| u8::from_str_radix(s.get(i..i + n)?, 16).ok();
        match s.len() {
            3 => Some(Self::rgb(v(0, 1)? * 17, v(1, 1)? * 17, v(2, 1)? * 17)),
            6 => Some(Self::rgb(v(0, 2)?, v(2, 2)?, v(4, 2)?)),
            _ => None,
        }
    }

    /// Opaque color from hue (degrees, any range), saturation and value (0…1).
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let [r, g, b] = hsv_to_rgb(h, s, v);
        let q = |c: f32| (c * 255.0).round().clamp(0.0, 255.0) as u8;
        Self::rgb(q(r), q(g), q(b))
    }

    /// Hue (0…360), saturation and value (0…1). Gray colors report hue 0.
    pub fn to_hsv(self) -> (f32, f32, f32) {
        let (r, g, b) = (self.r as f32 / 255.0, self.g as f32 / 255.0, self.b as f32 / 255.0);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let d = max - min;
        let h = if d == 0.0 {
            0.0
        } else if max == r {
            60.0 * ((g - b) / d).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        let s = if max == 0.0 { 0.0 } else { d / max };
        (h, s, max)
    }
}

/// HSV → sRGB channels in 0…1.
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let (s, v) = (s.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [r + m, g + m, b + m]
}

/// Premultiplied RGBA8 pixel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Px(pub [u8; 4]);

impl Px {
    pub const TRANSPARENT: Self = Self([0, 0, 0, 0]);

    pub fn alpha(self) -> u8 {
        self.0[3]
    }

    /// Back to straight alpha (rounded).
    pub fn unpremultiplied(self) -> Rgba {
        let [r, g, b, a] = self.0;
        if a == 0 {
            return Rgba::TRANSPARENT;
        }
        if a == 255 {
            return Rgba::new(r, g, b, a);
        }
        let un = |c: u8| ((c as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8;
        Rgba::new(un(r), un(g), un(b), a)
    }

    /// Source-over compositing of `src` onto `self`.
    #[inline]
    pub fn over(self, src: Px) -> Px {
        let sa = src.0[3];
        if sa == 255 {
            return src;
        }
        if sa == 0 {
            return self;
        }
        let inv = 255 - sa;
        let d = self.0;
        let s = src.0;
        Px([
            s[0].saturating_add(mul_div255(d[0], inv)),
            s[1].saturating_add(mul_div255(d[1], inv)),
            s[2].saturating_add(mul_div255(d[2], inv)),
            s[3].saturating_add(mul_div255(d[3], inv)),
        ])
    }

    /// Multiplies all channels by `k / 255` (used for layer/brush opacity).
    #[inline]
    pub fn scaled(self, k: u8) -> Px {
        if k == 255 {
            return self;
        }
        let p = self.0;
        Px([mul_div255(p[0], k), mul_div255(p[1], k), mul_div255(p[2], k), mul_div255(p[3], k)])
    }
}

/// `round(a * b / 255)` for bytes, exact for all inputs.
#[inline]
pub fn mul_div255(a: u8, b: u8) -> u8 {
    let t = a as u32 * b as u32 + 128;
    ((t + (t >> 8)) >> 8) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_div255_is_exact() {
        for a in 0..=255u32 {
            for b in 0..=255u32 {
                let expected = ((a * b) as f32 / 255.0).round() as u8;
                assert_eq!(mul_div255(a as u8, b as u8), expected, "{a}*{b}");
            }
        }
    }

    #[test]
    fn premultiply_roundtrip_opaque() {
        let c = Rgba::rgb(12, 200, 99);
        assert_eq!(c.premultiplied().unpremultiplied(), c);
    }

    #[test]
    fn over_half_black_on_white() {
        let white = Rgba::WHITE.premultiplied();
        let half_black = Rgba::new(0, 0, 0, 128).premultiplied();
        let out = white.over(half_black).unpremultiplied();
        assert_eq!(out.a, 255);
        assert!((out.r as i32 - 127).abs() <= 1);
    }

    #[test]
    fn hex_parse() {
        assert_eq!(Rgba::from_hex("#3366CC"), Some(Rgba::rgb(0x33, 0x66, 0xCC)));
        assert_eq!(Rgba::from_hex("fff"), Some(Rgba::WHITE));
        assert_eq!(Rgba::from_hex("#12"), None);
        assert_eq!(Rgba::rgb(0x33, 0x66, 0xCC).to_hex(), "#3366CC");
    }

    #[test]
    fn hsv_roundtrip() {
        assert_eq!(Rgba::from_hsv(0.0, 1.0, 1.0), Rgba::rgb(255, 0, 0));
        assert_eq!(Rgba::from_hsv(120.0, 1.0, 1.0), Rgba::rgb(0, 255, 0));
        assert_eq!(Rgba::from_hsv(240.0, 0.0, 1.0), Rgba::WHITE);
        assert_eq!(Rgba::from_hsv(77.0, 0.5, 0.0), Rgba::BLACK);
        for c in [Rgba::rgb(0xEB, 0x33, 0x24), Rgba::rgb(12, 200, 99), Rgba::rgb(7, 7, 250)] {
            let (h, s, v) = c.to_hsv();
            assert_eq!(Rgba::from_hsv(h, s, v), c);
        }
    }
}
