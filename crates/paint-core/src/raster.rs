//! Premultiplied RGBA8 pixel buffer.

use crate::color::Px;
use crate::geom::IRect;

/// A `width × height` image of premultiplied RGBA8 pixels, row-major, no padding.
#[derive(Clone, PartialEq, Eq)]
pub struct Raster {
    width: u32,
    height: u32,
    data: Vec<Px>,
}

impl std::fmt::Debug for Raster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Raster({}×{})", self.width, self.height)
    }
}

impl Raster {
    /// Fully transparent raster.
    pub fn new(width: u32, height: u32) -> Self {
        Self::filled(width, height, Px::TRANSPARENT)
    }

    pub fn filled(width: u32, height: u32, px: Px) -> Self {
        Self { width, height, data: vec![px; width as usize * height as usize] }
    }

    /// Wraps premultiplied pixels. Panics if the length does not match.
    pub fn from_pixels(width: u32, height: u32, data: Vec<Px>) -> Self {
        assert_eq!(data.len(), width as usize * height as usize, "raster size mismatch");
        Self { width, height, data }
    }

    /// From straight-alpha RGBA8 bytes (e.g. a decoded image).
    pub fn from_straight_rgba8(width: u32, height: u32, bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), width as usize * height as usize * 4, "raster size mismatch");
        let data = bytes
            .chunks_exact(4)
            .map(|c| crate::color::Rgba::new(c[0], c[1], c[2], c[3]).premultiplied())
            .collect();
        Self { width, height, data }
    }

    /// Straight-alpha RGBA8 bytes (for encoders).
    pub fn to_straight_rgba8(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.data.len() * 4);
        for px in &self.data {
            let c = px.unpremultiplied();
            out.extend_from_slice(&[c.r, c.g, c.b, c.a]);
        }
        out
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn bounds(&self) -> IRect {
        IRect::new(0, 0, self.width, self.height)
    }

    pub fn pixels(&self) -> &[Px] {
        &self.data
    }

    pub fn pixels_mut(&mut self) -> &mut [Px] {
        &mut self.data
    }

    /// Raw bytes, 4 per pixel.
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: `Px` is `#[repr(transparent)]` over `[u8; 4]`, so the buffer is plain bytes.
        let len = self.data.len() * 4;
        unsafe { std::slice::from_raw_parts(self.data.as_ptr().cast::<u8>(), len) }
    }

    pub fn row(&self, y: u32) -> &[Px] {
        let w = self.width as usize;
        &self.data[y as usize * w..(y as usize + 1) * w]
    }

    pub fn row_mut(&mut self, y: u32) -> &mut [Px] {
        let w = self.width as usize;
        &mut self.data[y as usize * w..(y as usize + 1) * w]
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Option<Px> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some(self.data[y as usize * self.width as usize + x as usize])
    }

    #[inline]
    pub fn put(&mut self, x: i32, y: i32, px: Px) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        self.data[y as usize * self.width as usize + x as usize] = px;
    }

    pub fn fill(&mut self, px: Px) {
        self.data.fill(px);
    }

    /// Fills `rect` (clipped) with `px`.
    pub fn fill_rect(&mut self, rect: IRect, px: Px) {
        let r = rect.intersect(&self.bounds());
        for y in r.y..r.bottom() {
            self.row_mut(y as u32)[r.x as usize..r.right() as usize].fill(px);
        }
    }

    /// Copy of `rect` (clipped to bounds; the result has the clipped size).
    pub fn crop(&self, rect: IRect) -> Raster {
        let r = rect.intersect(&self.bounds());
        let mut out = Raster::new(r.w, r.h);
        for row in 0..r.h {
            let src = &self.row((r.y as u32) + row)[r.x as usize..r.right() as usize];
            out.row_mut(row).copy_from_slice(src);
        }
        out
    }

    /// Writes `src` with its top-left corner at (`x`, `y`), replacing pixels (clipped).
    pub fn blit(&mut self, src: &Raster, x: i32, y: i32) {
        let dst = IRect::new(x, y, src.width, src.height).intersect(&self.bounds());
        for yy in dst.y..dst.bottom() {
            let sy = (yy - y) as u32;
            let sx0 = (dst.x - x) as usize;
            let line = &src.row(sy)[sx0..sx0 + dst.w as usize];
            self.row_mut(yy as u32)[dst.x as usize..dst.right() as usize].copy_from_slice(line);
        }
    }

    /// Source-over composites `src` (scaled by `opacity`) at (`x`, `y`), clipped to `clip`.
    pub fn draw_over(&mut self, src: &Raster, x: i32, y: i32, opacity: u8, clip: IRect) {
        if opacity == 0 {
            return;
        }
        let dst = IRect::new(x, y, src.width, src.height).intersect(&self.bounds()).intersect(&clip);
        for yy in dst.y..dst.bottom() {
            let sy = (yy - y) as u32;
            let sx0 = (dst.x - x) as usize;
            let sline = &src.row(sy)[sx0..sx0 + dst.w as usize];
            let dline = &mut self.row_mut(yy as u32)[dst.x as usize..dst.right() as usize];
            for (d, s) in dline.iter_mut().zip(sline) {
                *d = d.over(s.scaled(opacity));
            }
        }
    }

    /// New raster of a different size; existing pixels are kept at offset (`dx`, `dy`),
    /// the rest is transparent.
    pub fn resized_canvas(&self, width: u32, height: u32, dx: i32, dy: i32) -> Raster {
        let mut out = Raster::new(width, height);
        out.blit(self, dx, dy);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba;

    #[test]
    fn crop_and_blit_roundtrip() {
        let mut r = Raster::new(8, 8);
        r.fill_rect(IRect::new(2, 2, 3, 3), Rgba::BLACK.premultiplied());
        let part = r.crop(IRect::new(1, 1, 5, 5));
        let mut r2 = Raster::new(8, 8);
        r2.blit(&part, 1, 1);
        assert_eq!(r, r2);
    }

    #[test]
    fn crop_is_clipped() {
        let r = Raster::new(4, 4);
        let c = r.crop(IRect::new(2, 2, 10, 10));
        assert_eq!((c.width(), c.height()), (2, 2));
    }

    #[test]
    fn resized_canvas_keeps_pixels() {
        let mut r = Raster::new(2, 2);
        r.put(0, 0, Rgba::BLACK.premultiplied());
        let big = r.resized_canvas(4, 4, 1, 1);
        assert_eq!(big.get(1, 1), Some(Rgba::BLACK.premultiplied()));
        assert_eq!(big.get(0, 0), Some(Px::TRANSPARENT));
    }

    #[test]
    fn straight_bytes_roundtrip_opaque() {
        let bytes = vec![10, 20, 30, 255, 200, 100, 0, 255];
        let r = Raster::from_straight_rgba8(2, 1, &bytes);
        assert_eq!(r.to_straight_rgba8(), bytes);
    }
}
