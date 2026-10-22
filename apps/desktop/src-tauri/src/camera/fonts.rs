//! System fonts for the Rust renderer (ADR-0014: no bundled font files).

use std::fmt;
use std::path::PathBuf;

use ab_glyph::FontVec;

use crate::camera::theme::FontId;

#[derive(Debug, PartialEq, Eq)]
pub enum FontError {
    NoSystemFont,
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontError::NoSystemFont => write!(f, "no usable system font was found"),
        }
    }
}

impl std::error::Error for FontError {}

/// One parsed face per font id, plus a fallback for ids the system does not provide.
pub struct Fonts {
    entries: Vec<(FontId, FontVec)>,
}

impl Fonts {
    /// Loads the system files that roughly match each `FontId` (ADR-0006, ADR-0014).
    /// Missing ids fall back to the first font that could be loaded.
    pub fn load_system() -> Result<Fonts, FontError> {
        let dirs = font_dirs();
        let mut entries = Vec::new();
        for id in FontId::ALL {
            if let Some(font) = candidates(id)
                .iter()
                .find_map(|name| load_first(&dirs, name))
            {
                entries.push((id, font));
            }
        }
        if entries.is_empty() {
            return Err(FontError::NoSystemFont);
        }
        Ok(Fonts { entries })
    }

    pub fn get(&self, id: FontId) -> &FontVec {
        self.entries
            .iter()
            .find(|(entry, _)| *entry == id)
            .map(|(_, font)| font)
            .unwrap_or(&self.entries[0].1)
    }
}

fn load_first(dirs: &[PathBuf], name: &str) -> Option<FontVec> {
    dirs.iter().find_map(|dir| {
        let bytes = std::fs::read(dir.join(name)).ok()?;
        FontVec::try_from_vec(bytes).ok()
    })
}

fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(windows) {
        if let Ok(windir) = std::env::var("WINDIR") {
            dirs.push(PathBuf::from(windir).join("Fonts"));
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            dirs.push(
                PathBuf::from(local)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Fonts"),
            );
        }
    } else if cfg!(target_os = "macos") {
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        dirs.push(PathBuf::from("/Library/Fonts"));
    } else {
        dirs.push(PathBuf::from("/usr/share/fonts/truetype/dejavu"));
        dirs.push(PathBuf::from("/usr/share/fonts/truetype/liberation"));
        dirs.push(PathBuf::from("/usr/share/fonts/truetype"));
        dirs.push(PathBuf::from("/usr/share/fonts"));
    }
    dirs
}

fn candidates(id: FontId) -> &'static [&'static str] {
    match id {
        FontId::System => &[
            "segoeui.ttf",
            "arial.ttf",
            "DejaVuSans.ttf",
            "LiberationSans-Regular.ttf",
        ],
        FontId::Serif => &[
            "georgia.ttf",
            "times.ttf",
            "DejaVuSerif.ttf",
            "LiberationSerif-Regular.ttf",
        ],
        FontId::Mono => &[
            "consola.ttf",
            "cour.ttf",
            "DejaVuSansMono.ttf",
            "LiberationMono-Regular.ttf",
        ],
        FontId::Rounded => &["seguisb.ttf", "segoeui.ttf", "arial.ttf", "DejaVuSans.ttf"],
        FontId::Condensed => &[
            "bahnschrift.ttf",
            "arialn.ttf",
            "arial.ttf",
            "DejaVuSansCondensed.ttf",
        ],
        FontId::Handwritten => &["segoepr.ttf", "comic.ttf", "DejaVuSans.ttf"],
        FontId::Display => &[
            "impact.ttf",
            "ariblk.ttf",
            "arialbd.ttf",
            "DejaVuSans-Bold.ttf",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ab_glyph::Font;

    #[test]
    fn loads_a_font_for_every_font_id() {
        if let Ok(fonts) = Fonts::load_system() {
            for id in FontId::ALL {
                let font = fonts.get(id);
                assert!(font.units_per_em().is_some(), "{id:?} has units per em");
            }
        }
    }
}
