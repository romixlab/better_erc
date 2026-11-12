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
    /// How the footprint is mixed with the page at first: `normal`, `multiply`, `screen` or `difference`
    pub blend: BlendMode,
    /// The footprint's colours at first, per layer group, `#rrggbb`. Copper, mask and paste have no picker in
    /// the tab (one is shown at a time, each a fixed colour): change them here
    pub colors: LayerColors,
    /// How much closer the ruler key zooms in on a ruler end than the view it starts from
    pub ruler_zoom: f32,
    /// Whether the page is shown inverted (white-ish on black) at first
    pub invert: bool,
    /// The canvas' keys, by egui's key names (`F`, `Space`, `F5`, ...)
    pub keys: Keys,
}

/// The footprint review canvas' keys (while the pointer is over it), by egui's key names; arrows nudge the
/// footprint, Page Up and Page Down turn pages, Escape drops the ruler step
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keys {
    /// The page at the view's full width
    pub full_width: String,
    /// The ruler: one end at the pointer, then the other
    pub ruler: String,
    /// Zoom in on one ruler end to place it exactly, then on the other, then back
    pub ruler_zoom: String,
    /// The ruler's length typed in: the length field takes the keyboard, Enter sets the scale
    pub scale: String,
    /// Turn the footprint by 90°
    pub rotate: String,
    /// Bottom view: the footprint mirrored
    pub mirror: String,
    /// Blink the footprint on and off
    pub blink: String,
    /// Cycles which of copper, mask or paste is shown (the three shown one at a time)
    pub cu_layer: String,
    /// Inverts the page (white-ish on black)
    pub invert: String,
}

impl Default for Keys {
    fn default() -> Self {
        Keys {
            full_width: "F".into(),
            ruler: "R".into(),
            ruler_zoom: "Z".into(),
            scale: "S".into(),
            rotate: "T".into(),
            mirror: "M".into(),
            blink: "B".into(),
            cu_layer: "C".into(),
            invert: "I".into(),
        }
    }
}

/// How the footprint's colour is mixed with the page under it
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlendMode {
    /// The footprint over the page, by its opacity
    #[default]
    Normal,
    /// Page times footprint: the drawing's lines stay dark through a light footprint
    Multiply,
    /// The inverse of multiply: light footprint shapes on a dark page stay light
    Screen,
    /// The difference of page and footprint: a white footprint over white paper goes black, so a footprint that
    /// covers the drawing's pads exactly leaves no light edge
    Difference,
}

impl BlendMode {
    pub const ALL: [BlendMode; 4] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Difference,
    ];

    pub fn name(self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Screen => "Screen",
            BlendMode::Difference => "Difference",
        }
    }
}

/// A colour as `#rrggbb` in the settings file
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub [u8; 3]);

impl Rgb {
    pub fn parse(s: &str) -> Option<Rgb> {
        let h = s.trim().strip_prefix('#')?;
        if h.len() != 6 {
            return None;
        }
        let v = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
        Some(Rgb([v(0)?, v(2)?, v(4)?]))
    }
}

impl std::fmt::Display for Rgb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [r, g, b] = self.0;
        write!(f, "#{r:02x}{g:02x}{b:02x}")
    }
}

impl Serialize for Rgb {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Rgb::parse(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("`{s}` is no colour: #rrggbb")))
    }
}

/// The footprint's colour per layer group. Copper, mask and paste are shown one at a time (not a picker in the
/// tab: a datasheet page is white, so each keeps a colour that reads clearly on it, changed here only)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayerColors {
    pub pads: Rgb,
    pub back_pads: Rgb,
    /// Vias in the package (thermal vias)
    pub vias: Rgb,
    pub courtyard: Rgb,
    pub fab: Rgb,
    pub silk: Rgb,
    pub copper: Rgb,
    pub mask: Rgb,
    pub paste: Rgb,
    pub edge: Rgb,
}

impl Default for LayerColors {
    fn default() -> Self {
        LayerColors {
            pads: Rgb([220, 30, 30]),
            back_pads: Rgb([30, 80, 230]),
            vias: Rgb([0, 150, 150]),
            courtyard: Rgb([230, 0, 230]),
            fab: Rgb([170, 140, 0]),
            silk: Rgb([60, 60, 60]),
            copper: Rgb([210, 30, 30]),
            mask: Rgb([230, 120, 180]),
            paste: Rgb([130, 130, 130]),
            edge: Rgb([200, 180, 0]),
        }
    }
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
            blend: BlendMode::Normal,
            colors: LayerColors::default(),
            ruler_zoom: 8.0,
            invert: false,
            keys: Keys::default(),
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
        // Blend mode and colours by name and #rrggbb
        let s = Settings::parse(
            "[footprint_review]\nblend = \"difference\"\n[footprint_review.colors]\npads = \"#FFFFFF\"\n",
        )
        .unwrap();
        assert_eq!(s.footprint_review.blend, BlendMode::Difference);
        assert_eq!(s.footprint_review.colors.pads, Rgb([255, 255, 255]));
        assert_eq!(s.footprint_review.colors.fab, LayerColors::default().fab);
        assert!(
            s.to_toml().contains("pads = \"#ffffff\""),
            "{}",
            s.to_toml()
        );
        assert!(Settings::parse("[footprint_review.colors]\npads = \"red\"\n").is_err());
        // Keys by name, the rest their defaults
        let s = Settings::parse("[footprint_review.keys]\nrotate = \"Space\"\n").unwrap();
        assert_eq!(s.footprint_review.keys.rotate, "Space");
        assert_eq!(s.footprint_review.keys.ruler, "R");
    }
}
