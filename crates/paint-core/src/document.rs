//! Document model: a solid background color (which can be hidden) plus a stack of transparent
//! raster layers, exactly like the original (see docs/behavior/layers.md).

use crate::color::{Px, Rgba};
use crate::geom::IRect;
use crate::raster::Raster;

/// Stable layer identifier (survives reordering; used by history).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LayerId(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    /// 0..=255. The original has no layer opacity; PaintRS adds it (see DIFFS.md).
    pub opacity: u8,
    pub raster: Raster,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Background {
    pub color: Rgba,
    pub visible: bool,
}

impl Default for Background {
    fn default() -> Self {
        Self { color: Rgba::WHITE, visible: true }
    }
}

/// Structural state that history snapshots for whole-document operations.
#[derive(Clone, Debug, PartialEq)]
pub struct DocState {
    pub width: u32,
    pub height: u32,
    pub background: Background,
    pub layers: Vec<Layer>,
    pub active: usize,
}

#[derive(Debug)]
pub struct Document {
    width: u32,
    height: u32,
    pub background: Background,
    /// Bottom to top.
    layers: Vec<Layer>,
    active: usize,
    next_id: u64,
    dirty: IRect,
    size_generation: u64,
}

impl Document {
    /// New document: white background and one empty transparent layer.
    pub fn new(width: u32, height: u32, first_layer_name: impl Into<String>) -> Self {
        let mut doc = Self {
            width,
            height,
            background: Background::default(),
            layers: Vec::new(),
            active: 0,
            next_id: 1,
            dirty: IRect::new(0, 0, width, height),
            size_generation: 0,
        };
        let id = doc.alloc_id();
        doc.layers.push(Layer {
            id,
            name: first_layer_name.into(),
            visible: true,
            opacity: 255,
            raster: Raster::new(width, height),
        });
        doc
    }

    /// Document holding an opened image in its single layer. Opaque images get the usual white
    /// background; images with transparency get a hidden background so it survives a re-save.
    pub fn from_raster(raster: Raster, layer_name: impl Into<String>) -> Self {
        let (w, h) = (raster.width(), raster.height());
        let has_alpha = raster.pixels().iter().any(|p| p.alpha() != 255);
        let mut doc = Self::new(w, h, layer_name);
        doc.layers[0].raster = raster;
        doc.background.visible = !has_alpha;
        doc
    }

