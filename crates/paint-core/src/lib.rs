//! PaintRS core: document model, raster operations, history and file formats.
//! Has no GUI dependencies so it can be tested and reused (CLI, WASM).

pub mod color;
pub mod document;
pub mod draw;
pub mod geom;
pub mod history;
pub mod io;
pub mod raster;
pub mod shape;
pub mod tool;
pub mod zoom;

pub use color::{Px, Rgba};
pub use document::{Background, DocState, Document, Layer, LayerId};
pub use geom::IRect;
pub use history::{History, PixelRecorder};
pub use raster::Raster;
