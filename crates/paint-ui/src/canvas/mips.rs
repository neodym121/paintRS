//! CPU mip chain of the composited canvas. Level 0 is the composite itself; higher levels are
//! 2×2 box-filtered (premultiplied, so edges of transparent areas stay clean).

use paint_core::{IRect, Px, Raster};

pub struct MipChain {
    levels: Vec<Raster>,
}

pub fn level_count(w: u32, h: u32) -> u32 {
    32 - w.max(h).max(1).leading_zeros()
}

impl MipChain {
    pub fn new(w: u32, h: u32) -> Self {
        let n = level_count(w, h);
        let mut levels = Vec::with_capacity(n as usize);
        let (mut lw, mut lh) = (w.max(1), h.max(1));
        for _ in 0..n {
            levels.push(Raster::new(lw, lh));
            lw = (lw / 2).max(1);
            lh = (lh / 2).max(1);
        }
        Self { levels }
    }

    pub fn len(&self) -> usize {
        self.levels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    pub fn level(&self, i: usize) -> &Raster {
        &self.levels[i]
    }

    pub fn base_mut(&mut self) -> &mut Raster {
        &mut self.levels[0]
    }

    /// Recomputes the higher levels covering `rect` (in level-0 pixels).
    /// Returns the changed rectangle of every level, level 0 included.
    pub fn update(&mut self, rect: IRect) -> Vec<(usize, IRect)> {
        let mut out = Vec::with_capacity(self.levels.len());
        let mut r = rect.intersect(&self.levels[0].bounds());
        if r.is_empty() {
            return out;
        }
        out.push((0, r));
        for i in 1..self.levels.len() {
            let (src, dst) = {
                let (a, b) = self.levels.split_at_mut(i);
                (&a[i - 1], &mut b[0])
            };
            let x0 = r.x.div_euclid(2);
            let y0 = r.y.div_euclid(2);
            let x1 = (r.right() + 1).div_euclid(2);
            let y1 = (r.bottom() + 1).div_euclid(2);
            let nr = IRect::from_corners(x0, y0, x1, y1).intersect(&dst.bounds());
            if nr.is_empty() {
                break;
            }
            downsample(src, dst, nr);
            out.push((i, nr));
            r = nr;
        }
        out
    }
}

fn downsample(src: &Raster, dst: &mut Raster, r: IRect) {
    let (sw, sh) = (src.width() as i32, src.height() as i32);
    for y in r.y..r.bottom() {
        let sy0 = (2 * y).min(sh - 1) as u32;
        let sy1 = (2 * y + 1).min(sh - 1) as u32;
        let row0 = src.row(sy0);
        let row1 = src.row(sy1);
        let drow = dst.row_mut(y as u32);
        for x in r.x..r.right() {
            let sx0 = (2 * x).min(sw - 1) as usize;
            let sx1 = (2 * x + 1).min(sw - 1) as usize;
            let (a, b, c, d) = (row0[sx0].0, row0[sx1].0, row1[sx0].0, row1[sx1].0);
            let mut px = [0u8; 4];
            for k in 0..4 {
                px[k] = ((a[k] as u32 + b[k] as u32 + c[k] as u32 + d[k] as u32 + 2) / 4) as u8;
            }
            drow[x as usize] = Px(px);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paint_core::Rgba;

    #[test]
    fn counts() {
        assert_eq!(level_count(1, 1), 1);
        assert_eq!(level_count(400, 300), 9);
        assert_eq!(level_count(4096, 10), 13);
    }

    #[test]
    fn averages_and_propagates() {
        let mut m = MipChain::new(4, 4);
        m.base_mut().fill_rect(IRect::new(0, 0, 2, 2), Rgba::WHITE.premultiplied());
        let changed = m.update(IRect::new(0, 0, 4, 4));
        assert_eq!(changed.len(), 3);
        assert_eq!(m.level(1).get(0, 0), Some(Rgba::WHITE.premultiplied()));
        assert_eq!(m.level(1).get(1, 0), Some(Px::TRANSPARENT));
        // 1×1 level = average of one white quarter.
        assert_eq!(m.level(2).get(0, 0).unwrap().alpha(), 64);
    }
}
