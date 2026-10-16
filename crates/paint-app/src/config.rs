//! config.toml in the user's config directory (e.g. %APPDATA%\PaintRS\config\config.toml).

use std::path::PathBuf;

use paint_ui::Settings;

fn path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "PaintRS").map(|d| d.config_dir().join("config.toml"))
}

pub fn load() -> Settings {
    let Some(path) = path() else { return Settings::default() };
    match std::fs::read_to_string(&path) {
        Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
            tracing::warn!("ignoring broken config {}: {e}", path.display());
            Settings::default()
        }),
        Err(_) => Settings::default(),
    }
}

pub fn save(settings: &Settings) {
    let Some(path) = path() else { return };
    let result = (|| -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, toml::to_string_pretty(settings)?)?;
        Ok(())
    })();
    if let Err(e) = result {
        tracing::warn!("could not save config {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip_through_toml() {
        let mut s = Settings::default();
        s.keymap.insert("save_as".into(), "Ctrl+Alt+S".into());
        s.push_recent(PathBuf::from("C:/a.png"));
        let text = toml::to_string_pretty(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn partial_config_uses_defaults() {
        let s: Settings = toml::from_str("theme = \"light\"").unwrap();
        assert_eq!(s.theme, paint_ui::ThemeMode::Light);
        assert!(s.show_status_bar);
    }
}
