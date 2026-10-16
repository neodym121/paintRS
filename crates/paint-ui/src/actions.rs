//! Commands with keyboard shortcuts. Defaults follow docs/behavior/shortcuts.md;
//! every binding can be overridden in config.toml (`[keymap]`, e.g. `save_as = "Ctrl+Shift+S"`).

use std::collections::BTreeMap;

use egui::{Key, KeyboardShortcut, Modifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Action {
    New,
    Open,
    Save,
    SaveAs,
    Print,
    ImageProperties,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    CopyVisibleLayers,
    SelectAll,
    Delete,
    Crop,
    InvertColors,
    ResizeSkew,
    Zoom100,
    ZoomIn,
    ZoomOut,
    ZoomInAlt,
    ZoomOutAlt,
    FitToWindow,
    ToggleRulers,
    ToggleGrid,
    FullScreen,
    ToolSelect,
    ToolPencil,
    ToolFill,
    ToolText,
    ToolEraser,
    ToolPicker,
    ToolMagnifier,
}

impl Action {
    pub const ALL: [Action; 33] = [
        Action::New,
        Action::Open,
        Action::Save,
        Action::SaveAs,
        Action::Print,
        Action::ImageProperties,
        Action::Undo,
        Action::Redo,
        Action::Cut,
        Action::Copy,
        Action::Paste,
        Action::CopyVisibleLayers,
        Action::SelectAll,
        Action::Delete,
        Action::Crop,
        Action::InvertColors,
        Action::ResizeSkew,
        Action::Zoom100,
        Action::ZoomIn,
        Action::ZoomOut,
        Action::ZoomInAlt,
        Action::ZoomOutAlt,
        Action::FitToWindow,
        Action::ToggleRulers,
        Action::ToggleGrid,
        Action::FullScreen,
        Action::ToolSelect,
        Action::ToolPencil,
        Action::ToolFill,
        Action::ToolText,
        Action::ToolEraser,
        Action::ToolPicker,
        Action::ToolMagnifier,
    ];

    /// Name used in config.toml.
    pub fn name(self) -> &'static str {
        match self {
            Action::New => "new",
            Action::Open => "open",
            Action::Save => "save",
            Action::SaveAs => "save_as",
            Action::Print => "print",
            Action::ImageProperties => "image_properties",
            Action::Undo => "undo",
            Action::Redo => "redo",
            Action::Cut => "cut",
            Action::Copy => "copy",
            Action::Paste => "paste",
            Action::CopyVisibleLayers => "copy_visible_layers",
            Action::SelectAll => "select_all",
            Action::Delete => "delete",
            Action::Crop => "crop",
            Action::InvertColors => "invert_colors",
            Action::ResizeSkew => "resize_skew",
            Action::Zoom100 => "zoom_100",
            Action::ZoomIn => "zoom_in",
            Action::ZoomOut => "zoom_out",
            Action::ZoomInAlt => "zoom_in_alt",
            Action::ZoomOutAlt => "zoom_out_alt",
            Action::FitToWindow => "fit_to_window",
            Action::ToggleRulers => "rulers",
            Action::ToggleGrid => "grid",
            Action::FullScreen => "full_screen",
            Action::ToolSelect => "tool_select",
            Action::ToolPencil => "tool_pencil",
            Action::ToolFill => "tool_fill",
            Action::ToolText => "tool_text",
            Action::ToolEraser => "tool_eraser",
            Action::ToolPicker => "tool_picker",
            Action::ToolMagnifier => "tool_magnifier",
        }
    }

    fn default_shortcut(self) -> KeyboardShortcut {
        let c = Modifiers::COMMAND;
        let cs = Modifiers::COMMAND | Modifiers::SHIFT;
        let n = Modifiers::NONE;
        let (m, k) = match self {
            Action::New => (c, Key::N),
            Action::Open => (c, Key::O),
            Action::Save => (c, Key::S),
            Action::SaveAs => (cs, Key::S),
            Action::Print => (c, Key::P),
            Action::ImageProperties => (c, Key::E),
            Action::Undo => (c, Key::Z),
            Action::Redo => (c, Key::Y),
            Action::Cut => (c, Key::X),
            Action::Copy => (c, Key::C),
            Action::Paste => (c, Key::V),
            Action::CopyVisibleLayers => (cs, Key::C),
            Action::SelectAll => (c, Key::A),
            Action::Delete => (n, Key::Delete),
            Action::Crop => (cs, Key::X),
            Action::InvertColors => (cs, Key::I),
            Action::ResizeSkew => (c, Key::W),
            Action::Zoom100 => (c, Key::Num0),
            Action::ZoomIn => (c, Key::PageUp),
            Action::ZoomOut => (c, Key::PageDown),
            Action::ZoomInAlt => (c, Key::Plus),
            Action::ZoomOutAlt => (c, Key::Minus),
            Action::FitToWindow => (c, Key::Num1),
            Action::ToggleRulers => (c, Key::R),
            Action::ToggleGrid => (c, Key::G),
            Action::FullScreen => (n, Key::F11),
            Action::ToolSelect => (n, Key::S),
            Action::ToolPencil => (n, Key::P),
            Action::ToolFill => (n, Key::B),
            Action::ToolText => (n, Key::T),
            Action::ToolEraser => (n, Key::E),
            Action::ToolPicker => (n, Key::I),
            Action::ToolMagnifier => (n, Key::Z),
        };
        KeyboardShortcut::new(m, k)
    }

    /// Single-letter tool keys must not fire while typing text.
    pub fn is_plain_key(self) -> bool {
        self.default_shortcut().modifiers.is_none() && !matches!(self, Action::Delete | Action::FullScreen)
    }
}

