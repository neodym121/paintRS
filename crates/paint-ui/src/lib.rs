//! PaintRS user interface (egui): ribbon, canvas widget, dialogs.

pub mod actions;
pub mod canvas;
mod color_dialog;
mod dialogs;
pub mod editor;
pub mod fonts;
pub mod i18n;
pub mod icons;
mod menubar;
mod ribbon;
pub mod settings;
mod settings_page;
mod shapes;
mod statusbar;
pub mod theme;
mod tools;
pub mod widgets;

pub use editor::Editor;
pub use i18n::Lang;
pub use settings::{Settings, Smoothing};
pub use theme::ThemeMode;

/// One-time setup of fonts (call before the first frame).
pub fn setup(ctx: &egui::Context) {
    fonts::install(ctx);
    // Ctrl+0 / Ctrl+± belong to the canvas zoom, not to egui's UI scale.
    ctx.options_mut(|o| o.zoom_with_keyboard = false);
}
