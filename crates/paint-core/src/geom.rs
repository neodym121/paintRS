//! Integer geometry used for pixel regions.

/// Axis-aligned integer rectangle: `x..x + w`, `y..y + h`. Empty when `w == 0 || h == 0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct IRect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl IRect {
    pub const EMPTY: Self = Self { x: 0, y: 0, w: 0, h: 0 };

    pub const fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    /// Rectangle from two corners (inclusive-exclusive: `x0..x1`, `y0..y1`).
    pub fn from_corners(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        let (ax, bx) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        let (ay, by) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        Self::new(ax, ay, (bx - ax) as u32, (by - ay) as u32)
    }

    pub fn is_empty(&self) -> bool {
        self.w == 0 || self.h == 0
    }

    pub fn right(&self) -> i32 {
        self.x + self.w as i32
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h as i32
    }

    pub fn area(&self) -> u64 {
        self.w as u64 * self.h as u64
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = self.right().min(other.right());
        let y1 = self.bottom().min(other.bottom());
        if x1 <= x0 || y1 <= y0 {
            Self::EMPTY
        } else {
            Self::from_corners(x0, y0, x1, y1)
        }
    }

    /// Smallest rectangle containing both; an empty side is ignored.
    pub fn union(&self, other: &Self) -> Self {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        Self::from_corners(
            self.x.min(other.x),
            self.y.min(other.y),
            self.right().max(other.right()),
            self.bottom().max(other.bottom()),
        )
    }

    /// Grow by `d` pixels on every side.
    pub fn inflate(&self, d: i32) -> Self {
        if self.is_empty() {
            return *self;
        }
        Self::from_corners(self.x - d, self.y - d, self.right() + d, self.bottom() + d)
    }

    pub fn translate(&self, dx: i32, dy: i32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.w, self.h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersect_and_union() {
        let a = IRect::new(0, 0, 10, 10);
        let b = IRect::new(5, 5, 10, 10);
        assert_eq!(a.intersect(&b), IRect::new(5, 5, 5, 5));
        assert_eq!(a.union(&b), IRect::new(0, 0, 15, 15));
        assert!(a.intersect(&IRect::new(20, 20, 1, 1)).is_empty());
        assert_eq!(IRect::EMPTY.union(&b), b);
    }

    #[test]
    fn corners_are_normalized() {
        assert_eq!(IRect::from_corners(5, 7, 1, 2), IRect::new(1, 2, 4, 5));
    }
}