#[derive(Clone, Debug)]
pub struct Keymap {
    map: BTreeMap<Action, KeyboardShortcut>,
}

impl Keymap {
    /// Defaults with overrides from config (`"Ctrl+Shift+S"`-style strings; invalid ones are ignored).
    pub fn new(overrides: &BTreeMap<String, String>) -> Self {
        let mut map = BTreeMap::new();
        for a in Action::ALL {
            let sc = overrides
                .get(a.name())
                .and_then(|s| parse_shortcut(s))
                .unwrap_or_else(|| a.default_shortcut());
            map.insert(a, sc);
        }
        Self { map }
    }

    pub fn get(&self, a: Action) -> KeyboardShortcut {
        self.map.get(&a).copied().unwrap_or_else(|| a.default_shortcut())
    }

    /// Human-readable shortcut for menus and tooltips.
    pub fn text(&self, a: Action) -> String {
        format_shortcut(&self.get(a))
    }

    /// Consumes pressed shortcuts. Shortcuts with more modifiers are checked first so that
    /// Ctrl+Shift+S does not trigger Ctrl+S.
    pub fn pressed(&self, ctx: &egui::Context, allow_plain: bool) -> Vec<Action> {
        let mut items: Vec<(Action, KeyboardShortcut)> = self.map.iter().map(|(a, s)| (*a, *s)).collect();
        items.sort_by_key(|(_, s)| std::cmp::Reverse(modifier_count(s.modifiers)));
        let mut out = Vec::new();
        ctx.input_mut(|i| {
            for (a, sc) in items {
                if !allow_plain && a.is_plain_key() {
                    continue;
                }
                if i.consume_shortcut(&sc) {
                    out.push(a);
                }
            }
        });
        out
    }
}

fn modifier_count(m: Modifiers) -> u8 {
    m.alt as u8 + m.shift as u8 + m.command as u8 + m.ctrl as u8
}

pub fn format_shortcut(s: &KeyboardShortcut) -> String {
    let mut parts = Vec::new();
    if s.modifiers.command || s.modifiers.ctrl {
        parts.push("Ctrl".to_owned());
    }
    if s.modifiers.shift {
        parts.push("Shift".to_owned());
    }
    if s.modifiers.alt {
        parts.push("Alt".to_owned());
    }
    let key = match s.logical_key {
        Key::PageUp => "PageUp".to_owned(),
        Key::PageDown => "PageDown".to_owned(),
        Key::Plus => "+".to_owned(),
        Key::Minus => "-".to_owned(),
        k => k.name().to_owned(),
    };
    parts.push(key);
    parts.join("+")
}

pub fn parse_shortcut(s: &str) -> Option<KeyboardShortcut> {
    let mut m = Modifiers::NONE;
    let mut key = None;
    for part in s.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "cmd" | "command" => m |= Modifiers::COMMAND,
            "shift" => m |= Modifiers::SHIFT,
            "alt" => m |= Modifiers::ALT,
            "plus" => key = Some(Key::Plus),
            "minus" => key = Some(Key::Minus),
            _ => key = Some(Key::from_name(part)?),
        }
    }
    if s.trim_end().ends_with("++") {
        key = Some(Key::Plus);
    }
    key.map(|k| KeyboardShortcut::new(m, k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_format() {
        let sc = parse_shortcut("Ctrl+Shift+S").unwrap();
        assert_eq!(sc, KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::S));
        assert_eq!(format_shortcut(&sc), "Ctrl+Shift+S");
        assert_eq!(parse_shortcut("F11").unwrap().logical_key, Key::F11);
        assert_eq!(parse_shortcut("Ctrl+PageUp").unwrap().logical_key, Key::PageUp);
        assert!(parse_shortcut("Ctrl+Nonsense").is_none());
    }

    #[test]
    fn overrides_apply() {
        let mut o = BTreeMap::new();
        o.insert("save_as".to_owned(), "Ctrl+Alt+S".to_owned());
        let k = Keymap::new(&o);
        assert_eq!(k.text(Action::SaveAs), "Ctrl+Alt+S");
        assert_eq!(k.text(Action::Save), "Ctrl+S");
    }
}
