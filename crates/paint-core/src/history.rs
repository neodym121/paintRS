//! Undo/redo. Pixel edits are stored as deltas: "before/after" copies of only the touched
//! 64×64 tiles. Structural operations (resize, layer changes) store document snapshots.

use std::collections::{BTreeMap, VecDeque};

use crate::document::{DocState, Document, LayerId};
use crate::geom::IRect;
use crate::raster::Raster;

/// Tile edge used to track touched regions of a stroke.
pub const TILE: u32 = 64;

/// Depth observed in the original (≈100 steps), see docs/PARITY.md H2.
pub const DEFAULT_LIMIT: usize = 100;

/// Memory cap for the whole history (bytes of stored pixels).
pub const DEFAULT_MEMORY_BUDGET: usize = 1024 * 1024 * 1024;

#[derive(Clone, Debug)]
struct TilePatch {
    rect: IRect,
    before: Raster,
    after: Raster,
}

#[derive(Clone, Debug)]
enum Change {
    Pixels { layer: LayerId, tiles: Vec<TilePatch> },
    State { before: Box<DocState>, after: Box<DocState> },
}

impl Change {
    fn bytes(&self) -> usize {
        match self {
            Change::Pixels { tiles, .. } => {
                tiles.iter().map(|t| (t.before.pixels().len() + t.after.pixels().len()) * 4).sum()
            }
            Change::State { before, after } => state_bytes(before) + state_bytes(after),
        }
    }

    fn apply(&self, doc: &mut Document, undo: bool) -> IRect {
        match self {
            Change::Pixels { layer, tiles } => {
                let mut dirty = IRect::EMPTY;
                if let Some(raster) = doc.raster_mut(*layer) {
                    for t in tiles {
                        let src = if undo { &t.before } else { &t.after };
                        raster.blit(src, t.rect.x, t.rect.y);
                        dirty = dirty.union(&t.rect);
                    }
                }
                doc.mark_dirty(dirty);
                dirty
            }
            Change::State { before, after } => {
                doc.restore(if undo { (**before).clone() } else { (**after).clone() });
                doc.bounds()
            }
        }
    }
}

fn state_bytes(s: &DocState) -> usize {
    s.layers.iter().map(|l| l.raster.pixels().len() * 4).sum()
}

#[derive(Clone, Debug)]
struct Entry {
    id: u64,
    label: &'static str,
    changes: Vec<Change>,
    bytes: usize,
}

/// Records the original pixels of every tile a tool touches, then turns them into a delta.
#[derive(Debug)]
pub struct PixelRecorder {
    layer: LayerId,
    tiles: BTreeMap<(u32, u32), Raster>,
}

impl PixelRecorder {
    pub fn new(layer: LayerId) -> Self {
        Self { layer, tiles: BTreeMap::new() }
    }

    pub fn layer(&self) -> LayerId {
        self.layer
    }

    /// Call *before* modifying `rect` of the layer.
    pub fn touch(&mut self, doc: &Document, rect: IRect) {
        let r = rect.intersect(&doc.bounds());
        if r.is_empty() {
            return;
        }
        let Some(layer) = doc.layer(self.layer) else { return };
        let (tx0, ty0) = (r.x as u32 / TILE, r.y as u32 / TILE);
        let (tx1, ty1) = ((r.right() as u32 - 1) / TILE, (r.bottom() as u32 - 1) / TILE);
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                self.tiles.entry((tx, ty)).or_insert_with(|| layer.raster.crop(tile_rect(tx, ty)));
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    /// Restores the recorded original pixels (cancelling an edit in progress).
    pub fn revert(&self, doc: &mut Document) {
        let mut dirty = IRect::EMPTY;
        if let Some(raster) = doc.raster_mut(self.layer) {
            for (&(tx, ty), before) in &self.tiles {
                let r = tile_rect(tx, ty);
                raster.blit(before, r.x, r.y);
                dirty = dirty.union(&r);
            }
        }
        doc.mark_dirty(dirty);
    }

    fn into_change(self, doc: &Document) -> Option<Change> {
        let layer = doc.layer(self.layer)?;
        let tiles: Vec<TilePatch> = self
            .tiles
            .into_iter()
            .filter_map(|((tx, ty), before)| {
                let rect = tile_rect(tx, ty).intersect(&doc.bounds());
                let after = layer.raster.crop(rect);
                (after != before).then_some(TilePatch { rect, before, after })
            })
            .collect();
        (!tiles.is_empty()).then_some(Change::Pixels { layer: self.layer, tiles })
    }
}

fn tile_rect(tx: u32, ty: u32) -> IRect {
    IRect::new((tx * TILE) as i32, (ty * TILE) as i32, TILE, TILE)
}

#[derive(Debug)]
pub struct History {
    undo: VecDeque<Entry>,
    redo: Vec<Entry>,
    limit: usize,
    budget: usize,
    next_id: u64,
}

impl Default for History {
    fn default() -> Self {
        Self::new(DEFAULT_LIMIT, DEFAULT_MEMORY_BUDGET)
    }
}

impl History {
    pub fn new(limit: usize, budget: usize) -> Self {
        Self { undo: VecDeque::new(), redo: Vec::new(), limit: limit.max(1), budget, next_id: 1 }
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_label(&self) -> Option<&'static str> {
        self.undo.back().map(|e| e.label)
    }

    pub fn redo_label(&self) -> Option<&'static str> {
        self.redo.last().map(|e| e.label)
    }

