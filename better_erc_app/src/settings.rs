//! The app's settings: what a person might want different, each a field with its default and a doc line, in
//! `$BERC_CONFIG`, else `$XDG_CONFIG_HOME/better_erc/config.toml`, else `~/.config/better_erc/config.toml`. A missing
//! file or field takes the default; the app shows and saves them (Footprint review, Settings).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    pub footprint_review: FootprintReviewSettings,
    /// The SpaceMouse (through spacenavd): which raw axis moves and zooms, the speed, which button is which;
    /// measured by the calibrate step (Footprint review, SpaceMouse)
    pub spacemouse: ve_spacemouse::Mapping,
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
    /// A command that names a document of the library for the review notes: the program and its arguments; the
    /// datasheet's file is added at the end. It prints a JSON list of `{"path", "value", "doc_no", "doc_rev"}`
    /// (`tpm lib open --json`); the note names the datasheet by its number and revision, else by `value`. Empty:
    /// the note gives the datasheet's file.
    pub library_name: Vec<String>,
    /// The review notes file (Markdown): a path of its own, or a name for a file beside the project or library
    /// under review
    pub notes_file: String,
    /// How the footprint is mixed with the page at first: `normal`, `multiply`, `screen` or `difference`
    pub blend: BlendMode,
    /// The footprint's colours per layer group, `#rrggbb`: no picker in the tab, change them here
    pub colors: LayerColors,
    /// How much closer the ruler key zooms in on a ruler end than the view it starts from
    pub ruler_zoom: f32,
    /// The ruler snaps level, horizontal or vertical (an end moved, the other follows on that axis); Shift held
    /// frees it for any angle
    pub ruler_snap: bool,
    /// How near (screen pixels) a ruler end snaps to the drawing's lines: a line's end, two lines crossing, else
    /// onto the line; 0: never. Alt held lets it go free of them.
    pub line_snap_px: f32,
    /// Whether the page is shown inverted (white-ish on black) at first
    pub invert: bool,
    /// How closely the DXF follows the drawing's curves, mm: a curve within it of a circle is an arc, any other is
    /// made of straight pieces within it
    pub dxf_tolerance_mm: f64,
    /// The canvas' keys, by egui's key names (`F`, `Space`, `F5`, ...)
    pub keys: Keys,
    /// The library nicknames a symbol and its footprint may be in, repeatable: an exact name, or a prefix with a
    /// trailing `*` (`VHRD_*`). Any other library named is reported in the symbol list. Empty: no rule
    pub allowed_libraries: Vec<String>,
}

/// The footprint review canvas' keys (while the pointer is over it), by egui's key names; arrows nudge the
/// footprint, Page Up and Page Down turn pages, Escape drops the ruler step
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keys {
    /// The page at the view's full width
    pub full_width: String,
    /// Back to Move: the mouse moves the footprint again (Esc too)
    pub move_tool: String,
    /// The ruler: one end at the pointer, then the other; from Move it starts the Scale mode
    pub ruler: String,
    /// The Measure PDF mode: the ruler snaps to the drawing's lines, its length goes into the review note; again:
    /// back to Move
    pub measure_pdf: String,
    /// The Measure footprint mode: the ruler snaps to the pads' centres and side middles, its length goes into the
    /// review note; again: back to Move
    pub measure_footprint: String,
    /// Zoom in on one ruler end to place it exactly, then on the other, then back
    pub ruler_zoom: String,
    /// The ruler's length typed in: the length field takes the keyboard, Enter sets the scale
    pub scale: String,
    /// The number the dimension prints between the ruler's ends sets the scale
    pub dimension: String,
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
    /// Switches what the SpaceMouse's puck moves: the page or the footprint
    pub puck: String,
    /// Accept: asks whether to write the review hash, Enter writes
    pub accept: String,
    /// The next symbol in the list that needs a review
    pub next: String,
    /// The review note field takes the keyboard; Enter there adds the note to the notes file
    pub note: String,
    /// The Select mode: a click picks a line, arc or shape of the drawing, a drag those in its box; again: back to
    /// Move
    pub select: String,
    /// Saves the pieces selected as a DXF in the footprint's mm beside the notes file, named in the note
    pub save_dxf: String,
}

