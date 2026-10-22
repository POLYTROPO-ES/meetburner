//! Counter themes and layout enums shared with the web UI (ADR-0006).
//! The colour data itself is generated from `packages/core/src/themes.ts` (ADR-0014).

use crate::camera::themes_data;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontId {
    System,
    Serif,
    Mono,
    Rounded,
    Condensed,
    Handwritten,
    Display,
}

impl FontId {
    pub const ALL: [FontId; 7] = [
        FontId::System,
        FontId::Serif,
        FontId::Mono,
        FontId::Rounded,
        FontId::Condensed,
        FontId::Handwritten,
        FontId::Display,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub background: (u8, u8, u8),
    pub background_alt: (u8, u8, u8),
    pub text: (u8, u8, u8),
    pub muted: (u8, u8, u8),
    pub accent: (u8, u8, u8),
    pub font: FontId,
    pub weight: f32,
    pub radius: f32,
    pub decoration: &'static str,
}

/// `fullscreen` when no source webcam, `widget` over a webcam (ADR-0006).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Fullscreen,
    Widget,
}

/// What to do when the cost is too wide: abbreviate (`1.23M`) or shrink the font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overflow {
    Compact,
    Shrink,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontChoice {
    Theme,
    Font(FontId),
}

impl FontChoice {
    pub fn resolve(self, theme: &Theme) -> FontId {
        match self {
            FontChoice::Theme => theme.font,
            FontChoice::Font(font) => font,
        }
    }
}

/// Theme by id; `"seasons"` resolves to the current meteorological season (northern hemisphere).
/// Unknown ids fall back to `classic`, like `resolveTheme` in the web UI.
pub fn theme(id: &str, now_ms: i64) -> Theme {
    let key = if id == "seasons" {
        season_of(now_ms)
    } else {
        id
    };
    find(key).unwrap_or_else(|| find("classic").expect("generated themes include classic"))
}

fn find(id: &str) -> Option<Theme> {
    themes_data::THEMES
        .iter()
        .find(|(key, _)| *key == id)
        .map(|(_, theme)| *theme)
}

/// Meteorological season for a UTC timestamp: spring (Mar–May), summer (Jun–Aug),
/// autumn (Sep–Nov), winter (Dec–Feb).
pub fn season_of(now_ms: i64) -> &'static str {
    match utc_month(now_ms) {
        3..=5 => "spring",
        6..=8 => "summer",
        9..=11 => "autumn",
        _ => "winter",
    }
}

fn utc_month(epoch_ms: i64) -> u32 {
    let days = epoch_ms.div_euclid(86_400_000);
    civil_from_days(days).1
}

/// Howard Hinnant's `civil_from_days`: (year, month, day) from days since 1970-01-01.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        if month <= 2 { year + 1 } else { year },
        month as u32,
        day as u32,
    )
}

pub fn parse_layout(value: &str) -> Layout {
    match value {
        "widget" => Layout::Widget,
        _ => Layout::Fullscreen,
    }
}

pub fn parse_overflow(value: &str) -> Overflow {
    match value {
        "shrink" => Overflow::Shrink,
        _ => Overflow::Compact,
    }
}

pub fn parse_font_id(value: &str) -> Option<FontId> {
    match value {
        "system" => Some(FontId::System),
        "serif" => Some(FontId::Serif),
        "mono" => Some(FontId::Mono),
        "rounded" => Some(FontId::Rounded),
        "condensed" => Some(FontId::Condensed),
        "handwritten" => Some(FontId::Handwritten),
        "display" => Some(FontId::Display),
        _ => None,
    }
}

/// `"theme"` (or anything unknown) means "use the theme font".
pub fn parse_font_choice(value: &str) -> FontChoice {
    match parse_font_id(value) {
        Some(font) => FontChoice::Font(font),
        None => FontChoice::Theme,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-01-15, 2026-04-15, 2026-07-15 and 2026-10-15 at 12:00 UTC.
    const JANUARY: i64 = 1_768_478_400_000;
    const APRIL: i64 = 1_776_254_400_000;
    const JULY: i64 = 1_784_116_800_000;
    const OCTOBER: i64 = 1_792_065_600_000;

    #[test]
    fn resolves_known_themes_by_id() {
        assert_eq!(theme("classic", JANUARY).background, (0x0f, 0x11, 0x15));
        assert_eq!(theme("halloween", JANUARY).accent, (0xff, 0x75, 0x18));
        assert_eq!(theme("halloween", JANUARY).font, FontId::Display);
        assert_eq!(theme("halloween", JANUARY).decoration, "🎃🦇");
        assert_eq!(theme("halloween", JANUARY).radius, 16.0);
        assert_eq!(theme("halloween", JANUARY).weight, 700.0);
    }

    #[test]
    fn unknown_themes_fall_back_to_classic() {
        assert_eq!(theme("nope", JANUARY), theme("classic", JANUARY));
    }

    #[test]
    fn seasons_pick_the_meteorological_season() {
        assert_eq!(season_of(JANUARY), "winter");
        assert_eq!(season_of(APRIL), "spring");
        assert_eq!(season_of(JULY), "summer");
        assert_eq!(season_of(OCTOBER), "autumn");
        assert_eq!(theme("seasons", JULY), theme("summer", JANUARY));
        assert_eq!(theme("seasons", OCTOBER), theme("autumn", JANUARY));
        assert_ne!(theme("seasons", JULY), theme("seasons", JANUARY));
    }

    #[test]
    fn font_choice_falls_back_to_the_theme_font() {
        let classic = theme("classic", JANUARY);
        assert_eq!(FontChoice::Theme.resolve(&classic), FontId::System);
        assert_eq!(
            FontChoice::Font(FontId::Mono).resolve(&classic),
            FontId::Mono
        );
    }

    #[test]
    fn untrusted_layout_and_overflow_strings_are_normalised() {
        assert_eq!(parse_layout("widget"), Layout::Widget);
        assert_eq!(parse_layout("nonsense"), Layout::Fullscreen);
        assert_eq!(parse_overflow("shrink"), Overflow::Shrink);
        assert_eq!(parse_overflow("nonsense"), Overflow::Compact);
    }

    #[test]
    fn font_choices_are_parsed_and_fall_back_to_the_theme() {
        assert_eq!(parse_font_choice("theme"), FontChoice::Theme);
        assert_eq!(parse_font_choice("mono"), FontChoice::Font(FontId::Mono));
        assert_eq!(parse_font_choice("comic-sans"), FontChoice::Theme);
        assert_eq!(parse_font_id("impact"), None);
    }
}