    fn alloc_id(&mut self) -> LayerId {
        let id = LayerId(self.next_id);
        self.next_id += 1;
        id
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

    /// Changes every time the canvas size changes (renderers recreate textures on change).
    pub fn size_generation(&self) -> u64 {
        self.size_generation
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active_layer(&self) -> &Layer {
        &self.layers[self.active]
    }

    pub fn active_id(&self) -> LayerId {
        self.layers[self.active].id
    }

    pub fn set_active(&mut self, index: usize) {
        if index < self.layers.len() {
            self.active = index;
        }
    }

    pub fn layer_index(&self, id: LayerId) -> Option<usize> {
        self.layers.iter().position(|l| l.id == id)
    }

    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    /// Mutable pixels of a layer. The caller must call [`Self::mark_dirty`] for what it changes.
    pub fn raster_mut(&mut self, id: LayerId) -> Option<&mut Raster> {
        self.layers.iter_mut().find(|l| l.id == id).map(|l| &mut l.raster)
    }

    pub fn mark_dirty(&mut self, rect: IRect) {
        self.dirty = self.dirty.union(&rect.intersect(&self.bounds()));
    }

    pub fn mark_all_dirty(&mut self) {
        self.dirty = self.bounds();
    }

    /// Region changed since the last call (empty if nothing changed).
    pub fn take_dirty(&mut self) -> IRect {
        std::mem::replace(&mut self.dirty, IRect::EMPTY)
    }

    pub fn set_background(&mut self, background: Background) {
        if self.background != background {
            self.background = background;
            self.mark_all_dirty();
        }
    }

    /// Snapshot of everything structural (layers included) for history.
    pub fn state(&self) -> DocState {
        DocState {
            width: self.width,
            height: self.height,
            background: self.background,
            layers: self.layers.clone(),
            active: self.active,
        }
    }

    pub fn restore(&mut self, state: DocState) {
        if (state.width, state.height) != (self.width, self.height) {
            self.size_generation += 1;
        }
        self.width = state.width;
        self.height = state.height;
        self.background = state.background;
        self.layers = state.layers;
        self.active = state.active.min(self.layers.len().saturating_sub(1));
        let max_id = self.layers.iter().map(|l| l.id.0).max().unwrap_or(0);
        self.next_id = self.next_id.max(max_id + 1);
        self.mark_all_dirty();
    }

    /// Resizes the canvas, keeping pixels at offset (`dx`, `dy`); new area is transparent.
    pub fn resize_canvas(&mut self, width: u32, height: u32, dx: i32, dy: i32) {
        for layer in &mut self.layers {
            layer.raster = layer.raster.resized_canvas(width, height, dx, dy);
        }
        self.width = width;
        self.height = height;
        self.size_generation += 1;
        self.mark_all_dirty();
    }

    /// Adds an empty layer above the active one and makes it active.
    pub fn add_layer(&mut self, name: impl Into<String>) -> LayerId {
        let id = self.alloc_id();
        let layer = Layer {
            id,
            name: name.into(),
            visible: true,
            opacity: 255,
            raster: Raster::new(self.width, self.height),
        };
        self.active += 1;
        self.layers.insert(self.active.min(self.layers.len()), layer);
        self.active = self.active.min(self.layers.len() - 1);
        id
    }

    /// Composites `rect` of the visible image into `out` (same size as the document),
    /// premultiplied. The background is included when visible.
    pub fn composite_into(&self, out: &mut Raster, rect: IRect) {
        debug_assert_eq!((out.width(), out.height()), (self.width, self.height));
        let r = rect.intersect(&self.bounds());
        if r.is_empty() {
            return;
        }
        let base =
            if self.background.visible { self.background.color.premultiplied() } else { Px::TRANSPARENT };
        out.fill_rect(r, base);
        for layer in self.layers.iter().filter(|l| l.visible && l.opacity > 0) {
            let opacity = layer.opacity;
            for y in r.y..r.bottom() {
                let src = &layer.raster.row(y as u32)[r.x as usize..r.right() as usize];
                let dst = &mut out.row_mut(y as u32)[r.x as usize..r.right() as usize];
                for (d, s) in dst.iter_mut().zip(src) {
                    if s.alpha() != 0 {
                        *d = d.over(s.scaled(opacity));
                    }
                }
            }
        }
    }

    /// The whole visible image (for saving to flat formats and "copy visible layers").
    pub fn flatten(&self) -> Raster {
        let mut out = Raster::new(self.width, self.height);
        self.composite_into(&mut out, self.bounds());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_document_is_white_background_plus_empty_layer() {
        let doc = Document::new(4, 3, "Layer 1");
        assert_eq!(doc.layers().len(), 1);
        assert!(doc.active_layer().raster.pixels().iter().all(|p| *p == Px::TRANSPARENT));
        let flat = doc.flatten();
        assert!(flat.pixels().iter().all(|p| *p == Rgba::WHITE.premultiplied()));
    }

    #[test]
    fn hidden_background_flattens_to_transparent() {
        let mut doc = Document::new(2, 2, "L");
        doc.background.visible = false;
        assert!(doc.flatten().pixels().iter().all(|p| *p == Px::TRANSPARENT));
    }

    #[test]
    fn layers_composite_bottom_to_top() {
        let mut doc = Document::new(1, 1, "L1");
        let l1 = doc.active_id();
        doc.raster_mut(l1).unwrap().put(0, 0, Rgba::rgb(255, 0, 0).premultiplied());
        let l2 = doc.add_layer("L2");
        doc.raster_mut(l2).unwrap().put(0, 0, Rgba::new(0, 0, 255, 128).premultiplied());
        let out = doc.flatten().get(0, 0).unwrap().unpremultiplied();
        assert!(out.r > 120 && out.r < 135 && out.b > 120 && out.b < 135 && out.a == 255, "{out:?}");
    }

    #[test]
    fn add_layer_goes_above_active() {
        let mut doc = Document::new(1, 1, "a");
        let b = doc.add_layer("b");
        doc.set_active(0);
        let c = doc.add_layer("c");
        let ids: Vec<_> = doc.layers().iter().map(|l| l.id).collect();
        assert_eq!(ids[1], c);
        assert_eq!(ids[2], b);
        assert_eq!(doc.active_id(), c);
    }

    #[test]
    fn opaque_image_keeps_background_visible() {
        let r = Raster::filled(2, 2, Rgba::BLACK.premultiplied());
        assert!(Document::from_raster(r, "L").background.visible);
        let mut t = Raster::filled(2, 2, Rgba::BLACK.premultiplied());
        t.put(0, 0, Px::TRANSPARENT);
        assert!(!Document::from_raster(t, "L").background.visible);
    }
}
