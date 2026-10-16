//! Zoom levels, as measured in the original (docs/behavior/view.md).
//! Zoom is a scale factor: 1.0 = 100%.

pub const MIN: f32 = 0.10;
pub const MAX: f32 = 10.0;

/// Presets of the zoom drop-down, top to bottom.
pub const PRESETS: [f32; 14] = [10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.75, 0.5, 0.25, 0.125];

/// Step (in percent) used around `percent`: 10 below 200%, 25 below 600%, 50 above.
fn step_for(percent: f32) -> f32 {
    if percent < 200.0 {
        10.0
    } else if percent < 600.0 {
        25.0
    } else {
        50.0
    }
}

pub fn clamp(z: f32) -> f32 {
    if z.is_finite() {
        z.clamp(MIN, MAX)
    } else {
        1.0
    }
}

/// Next larger zoom level (Ctrl+PageUp, magnifier click, "+" button).
pub fn zoom_in(z: f32) -> f32 {
    let p = (z * 100.0 * 1000.0).round() / 1000.0;
    let step = step_for(p);
    let next = ((p / step).floor() + 1.0) * step;
    clamp(next / 100.0)
}

/// Next smaller zoom level (Ctrl+PageDown, magnifier right click, "−" button).
pub fn zoom_out(z: f32) -> f32 {
    let p = (z * 100.0 * 1000.0).round() / 1000.0;
    let below = p - 0.001;
    let step = step_for(below);
    let prev = (below / step).floor() * step;
    clamp(prev / 100.0)
}

/// Zoom that fits `content` into `available` leaving `margin` on each side, rounded down to 0.1%.
pub fn fit(content: (f32, f32), available: (f32, f32), margin: f32) -> f32 {
    let (cw, ch) = content;
    let (aw, ah) = ((available.0 - 2.0 * margin).max(1.0), (available.1 - 2.0 * margin).max(1.0));
    if cw <= 0.0 || ch <= 0.0 {
        return 1.0;
    }
    let z = (aw / cw).min(ah / ch);
    clamp((z * 1000.0).floor() / 1000.0)
}

/// Text for the status bar: "100%", "12.5%", "235.5%".
pub fn label(z: f32) -> String {
    let p = (z * 1000.0).round() / 10.0;
    if (p - p.round()).abs() < 0.05 {
        format!("{}%", p.round() as i64)
    } else {
        format!("{p:.1}%")
    }
}

/// Parses user input like "150", "150%", "12,5 %".
pub fn parse(text: &str) -> Option<f32> {
    let t = text.trim().trim_end_matches('%').trim().replace(',', ".");
    let p: f32 = t.parse().ok()?;
    (p > 0.0).then(|| clamp(p / 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pct(z: f32) -> i32 {
        (z * 100.0).round() as i32
    }

    #[test]
    fn steps_match_the_original() {
        // 100 → 150 in five steps; 200 → 325 in five; 500 → 525 → 550; 100 → 800 in thirty.
        let mut z = 1.0;
        for _ in 0..5 {
            z = zoom_in(z);
        }
        assert_eq!(pct(z), 150);
        let mut z = 2.0;
        for _ in 0..5 {
            z = zoom_in(z);
        }
        assert_eq!(pct(z), 325);
        assert_eq!(pct(zoom_in(5.0)), 525);
        let mut z = 1.0;
        for _ in 0..30 {
            z = zoom_in(z);
        }
        assert_eq!(pct(z), 800);
        assert_eq!(pct(zoom_out(10.0)), 950);
        assert_eq!(pct(zoom_out(2.0)), 190);
        assert_eq!(pct(zoom_out(6.0)), 575);
    }

    #[test]
    fn limits() {
        assert_eq!(pct(zoom_in(10.0)), 1000);
        assert_eq!(pct(zoom_out(0.1)), 10);
        let mut z = 1.0;
        for _ in 0..9 {
            z = zoom_out(z);
        }
        assert_eq!(pct(z), 10);
    }

    #[test]
    fn fractional_levels_snap_to_grid() {
        assert_eq!(pct(zoom_in(2.355)), 250);
        assert_eq!(pct(zoom_out(2.355)), 225);
        assert_eq!(pct(zoom_in(0.125)), 20);
    }

    #[test]
    fn labels_and_parsing() {
        assert_eq!(label(1.0), "100%");
        assert_eq!(label(0.125), "12.5%");
        assert_eq!(label(2.355), "235.5%");
        assert_eq!(parse("150 %").map(pct), Some(150));
        assert_eq!(parse("12,5").map(|z| (z * 1000.0).round() as i32), Some(125));
        assert_eq!(parse("abc"), None);
        assert_eq!(parse("99999").map(pct), Some(1000));
    }

    #[test]
    fn fit_rounds_down() {
        // 400×300 into the original's viewport gave 235.5%.
        let z = fit((400.0, 300.0), (1920.0, 809.0), 51.25);
        assert_eq!((z * 1000.0).round() as i32, 2355);
    }
}