    /// Identifier of the current state (0 = the state the history started from).
    /// Compare with a remembered value to know whether the document is modified.
    pub fn state_id(&self) -> u64 {
        self.undo.back().map_or(0, |e| e.id)
    }

    /// Commits a finished pixel edit. Returns `false` if nothing actually changed.
    pub fn push_pixels(&mut self, doc: &Document, label: &'static str, rec: PixelRecorder) -> bool {
        match rec.into_change(doc) {
            Some(change) => {
                self.push(label, vec![change]);
                true
            }
            None => false,
        }
    }

    /// Commits a structural change given the state before it (the current document is "after").
    pub fn push_state(&mut self, doc: &Document, label: &'static str, before: DocState) {
        let after = doc.state();
        if after == before {
            return;
        }
        self.push(label, vec![Change::State { before: Box::new(before), after: Box::new(after) }]);
    }

    fn push(&mut self, label: &'static str, changes: Vec<Change>) {
        let bytes = changes.iter().map(Change::bytes).sum();
        let id = self.next_id;
        self.next_id += 1;
        self.redo.clear();
        self.undo.push_back(Entry { id, label, changes, bytes });
        self.trim();
    }

    fn trim(&mut self) {
        let mut total: usize = self.undo.iter().map(|e| e.bytes).sum();
        while self.undo.len() > self.limit || (total > self.budget && self.undo.len() > 1) {
            if let Some(e) = self.undo.pop_front() {
                total -= e.bytes;
            }
        }
    }

    /// Undoes one step; returns the dirty region (empty if there was nothing to undo).
    pub fn undo(&mut self, doc: &mut Document) -> IRect {
        let Some(entry) = self.undo.pop_back() else { return IRect::EMPTY };
        let mut dirty = IRect::EMPTY;
        for c in entry.changes.iter().rev() {
            dirty = dirty.union(&c.apply(doc, true));
        }
        self.redo.push(entry);
        dirty
    }

    pub fn redo(&mut self, doc: &mut Document) -> IRect {
        let Some(entry) = self.redo.pop() else { return IRect::EMPTY };
        let mut dirty = IRect::EMPTY;
        for c in &entry.changes {
            dirty = dirty.union(&c.apply(doc, false));
        }
        self.undo.push_back(entry);
        dirty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{Px, Rgba};

    fn paint(doc: &mut Document, hist: &mut History, x: i32, y: i32) {
        let id = doc.active_id();
        let mut rec = PixelRecorder::new(id);
        rec.touch(doc, IRect::new(x, y, 1, 1));
        doc.raster_mut(id).unwrap().put(x, y, Rgba::BLACK.premultiplied());
        assert!(hist.push_pixels(doc, "dot", rec));
    }

    #[test]
    fn undo_redo_pixels() {
        let mut doc = Document::new(200, 100, "L");
        let mut h = History::default();
        paint(&mut doc, &mut h, 10, 10);
        paint(&mut doc, &mut h, 150, 90);
        let id = doc.active_id();
        h.undo(&mut doc);
        assert_eq!(doc.layer(id).unwrap().raster.get(150, 90), Some(Px::TRANSPARENT));
        assert_eq!(doc.layer(id).unwrap().raster.get(10, 10), Some(Rgba::BLACK.premultiplied()));
        h.redo(&mut doc);
        assert_eq!(doc.layer(id).unwrap().raster.get(150, 90), Some(Rgba::BLACK.premultiplied()));
    }

    #[test]
    fn no_change_is_not_recorded() {
        let doc = Document::new(10, 10, "L");
        let mut h = History::default();
        let mut rec = PixelRecorder::new(doc.active_id());
        rec.touch(&doc, IRect::new(0, 0, 5, 5));
        assert!(!h.push_pixels(&doc, "noop", rec));
        assert!(!h.can_undo());
    }

    #[test]
    fn limit_is_respected() {
        let mut doc = Document::new(16, 16, "L");
        let mut h = History::new(5, usize::MAX);
        for i in 0..10 {
            paint(&mut doc, &mut h, i, 0);
        }
        let mut n = 0;
        while h.can_undo() {
            h.undo(&mut doc);
            n += 1;
        }
        assert_eq!(n, 5);
        // The first five dots stay (they fell out of history).
        let r = &doc.active_layer().raster;
        assert_eq!(r.get(4, 0), Some(Rgba::BLACK.premultiplied()));
        assert_eq!(r.get(5, 0), Some(Px::TRANSPARENT));
    }

    #[test]
    fn state_change_roundtrip_and_modified_tracking() {
        let mut doc = Document::new(10, 10, "L");
        let mut h = History::default();
        let saved = h.state_id();
        let before = doc.state();
        doc.resize_canvas(20, 5, 0, 0);
        h.push_state(&doc, "resize", before);
        assert_ne!(h.state_id(), saved);
        h.undo(&mut doc);
        assert_eq!((doc.width(), doc.height()), (10, 10));
        assert_eq!(h.state_id(), saved);
        h.redo(&mut doc);
        assert_eq!((doc.width(), doc.height()), (20, 5));
    }

    #[test]
    fn new_edit_clears_redo() {
        let mut doc = Document::new(8, 8, "L");
        let mut h = History::default();
        paint(&mut doc, &mut h, 1, 1);
        h.undo(&mut doc);
        assert!(h.can_redo());
        paint(&mut doc, &mut h, 2, 2);
        assert!(!h.can_redo());
    }
}