impl Default for Keys {
    fn default() -> Self {
        Keys {
            full_width: "F".into(),
            move_tool: "M".into(),
            ruler: "R".into(),
            measure_pdf: "E".into(),
            measure_footprint: "G".into(),
            ruler_zoom: "Z".into(),
            scale: "S".into(),
            dimension: "D".into(),
            rotate: "T".into(),
            mirror: "X".into(),
            blink: "B".into(),
            cu_layer: "C".into(),
            invert: "I".into(),
            puck: "P".into(),
            accept: "A".into(),
            next: "N".into(),
            note: "Enter".into(),
            select: "V".into(),
            save_dxf: "W".into(),
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

/// The footprint's colour per layer group, set here only (no picker in the tab). Copper, mask and paste are the
/// views shown one at a time, each everything of its layer filled in its one colour, the back side's too; a datasheet
/// page is white, so each keeps a colour that reads clearly on it (white copper suits the Difference blend)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayerColors {
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
            library_name: ["tpm", "lib", "open", "--json", "-n", "10"]
                .map(String::from)
                .to_vec(),
            notes_file: "footprint-review-notes.md".into(),
            blend: BlendMode::Normal,
            colors: LayerColors::default(),
            ruler_zoom: 8.0,
            ruler_snap: true,
            line_snap_px: 10.0,
            invert: false,
            dxf_tolerance_mm: 0.005,
            keys: Keys::default(),
            allowed_libraries: Vec::new(),
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
        match Settings::file() {
            Some(file) => Settings::load_from(&file),
            None => (Settings::default(), None),
        }
    }

    /// The settings from `file`, as [`Settings::load`] reads them
    pub fn load_from(file: &Path) -> (Settings, Option<String>) {
        match std::fs::read_to_string(file) {
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
        let mut s: Settings = toml::from_str(text).map_err(|e| e.to_string())?;
        // Up to 0.4.36 M was the bottom view's key, and a saved file still says so; M is Move's now
        let k = &mut s.footprint_review.keys;
        if k.mirror.trim() == k.move_tool.trim() && k.mirror.trim() == "M" {
            k.mirror = Keys::default().mirror;
        }
        Ok(s)
    }

    pub fn to_toml(&self) -> String {
        toml::to_string(self).expect("settings serialise")
    }

    /// Writes the settings file (a temporary file, then renamed over it)
    pub fn save(&self) -> Result<PathBuf, String> {
        let file =
            Settings::file().ok_or("neither BERC_CONFIG, XDG_CONFIG_HOME nor HOME is set")?;
        self.save_to(&file)?;
        Ok(file)
    }

    /// Writes `file` as [`Settings::save`] does
    pub fn save_to(&self, file: &Path) -> Result<(), String> {
        if let Some(d) = file.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("can't make {}: {e}", d.display()))?;
        }
        let tmp = file.with_extension("toml.tmp");
        std::fs::write(&tmp, self.to_toml())
            .and_then(|_| std::fs::rename(&tmp, file))
            .map_err(|e| format!("can't write {}: {e}", file.display()))
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
            "[footprint_review]\nblend = \"difference\"\n[footprint_review.colors]\ncopper = \"#FFFFFF\"\n",
        )
        .unwrap();
        assert_eq!(s.footprint_review.blend, BlendMode::Difference);
        assert_eq!(s.footprint_review.colors.copper, Rgb([255, 255, 255]));
        assert_eq!(s.footprint_review.colors.fab, LayerColors::default().fab);
        assert!(
            s.to_toml().contains("copper = \"#ffffff\""),
            "{}",
            s.to_toml()
        );
        assert!(Settings::parse("[footprint_review.colors]\ncopper = \"red\"\n").is_err());
        // Keys by name, the rest their defaults
        let s = Settings::parse("[footprint_review.keys]\nrotate = \"Space\"\n").unwrap();
        assert_eq!(s.footprint_review.keys.rotate, "Space");
        assert_eq!(s.footprint_review.keys.ruler, "R");
        // A file saved up to 0.4.36 has M for the bottom view: M is Move's now, the bottom view takes its new key;
        // a key of the person's own choosing stays
        let s = Settings::parse("[footprint_review.keys]\nmirror = \"M\"\n").unwrap();
        assert_eq!(s.footprint_review.keys.move_tool, "M");
        assert_eq!(s.footprint_review.keys.mirror, "X");
        let s = Settings::parse("[footprint_review.keys]\nmirror = \"V\"\n").unwrap();
        assert_eq!(s.footprint_review.keys.mirror, "V");
        // The SpaceMouse's mapping, the rest its defaults, through the file as it is
        let s =
            Settings::parse("[spacemouse]\nfull_scale = 300.0\n[spacemouse.speed]\npan = 3000.0\n")
                .unwrap();
        assert_eq!(s.spacemouse.full_scale, 300.0);
        assert_eq!(s.spacemouse.speed.pan, 3000.0);
        assert_eq!(
            s.spacemouse.buttons,
            ve_spacemouse::Mapping::default().buttons
        );
        assert_eq!(Settings::parse(&s.to_toml()).unwrap(), s);
    }
}
