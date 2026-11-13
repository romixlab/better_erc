//! The app's settings: what a person might want different, each a field with its default and a doc line, in
//! `$BERC_CONFIG`, else `$XDG_CONFIG_HOME/better_erc/config.toml`, else `~/.config/better_erc/config.toml`. A missing
//! file or field takes the default; the app shows and saves them (Footprint review, Settings).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
    /// A command that names a document of the library for the review notes: the program and its arguments; the
    /// datasheet's file is added at the end. It prints a JSON list of `{"path", "value", "doc_no", "doc_rev"}`
    /// (`tpm lib open --json`); the note names the datasheet by its number and revision, else by `value`. Empty:
    /// the note gives the datasheet's file.
    pub library_name: Vec<String>,
    /// A command that lists the document library's index: the program and its arguments. It prints a JSON list of
    /// `{"path", "hash", "doc", "rev"}` (`tpm lib docs --json --all`, paths relative to the library's root, the
    /// content hash in hex). Accept records the datasheet by its hash and document number with it, and a reviewed
    /// symbol's datasheet is found again through it and `library_name`. Empty: no datasheet recorded but the page,
    /// scale and placement.
    pub library_docs: Vec<String>,
    /// A command that names the library document of each part number: the program and its arguments; the
    /// symbol's MPNs are added at the end. It prints a JSON list of `{"mpn", "path", "hash", "doc", "rev", "why"}`
    /// (`tpm lib mpn --json`): one object per document of an MPN, best first, or one with no `path` and `why` it
    /// has none. The Datasheet section's table of MPNs comes from it. Empty: no table.
    pub library_mpn: Vec<String>,
    /// A command that fetches an MPN's datasheet into the library: the program and its arguments; the MPN is added
    /// at the end (`tpm lib fetch --mpn`). Empty: no Fetch button for an MPN without a document.
    pub library_fetch: Vec<String>,
    /// Who reviews, written into the review record (`by=`); empty: the login name
    pub reviewer: String,
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
    /// The shaded 3D model over the page: a turn by the SpaceMouse or Ctrl+drag finishes onto the nearest side
    /// when it lets go, over this time (VIEW-27); Alt held keeps the free angle
    pub model_snap: ve_view3d::SnapSettings,
    /// How X-ray draws the model's edges behind its faces, in the 3D view and over the page: thin or dashed
    /// (VIEW-30)
    pub hidden_edges: ve_view3d::HiddenEdges,
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
    /// Swaps the canvas between the page and the footprint's 3D model (VIEW-23)
    pub view_3d: String,
    /// The Measure model mode: on the 3D model a click picks a face or an edge, a second click another; what lies
    /// between them goes into the review note; again: back to Move
    pub measure_model: String,
    /// Silhouettes: picks the next of the 3D model's views (top, bottom, front, back, left, right) and moves it,
    /// shown (VIEW-24); 3D: turns the shaded model to the next side (VIEW-27)
    pub model_view: String,
    /// How the model is shown over the page: Silhouettes (its six flat views) or 3D (the shaded model) (VIEW-27)
    pub model_look: String,
    /// What the mouse in Move, the arrow keys, turn and mirror move: the footprint or the model (VIEW-27)
    pub moves: String,
    /// Hides the footprint over the page, and shows it again where it was (VIEW-28)
    pub hide_footprint: String,
    /// Hides the model over the page (its silhouettes or the 3D model), and shows it again where it was (VIEW-28)
    pub hide_model: String,
    /// Held: the page alone, the footprint and the model hidden until it is let go (VIEW-28)
    pub peek: String,
    /// The Measure across mode: each ruler end snaps to the footprint or to the drawing, whichever is nearer
    /// (VIEW-29); again: back to Move
    pub measure_across: String,
    /// X-ray: the 3D model see-through, its edges behind the faces drawn too, in the 3D view and over the page
    /// (VIEW-30)
    pub xray: String,
    /// Reads everything shown again from disk (the symbols, footprints, board, models and datasheet), the view kept
    /// (VIEW-31)
    pub reload: String,
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
            view_3d: "3".into(),
            measure_model: "O".into(),
            model_view: "K".into(),
            model_look: "L".into(),
            moves: "Q".into(),
            hide_footprint: "H".into(),
            hide_model: "J".into(),
            peek: "Space".into(),
            measure_across: "Y".into(),
            xray: "U".into(),
            reload: "F5".into(),
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
    /// The 3D model's views over the page: the fill, and the outline and edges
    pub model: Rgb,
    pub model_edges: Rgb,
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
            model: Rgb([40, 140, 255]),
            model_edges: Rgb([0, 60, 170]),
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
            library_docs: ["tpm", "lib", "docs", "--json", "--all"]
                .map(String::from)
                .to_vec(),
            library_mpn: ["tpm", "lib", "mpn", "--json"].map(String::from).to_vec(),
            library_fetch: ["tpm", "lib", "fetch", "--mpn"].map(String::from).to_vec(),
            reviewer: String::new(),
            notes_file: "footprint-review-notes.md".into(),
            blend: BlendMode::Normal,
            colors: LayerColors::default(),
            ruler_zoom: 8.0,
            ruler_snap: true,
            line_snap_px: 10.0,
            model_snap: ve_view3d::SnapSettings::default(),
            hidden_edges: ve_view3d::HiddenEdges::default(),
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

