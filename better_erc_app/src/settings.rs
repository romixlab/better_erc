//! The app's settings: what a person might want different, each a field with its default and a doc line, in
//! `$BERC_CONFIG`, else `$XDG_CONFIG_HOME/better_erc/config.toml`, else `~/.config/better_erc/config.toml`. A missing
//! file or field takes the default; the app shows and saves them (Footprint review, Settings).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    pub footprint_review: FootprintReviewSettings,
}

/// The footprint review tab
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FootprintReviewSettings {
    /// How often the footprint blinks on and off while blinking, times per second
    pub blink_hz: f32,
    /// How opaque the footprint is over the datasheet at first, 0 (not seen) to 1 (covers the drawing)
    pub opacity: f32,
    /// Words that find the datasheet page with the land pattern, best first; case, spaces and punctuation don't
    /// matter
    pub page_search_words: Vec<String>,
    /// The finest a page is drawn at, pixels per PDF point (1/72 inch); finer needs more memory
    pub max_render_scale: f32,
    /// Folders with the Liberation fonts, which stand in for the PDF standard fonts (Helvetica, Times, Courier)
    /// a datasheet uses without embedding them; without them, egui's own font stands in
    pub font_dirs: Vec<PathBuf>,
    /// A command that searches a local document library for a datasheet: the program and its arguments; the
    /// search words are added at the end. It prints a JSON list of `{"path": ..., "page": ...}`. Empty: no
    /// library search.
    pub library_search: Vec<String>,
}

impl Default for FootprintReviewSettings {
    fn default() -> Self {
        FootprintReviewSettings {
            blink_hz: 2.0,
            opacity: 0.55,
            page_search_words: [
                "recommended land pattern",
                "recommended footprint",
                "land pattern",
                "soldering footprint",
                "footprint",
                "pcb layout",
                "mounting pad",
                "solder pad",
                "package outline",
                "package dimensions",
                "mechanical data",
            ]
            .map(String::from)
            .to_vec(),
            max_render_scale: 8.0,
            font_dirs: [
                "/usr/share/fonts/liberation",
                "/usr/share/fonts/truetype/liberation",
                "/usr/share/fonts/liberation-sans",
                "/usr/share/fonts/TTF",
            ]
            .map(PathBuf::from)
            .to_vec(),
            library_search: ["tpm", "lib", "search", "--json", "-n", "10"]
                .map(String::from)
                .to_vec(),
        }
    }
}

impl Settings {
    /// The settings file
    pub fn file() -> Option<PathBuf> {
        if let Some(p) = std::env::var_os("BERC_CONFIG") {
            return Some(p.into());
        }
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("better_erc/config.toml"))
    }

    /// The settings from the file, the defaults where it has none; the error says why the file was not read
    pub fn load() -> (Settings, Option<String>) {
        let Some(file) = Settings::file() else {
            return (Settings::default(), None);
        };
        match std::fs::read_to_string(&file) {
            Ok(text) => match Settings::parse(&text) {
                Ok(s) => (s, None),
                Err(e) => (
                    Settings::default(),
                    Some(format!("{}: {e}; using the defaults", file.display())),
                ),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Settings::default(), None),
            Err(e) => (
                Settings::default(),
                Some(format!(
                    "can't read {}: {e}; using the defaults",
                    file.display()
                )),
            ),
        }
    }

    pub fn parse(text: &str) -> Result<Settings, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }

    pub fn to_toml(&self) -> String {
        toml::to_string(self).expect("settings serialise")
    }

    /// Writes the settings file (a temporary file, then renamed over it)
    pub fn save(&self) -> Result<PathBuf, String> {
        let file =
            Settings::file().ok_or("neither BERC_CONFIG, XDG_CONFIG_HOME nor HOME is set")?;
        if let Some(d) = file.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("can't make {}: {e}", d.display()))?;
        }
        let tmp = file.with_extension("toml.tmp");
        std::fs::write(&tmp, self.to_toml())
            .and_then(|_| std::fs::rename(&tmp, &file))
            .map_err(|e| format!("can't write {}: {e}", file.display()))?;
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file with one field changed keeps the defaults for the rest, and the settings go through the file as
    /// they are
    #[test]
    fn partial_file_and_round_trip() {
        let s = Settings::parse("[footprint_review]\nblink_hz = 5.0\n").unwrap();
        assert_eq!(s.footprint_review.blink_hz, 5.0);
        assert_eq!(
            s.footprint_review.page_search_words,
            FootprintReviewSettings::default().page_search_words
        );
        assert_eq!(Settings::parse(&s.to_toml()).unwrap(), s);
        assert!(Settings::parse("[footprint_review]\nblink_hz = \"fast\"\n").is_err());
    }
}
