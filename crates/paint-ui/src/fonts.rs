//! Embedded fonts: IBM Plex Sans (OFL) for text, Lucide (ISC) for icons.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily, FontId};

const PLEX_REGULAR: &[u8] = include_bytes!("../assets/fonts/IBMPlexSans-Regular.ttf");
const PLEX_MEDIUM: &[u8] = include_bytes!("../assets/fonts/IBMPlexSans-Medium.ttf");
const PLEX_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/IBMPlexSans-SemiBold.ttf");
const LUCIDE: &[u8] = include_bytes!("../assets/fonts/lucide-subset.ttf");

pub const MEDIUM: &str = "plex-medium";
pub const SEMIBOLD: &str = "plex-semibold";
pub const ICONS: &str = "icons";

pub fn install(ctx: &egui::Context) {
    let mut defs = FontDefinitions::empty();
    let add = |defs: &mut FontDefinitions, name: &str, bytes: &'static [u8]| {
        defs.font_data.insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    };
    add(&mut defs, "plex-regular", PLEX_REGULAR);
    add(&mut defs, MEDIUM, PLEX_MEDIUM);
    add(&mut defs, SEMIBOLD, PLEX_SEMIBOLD);
    add(&mut defs, "lucide", LUCIDE);

    defs.families.insert(FontFamily::Proportional, vec!["plex-regular".into(), "lucide".into()]);
    defs.families.insert(FontFamily::Monospace, vec!["plex-regular".into(), "lucide".into()]);
    defs.families.insert(FontFamily::Name(MEDIUM.into()), vec![MEDIUM.into(), "lucide".into()]);
    defs.families.insert(FontFamily::Name(SEMIBOLD.into()), vec![SEMIBOLD.into(), "lucide".into()]);
    defs.families.insert(FontFamily::Name(ICONS.into()), vec!["lucide".into()]);
    ctx.set_fonts(defs);

    ctx.global_style_mut(|s| {
        use egui::TextStyle::*;
        s.text_styles = [
            (Small, FontId::proportional(11.0)),
            (Body, FontId::proportional(13.0)),
            (Button, FontId::proportional(13.0)),
            (Monospace, FontId::monospace(13.0)),
            (Heading, FontId::new(18.0, FontFamily::Name(SEMIBOLD.into()))),
        ]
        .into();
    });
}

pub fn icon(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(ICONS.into()))
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(MEDIUM.into()))
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}