/// Moves the SpaceMouse's mapping this file kept up to 0.4.42 (`[spacemouse]`) into the `spacemouse.toml` every app
/// on ve_spacemouse shares, once: keys that file lacks come from the section, keys it has keep their values; then
/// the section leaves this file. Nothing happens without a section; a file that does not read, or a failed write,
/// leaves this file as it is. Returns what happened, for the settings' note
pub fn move_spacemouse(file: &Path, config: &mut ve_spacemouse::Config) -> Option<String> {
    let text = std::fs::read_to_string(file).ok()?;
    let table: toml::Table = text.parse().ok()?;
    let old = table.get("spacemouse")?.as_table()?;
    let settings = match Settings::parse(&text) {
        Ok(s) => s,
        Err(e) => {
            return Some(format!(
                "the SpaceMouse's mapping stays in {}: {e}",
                file.display()
            ));
        }
    };
    let from = format!("{} [spacemouse]", file.display());
    let note = match config.adopt(old, &from) {
        Ok(ve_spacemouse::Adopted::AlreadyThere) => {
            format!("the SpaceMouse's mapping in {from} was in spacemouse.toml already")
        }
        Ok(ve_spacemouse::Adopted::Moved { path, kept, .. }) if kept.is_empty() => format!(
            "moved the SpaceMouse's mapping from {from} to {}",
            path.display()
        ),
        Ok(ve_spacemouse::Adopted::Moved { path, kept, .. }) => format!(
            "moved the SpaceMouse's mapping from {from} to {}, which kept its own {}",
            path.display(),
            kept.join(", ")
        ),
        Err(e) => return Some(format!("the SpaceMouse's mapping stays in {from}: {e}")),
    };
    Some(match settings.save_to(file) {
        Ok(()) => note,
        Err(e) => format!("{note}; the old section stays: {e}"),
    })
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
        // A file from before spacemouse.toml still reads; its [spacemouse] is not this file's any more
        let s =
            Settings::parse("[spacemouse]\nfull_scale = 300.0\n[spacemouse.speed]\npan = 3000.0\n")
                .unwrap();
        assert!(!s.to_toml().contains("spacemouse"));
    }

    /// The `[spacemouse]` a calibration on the device saved (omarchy-m1, 13 Nov 2026: z and y swapped, y reversed,
    /// full scale 944), with the review's settings beside it
    const OLD: &str = "[footprint_review]\nblink_hz = 3.0\nreviewer = \"me\"\n\n[spacemouse]\nconvention = \"Content\"\n\
        full_scale = 944.0\ndead_zone = 0.08\n\n[[spacemouse.axes]]\nraw = 0\ninvert = false\n\n[[spacemouse.axes]]\n\
        raw = 2\ninvert = false\n\n[[spacemouse.axes]]\nraw = 1\ninvert = true\n\n[[spacemouse.axes]]\nraw = 3\n\
        invert = false\n\n[[spacemouse.axes]]\nraw = 4\ninvert = false\n\n[[spacemouse.axes]]\nraw = 5\n\
        invert = false\n\n[spacemouse.speed]\npan = 1200.0\nzoom = 1.5\nrotate = 2.0\n\n[spacemouse.buttons]\n\
        left = 0\nright = 1\n";

    /// The old section's mapping as `Mapping`, read straight
    fn old_mapping(text: &str) -> ve_spacemouse::Mapping {
        let t: toml::Table = text.parse().unwrap();
        t["spacemouse"].clone().try_into().unwrap()
    }

    /// Moves `text` (a copy) in a folder of its own, the user's spacemouse.toml there too; the settings file,
    /// the spacemouse.toml and the note after
    fn moved(
        text: &str,
        existing: Option<&str>,
    ) -> (String, ve_spacemouse::Config, Option<String>) {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("better_erc/config.toml");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, text).unwrap();
        let user = dir.path().join("spacemouse.toml");
        if let Some(e) = existing {
            std::fs::write(&user, e).unwrap();
        }
        let cwd = dir.path().join("work");
        std::fs::create_dir_all(&cwd).unwrap();
        let mut config = ve_spacemouse::Config::load_from(&cwd, Some(user.clone()));
        let note = move_spacemouse(&file, &mut config);
        let after = std::fs::read_to_string(&file).unwrap();
        // What a fresh start reads
        let config = ve_spacemouse::Config::load_from(&cwd, Some(user));
        (after, config, note)
    }

    #[test]
    fn the_old_section_moves_once_keeping_the_calibration() {
        let (after, config, note) = moved(OLD, None);
        assert_eq!(config.mapping(), &old_mapping(OLD));
        assert_eq!(config.mapping().full_scale, 944.0);
        assert!(config.problems().is_empty(), "{:?}", config.problems());
        assert!(!after.contains("spacemouse"), "{after}");
        let s = Settings::parse(&after).unwrap();
        assert_eq!(s.footprint_review.blink_hz, 3.0);
        assert_eq!(s.footprint_review.reviewer, "me");
        assert!(note.unwrap().starts_with("moved the SpaceMouse's mapping"));
        // Moved already: nothing to do
        let (again, _, note) = moved(&after, None);
        assert_eq!(again, after);
        assert_eq!(note, None);
    }

    #[test]
    fn a_spacemouse_toml_there_already_keeps_its_keys() {
        let (after, config, note) = moved(OLD, Some("full_scale = 500.0\n[speed]\npan = 3000.0\n"));
        assert_eq!(config.mapping().full_scale, 500.0);
        assert_eq!(config.mapping().speed.pan, 3000.0);
        assert_eq!(config.mapping().axes, old_mapping(OLD).axes);
        assert!(!after.contains("spacemouse"));
        let note = note.unwrap();
        assert!(
            note.contains("kept its own full_scale, speed.pan"),
            "{note}"
        );
    }

    #[test]
    fn a_file_that_does_not_read_is_left_alone() {
        let bad = OLD.replace("blink_hz = 3.0", "blink_hz = \"fast\"");
        let (after, config, note) = moved(&bad, None);
        assert_eq!(after, bad);
        assert_eq!(config.mapping(), &ve_spacemouse::Mapping::default());
        assert!(
            note.unwrap()
                .starts_with("the SpaceMouse's mapping stays in")
        );
    }

    /// The move on a copy of a real settings file, when `BERC_SPACEMOUSE_COPY` names one: the copy only, in a
    /// temporary folder, never the file itself
    #[test]
    fn a_real_settings_file_moves_on_a_copy() {
        let Some(real) = std::env::var_os("BERC_SPACEMOUSE_COPY") else {
            eprintln!("BERC_SPACEMOUSE_COPY not set: no real settings file to try the move on");
            return;
        };
        let text = std::fs::read_to_string(&real).unwrap();
        let before = Settings::parse(&text).unwrap();
        let (after, config, note) = moved(&text, None);
        eprintln!("{}", note.unwrap_or_default());
        assert_eq!(config.mapping(), &old_mapping(&text));
        assert!(!after.contains("spacemouse"));
        assert_eq!(Settings::parse(&after).unwrap(), before);
    }
}
