//! Tool identifiers shared by the core and the UI.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tool {
    Select,
    Pencil,
    Fill,
    Text,
    Eraser,
    Picker,
    Magnifier,
    /// The brush selected in the Brushes drop-down. The original starts with this tool.
    #[default]
    Brush,
    /// The shape selected in the Shapes gallery.
    Shape,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectionKind {
    #[default]
    Rectangle,
    FreeForm,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BrushKind {
    #[default]
    Brush,
    CalligraphyBrush,
    CalligraphyPen,
    Airbrush,
    Oil,
    Crayon,
    Marker,
    NaturalPencil,
    Watercolor,
}

impl BrushKind {
    pub const ALL: [BrushKind; 9] = [
        BrushKind::Brush,
        BrushKind::CalligraphyBrush,
        BrushKind::CalligraphyPen,
        BrushKind::Airbrush,
        BrushKind::Oil,
        BrushKind::Crayon,
        BrushKind::Marker,
        BrushKind::NaturalPencil,
        BrushKind::Watercolor,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&k| k == self).unwrap_or(0)
    }

    /// Smallest size the original allows for this brush (docs/behavior/tools.md).
    pub fn min_size(self) -> u32 {
        match self {
            BrushKind::Brush
            | BrushKind::CalligraphyBrush
            | BrushKind::CalligraphyPen
            | BrushKind::NaturalPencil => 1,
            _ => 4,
        }
    }
}

/// Largest brush/pencil/eraser size in the original.
pub const MAX_SIZE: u32 = 248;

/// The size slider has 91 positions; ↑/↓ move one position. Measured on the original
/// (docs/behavior/tools.md): 1 px per position up to 24 px, then 2 px up to 104 px,
/// 4 px up to 172 px and 8 px up to the 248 px maximum.
pub const SIZE_POSITIONS: u32 = 91;

/// Size in pixels for a size-slider position (1…[`SIZE_POSITIONS`]).
pub fn size_for_position(pos: u32) -> u32 {
    let p = pos.clamp(1, SIZE_POSITIONS);
    match p {
        0..=24 => p,
        25..=64 => 24 + 2 * (p - 24),
        65..=81 => 104 + 4 * (p - 64),
        _ => (172 + 8 * (p - 81)).min(MAX_SIZE),
    }
}

/// Smallest slider position whose size is at least `size`.
pub fn position_for_size(size: u32) -> u32 {
    (1..=SIZE_POSITIONS).find(|&p| size_for_position(p) >= size).unwrap_or(SIZE_POSITIONS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_scale_matches_the_original() {
        // (position, size) pairs read from the original's slider tooltips.
        let measured = [
            (1, 1),
            (3, 3),
            (10, 10),
            (20, 20),
            (24, 24),
            (25, 26),
            (26, 28),
            (30, 36),
            (40, 56),
            (45, 66),
            (46, 68),
            (60, 96),
            (63, 102),
            (64, 104),
            (65, 108),
            (66, 112),
            (80, 168),
            (81, 172),
            (82, 180),
            (89, 236),
            (90, 244),
            (91, 248),
        ];
        for (p, s) in measured {
            assert_eq!(size_for_position(p), s, "position {p}");
        }
        assert_eq!(position_for_size(8), 8);
        assert_eq!(position_for_size(248), 91);
        assert_eq!(position_for_size(100), 62);
    }
}
