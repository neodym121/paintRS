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
