//! User settings persisted by the app (config.toml).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::i18n::Lang;
use crate::theme::ThemeMode;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Smoothing {
    /// Smooth (linear + mipmaps) when zoomed out; crisp pixels and a pixel grid from 800%.
    #[default]
    Auto,
    Always,
    Never,
}

pub const MAX_RECENT: usize = 10;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeMode,
    pub language: Lang,
    pub smoothing: Smoothing,
    /// Size of the last canvas; new documents start with it (as the original does).
    pub last_canvas_size: (u32, u32),
    pub show_status_bar: bool,
    pub show_rulers: bool,
    pub show_grid: bool,
    /// Off by default: the original draws shapes without anti-aliasing.
    pub antialias_shapes: bool,
    pub recent_files: Vec<PathBuf>,
    /// Action name → shortcut text, e.g. `save_as = "Ctrl+Shift+S"` (overrides defaults).
    pub keymap: std::collections::BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            language: Lang::Ru,
            smoothing: Smoothing::Auto,
            last_canvas_size: (1152, 648),
            show_status_bar: true,
            show_rulers: false,
            show_grid: false,
            antialias_shapes: false,
            recent_files: Vec::new(),
            keymap: Default::default(),
        }
    }
}

impl Settings {
    pub fn push_recent(&mut self, path: PathBuf) {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(MAX_RECENT);
    }
}
