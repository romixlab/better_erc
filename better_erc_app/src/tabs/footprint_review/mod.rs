//! Footprint review (VIEW-19): a library symbol's footprint drawn over its datasheet drawing. The drawing's scale
//! is set with a ruler along one of its dimensions; the footprint is moved, turned and mirrored onto the drawing,
//! shown half see-through or blinking; when it matches, Accept writes its review hash (`fp1:`, [`better_erc::fp`])
//! into the symbol's `Footprint_Review` field.
//!
//! The review starts from a library symbol, since the symbol is what a schematic places and what holds the hash:
//! picked from a symbol library (a `.kicad_sym`, a `.kicad_symdir`, a nickname in the `sym-lib-table`s), or from
//! a board's list of the library symbols it uses ([`better_erc::fp::todo`]), stepping through those to review.
//! The footprint is the library footprint the symbol's `Footprint` field names, the datasheet the file its
//! `Datasheet` field names, else a local library search (a setting), else a file picker; the page search finds the
//! land pattern page by words that are a setting too. A footprint opened alone (a `.kicad_mod`, a board's copy) is
//! a preview: there is no symbol to write the hash into, so Accept is off and says so.
//!
//! The canvas has a key for each step (settings, `[footprint_review.keys]`; a help line under the canvas names
//! them): the page at full width; the ruler placed coarsely at the pointer, then each end zoomed in on to place it
//! exactly, and back; once the scale is set, the footprint is the one to move again.

use crate::prelude::*;
use crate::settings::{BlendMode, FootprintReviewSettings, Keys, LayerColors, Rgb, Settings};
use crate::tabs::TabUi;
use better_erc::fp::{
    AcceptRun, AcceptWhat, FIELD, FootprintSource, HashSource, LibCheckOptions, LibrarySearch,
    Sourced, SymbolLibraries, SymbolStatus, TodoEntry,
};
use better_erc::model::layout::{FootprintId, Layout};
use egui::{Align2, FontId, Pos2, Rect, Shape, Stroke, pos2, vec2};
use geometry::{DimNumber, Snap, SnapKind};
use outline::{Item, Kind};
use pdf::{PageHit, PdfDoc};
pub use puck::PuckMoves;
use puck::{Action, Puck};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Receiver;

pub mod blend;
mod dxf;
mod geometry;
#[cfg(test)]
mod mode_tests;
mod notes;
mod outline;
mod pdf;
mod prims;
mod puck;
#[cfg(test)]
mod testpdf;
#[cfg(test)]
mod tests;

/// Around the page
const CANVAS_BACKGROUND: Color32 = Color32::from_gray(60);

/// The drawing's pieces selected, and the box that selects them
const SELECT_COLOUR: Color32 = Color32::from_rgb(0, 200, 255);

/// What the ruler snaps to, and the dimension's number found for it
const SNAP_COLOUR: Color32 = Color32::from_rgb(255, 160, 0);

/// The views: what the board makes on copper, mask or paste, one shown at a time (a radio choice, not toggles that
/// stack), each filled in its own colour (a setting, no picker in the tab: the colour says what is shown)
const VIEWS: [Kind; 3] = [Kind::Copper, Kind::Mask, Kind::Paste];

/// The back side's checkbox: the pads' copper, mask openings and paste apertures on the back, in the view picked
const BACK: &str = "Back pads";

/// Millimetres per PDF point at the PDF's own scale (1:1)
const MM_PER_POINT: f64 = 25.4 / 72.0;

/// Records one usage signal (`app fp-review accept`)
pub type Usage = Arc<dyn Fn(&str) + Send + Sync>;

fn record_usage() -> Usage {
    Arc::new(|signal: &str| better_erc::usage::record(&[signal.to_string()], "app"))
}

/// How the footprint sits on the page
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Placement {
    /// Where the footprint's origin is, in page points
    pub origin: [f32; 2],
    /// Degrees, counter-clockwise on screen
    pub rotation: f32,
    /// Seen from below: x mirrored before turning
    pub mirror: bool,
}

impl Default for Placement {
    fn default() -> Self {
        Placement {
            origin: [100.0, 100.0],
            rotation: 0.0,
            mirror: false,
        }
    }
}

/// The drawing's scale, set with the ruler along a dimension
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scale {
    pub mm_per_point: f64,
    /// The page it was set on
    pub page: usize,
    /// The ruler's length it was set from, mm; 0 when it came from the page's own scale note
    pub mm: f64,
    /// The page's scale note it came from (`SCALE: 20X`), when it did
    #[serde(default)]
    pub note: Option<String>,
}

/// A footprint accepted as matching its datasheet
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Review {
    pub lib_id: String,
    pub hash: String,
    pub source: String,
    pub datasheet: Option<PathBuf>,
    /// 1-based, as a viewer shows it
    pub page: usize,
    pub mm_per_point: f64,
    /// Unix seconds
    pub when: u64,
}

/// What the primary drag does on the canvas: the mode, always shown (the switcher in the controls, the chip on the
/// canvas) and always left the same ways (Esc, the Move key, a mode's own key again)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Tool {
    /// Primary drag moves the footprint
    #[default]
    Move,
    /// The ruler on the drawing's lines, to set the scale from a dimension (S types its length, D takes the
    /// printed number); setting it goes back to Move
    #[serde(alias = "Ruler")]
    Scale,
    /// The ruler on the drawing's lines, measuring: its length goes into the review note
    MeasurePdf,
    /// The ruler on the footprint's pads (centres, and the middles of a rectangular pad's sides), measuring: its
    /// length goes into the review note
    MeasureFootprint,
    /// Picks the drawing's lines, arcs, curves and shapes, to save them as a DXF in the footprint's mm (VIEW-22)
    Select,
}

impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Tool::Move => "Move",
            Tool::Scale => "Scale",
            Tool::MeasurePdf => "Measure PDF",
            Tool::MeasureFootprint => "Measure footprint",
            Tool::Select => "Select",
        }
    }

    /// A ruler mode: the primary drag lays the ruler
    pub fn ruler(self) -> bool {
        !matches!(self, Tool::Move | Tool::Select)
    }

    /// A measuring mode: the ruler's length goes into the review note
    pub fn measures(self) -> bool {
        matches!(self, Tool::MeasurePdf | Tool::MeasureFootprint)
    }

    /// What the primary drag does, for the switcher's tooltip and the canvas' chip
    fn what(self) -> &'static str {
        match self {
            Tool::Move => "the mouse moves the footprint; arrow keys nudge it (Shift: more)",
            Tool::Scale => {
                "the ruler on the drawing's lines sets the scale from a dimension: type its length, or take the \
                 number the drawing prints"
            }
            Tool::MeasurePdf => {
                "the ruler on the drawing's lines measures; its length goes into the review note"
            }
            Tool::MeasureFootprint => {
                "the ruler on the pads' centres and side middles measures the footprint; its length goes into the \
                 review note"
            }
            Tool::Select => {
                "a click picks a line, arc or shape of the drawing (again: drops it), a drag the ones inside its box \
                 (Shift adds); Save writes them as a DXF in the footprint's mm and names it in the note"
            }
        }
    }
}

/// Where the list of symbols to review comes from
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SymbolsFrom {
    /// The library symbols a KiCad project's board uses (its directory, `.kicad_pro`, `.kicad_sch` or
    /// `.kicad_pcb`), as `berc fp todo` lists them
    Project(PathBuf),
    /// A symbol library: a `.kicad_sym`, a `.kicad_symdir`, or a nickname in the `sym-lib-table`s of `project`
    /// and the global one; footprints found through `project`'s and the global `fp-lib-table`
    Library { library: String, project: PathBuf },
}

impl SymbolsFrom {
    /// The directory whose library tables are read
    fn dir(&self) -> PathBuf {
        match self {
            SymbolsFrom::Project(p) if p.is_dir() => p.clone(),
            SymbolsFrom::Project(p) => p.parent().unwrap_or(Path::new(".")).to_path_buf(),
            SymbolsFrom::Library { project, .. } => project.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct FootprintReview {
    /// The symbol list open
    symbols_from: Option<SymbolsFrom>,
    /// The library symbol under review (`Lib:Name`)
    symbol: Option<String>,
    /// A footprint opened alone, a preview (no symbol under review)
    source: Option<FootprintSource>,
    /// A board opened to preview its footprints from
    board: Option<PathBuf>,
    datasheet: Option<PathBuf>,
    /// 0-based
    page: usize,
    /// Screen points per page point
    zoom: f32,
    /// Where the page's top left corner is, from the canvas' top left, screen points
    pan: [f32; 2],
    place: Placement,
    /// The ruler's two ends, page points
    ruler: Option<[[f32; 2]; 2]>,
    scale: Option<Scale>,
    opacity: Option<f32>,
    /// The blend mode chosen here; none: the settings' default
    blend: Option<BlendMode>,
    blink: bool,
    /// The page inverted (white-ish on black); none: the settings' default
    invert: Option<bool>,
    hidden: Vec<String>,
    /// Which of copper, mask or paste is shown (the three views shown one at a time, see [`VIEWS`])
    cu_shown: Kind,
    tool: Tool,
    /// What the SpaceMouse's puck moves
    puck: PuckMoves,
    reviews: Vec<Review>,
    #[serde(skip)]
    rt: Runtime,
}

impl Default for FootprintReview {
    fn default() -> Self {
        FootprintReview {
            symbols_from: None,
            symbol: None,
            source: None,
            board: None,
            datasheet: None,
            page: 0,
            zoom: 1.0,
            pan: [0.0, 0.0],
            place: Placement::default(),
            ruler: None,
            scale: None,
            opacity: None,
            blend: None,
            blink: false,
            invert: None,
            hidden: Kind::OWN
                .iter()
                .filter(|k| !k.shown_at_first())
                .map(|k| k.name().to_string())
                .collect(),
            cu_shown: Kind::Copper,
            tool: Tool::Move,
            puck: PuckMoves::Page,
            reviews: Vec::new(),
            rt: Runtime::default(),
        }
    }
}

/// The library symbols to review, one row each
struct SymbolList {
    from: SymbolsFrom,
    /// A line on where they come from
    title: String,
    rows: Vec<TodoEntry>,
    filter: String,
}

/// A measurement put into the review note: `M{n} = {mm} mm`, from the ruler with these ends
#[derive(Clone, Debug, PartialEq)]
struct Measured {
    n: usize,
    ends: [[f32; 2]; 2],
    /// As written into the note
    text: String,
}

/// Accept's question before it writes: what its dry run would write where, and whether KiCad may overwrite that
struct Confirm {
    review: Review,
    target: Target,
    run: AcceptRun,
    /// Why KiCad may overwrite the change on its next save, one line each
    warnings: Vec<String>,
}

/// Why KiCad may overwrite what `run` writes: a file it would change is open in KiCad, by its lock file (see
/// [`kicad_lock`]), one line each naming the lock file. A KiCad running without a lock beside the file is no warning.
fn kicad_warnings(run: &AcceptRun, running: bool) -> Vec<String> {
    let host = this_host();
    run.symbols
        .iter()
        .flat_map(|a| &a.files)
        .filter(|w| w.changed)
        .filter_map(|w| {
            let lock = kicad_lock(&w.file, host.as_deref(), running)?;
            Some(format!(
                "{} is open in KiCad (its lock file {}{}): KiCad's next save of it overwrites the change; close it \
                 there first",
                file_name(&w.file),
                file_name(&lock.path),
                lock.owner.map(|o| format!(", {o}")).unwrap_or_default(),
            ))
        })
        .collect()
}

/// KiCad's lock file of an open file, held
struct KicadLock {
    path: PathBuf,
    /// Who holds it, `user@host`, when the lock says
    owner: Option<String>,
}

/// The lock file KiCad keeps beside `file` while it has it open (`~NAME.EXT.lck`, `LOCKFILE::LockPathFor` in
/// KiCad's `include/lockfile.h`), when it is held. KiCad 8 to 10 write `{"username", "hostname"}` into it and no
/// process id: a lock from this computer (`host`) is held while a KiCad runs here (`running`), or while its `pid`
/// lives should a lock carry one; a lock from another computer, or one that names none, can't be checked from here
/// and counts as held. An empty or unreadable lock is stale, as KiCad 10 takes it.
fn kicad_lock(file: &Path, host: Option<&str>, running: bool) -> Option<KicadLock> {
    let path = file.with_file_name(format!("~{}.lck", file_name(file)));
    let text = std::fs::read_to_string(&path).ok()?;
    let rec: serde_json::Value = serde_json::from_str(&text).ok()?;
    rec.as_object()?;
    let field = |k: &str| {
        rec.get(k)
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
    };
    let short = |h: &str| h.split('.').next().unwrap_or(h).to_lowercase();
    let here = field("hostname")
        .zip(host)
        .is_some_and(|(h, me)| short(h) == short(me));
    if here {
        let held = match rec.get("pid").and_then(|p| p.as_u64()) {
            Some(pid) => process_alive(pid),
            None => running,
        };
        if !held {
            return None;
        }
    }
    let owner = match (field("username"), field("hostname")) {
        (Some(u), Some(h)) => Some(format!("{u}@{h}")),
        (u, h) => u.or(h).map(String::from),
    };
    Some(KicadLock { path, owner })
}

/// This computer's name, as KiCad writes it into its locks (wxGetHostName: without the domain)
fn this_host() -> Option<String> {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
}

/// Whether process `pid` lives on this computer; where that can't be told (no `/proc`), it is taken to
fn process_alive(pid: u64) -> bool {
    if !Path::new("/proc/self").exists() {
        return true;
    }
    Path::new("/proc").join(pid.to_string()).exists()
}

/// Whether a KiCad process (the project manager or the schematic editor) runs on this computer
fn kicad_running() -> bool {
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return false;
    };
    dir.flatten().any(|e| {
        std::fs::read_to_string(e.path().join("comm"))
            .is_ok_and(|c| matches!(c.trim(), "kicad" | "eeschema"))
    })
}

/// The library symbol under review: Accept writes the review hash into it
#[derive(Clone, Debug)]
struct Target {
    symbol: String,
    from: SymbolsFrom,
}

/// A board's footprints, one row per library footprint
struct BoardList {
    path: PathBuf,
    layout: Layout,
    /// Library id, the references using it (natural order), the first one's index
    groups: Vec<(String, Vec<String>, usize)>,
    filter: String,
}

/// Names a document of the library for the review notes ([`notes::run_library_name`]); tests give their own
type Namer = Arc<dyn Fn(&Path) -> Result<Option<String>, String> + Send + Sync>;

/// A library search: the datasheets it finds for the search words
type Searcher = Arc<dyn Fn(&[String]) -> Result<Vec<LibraryHit>, String> + Send + Sync>;

/// The library search's hits and the query that found them, or why it failed
type SearchReply = Result<(Vec<LibraryHit>, String), String>;

/// A datasheet the library search found
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct LibraryHit {
    pub path: PathBuf,
    #[serde(default)]
    pub page: Option<usize>,
}

struct Runtime {
    started: bool,
    settings: FootprintReviewSettings,
    settings_note: Option<String>,
    usage: Usage,
    sourced: Option<Sourced>,
    items: Vec<Item>,
    /// For a footprint picked from a board: how its board copy compares with what is reviewed
    board_note: Option<String>,
    error: Option<String>,
    board: Option<BoardList>,
    pdf: Option<PdfDoc>,
    page_hits: Option<Vec<PageHit>>,
    /// The page search may still pick the page (a datasheet just opened)
    auto_page: bool,
    /// The footprint goes to the middle of the view on the next frame
    center_footprint: bool,
    /// The page is fitted into the view on the next frame
    fit_page: bool,
    query: String,
    /// What the library search tries for a footprint just opened, best first
    queries: Vec<String>,
    /// The library search running: its hits and the query that found them
    searching: Option<Receiver<SearchReply>>,
    search_result: Option<Result<Vec<LibraryHit>, String>>,
    ruler_mm: String,
    /// Which end of the ruler a drag moves
    ruler_end: Option<usize>,
    /// While a new ruler is laid (the ruler key, a drag from nothing): its first end, which picks the snapped axis
    /// with the end that moves
    ruler_anchor: Option<[f32; 2]>,
    /// The ruler key placed the first end: the other follows the pointer until the key again or a click
    ruler_placing: bool,
    /// The ruler end zoomed in on: a click or a drag places it at the pointer
    ruler_focus: Option<usize>,
    /// Where a ruler end snaps to the drawing's lines this frame, shown at the pointer before it is let go
    snap: Option<Snap>,
    /// The view (zoom, pan) from before zooming in on a ruler end
    overview: Option<(f32, [f32; 2])>,
    /// The ruler's length asked in a box in the middle of the canvas (the scale key), the number typed shown large
    length_box: bool,
    /// The length field has the keyboard
    typing_mm: bool,
    /// The page is fitted to the view's width on the next frame
    fit_width: bool,
    /// The other ruler: the scale's while measuring, the measuring one while in Move or Scale
    parked_ruler: Option<[[f32; 2]; 2]>,
    /// Where the measuring ruler snaps on the footprint: its pads' centres and side middles, footprint mm
    fp_snaps: Vec<outline::SnapPoint>,
    /// The measuring ruler's number and length as it went into the note; it is the ruler shown while its ends are
    /// where they were
    measured: Option<Measured>,
    /// How many measurements the note has had: the next is M(n+1)
    measure_count: usize,
    /// The drawing's pieces selected (indices into the page's [`geometry::PageGeometry::prims`], sorted), on
    /// [`Runtime::selected_page`] of the datasheet open
    selected: Vec<usize>,
    selected_page: usize,
    /// The piece a click would pick, under the pointer in the Select mode
    select_hover: Option<usize>,
    /// Where the Select mode's box drag started, page points, and where the pointer is
    select_box: Option<[f32; 2]>,
    select_box_end: Option<[f32; 2]>,
    /// Where the last DXF went, or why it did not
    dxf_written: Option<Result<PathBuf, String>>,
    /// The review note being written
    note: String,
    /// The note field takes the keyboard on the next frame (the note key)
    focus_note: bool,
    /// Where the last note went, or why it did not
    note_written: Option<Result<PathBuf, String>>,
    /// How the library names the datasheet open: its file, and the name (none: the library has no such file, or
    /// no name command; the note gives the file), or why the name command failed
    doc_name: Option<(PathBuf, Result<Option<String>, String>)>,
    /// The library's name command in place of the settings' one (tests)
    namer: Option<Namer>,
    accepted: Option<String>,
    /// Accept waits for its confirmation
    confirm: Option<Confirm>,
    /// Whether KiCad runs (a fake in tests)
    kicad_running: fn() -> bool,
    /// The symbol list open
    symbols: Option<SymbolList>,
    /// The symbol under review, whose footprint is shown; none: a footprint opened alone, a preview
    target: Option<Target>,
    /// Why the symbol under review has no footprint to show (it sets none, or it was not found)
    symbol_note: Option<String>,
    /// KiCad's configuration directory, for the global library tables ([`better_erc::fp::kicad_config_dir`])
    config: Option<PathBuf>,
    /// The symbol libraries the tables name: the directory they were read for, and the nicknames
    tables: Option<(PathBuf, Vec<String>)>,
    tables_filter: String,
    /// What Accept wrote into the symbols, or why it could not
    written: Option<Result<AcceptRun, String>>,
    /// The settings as text being edited: page search words and font folders one per line, the search command
    words_text: String,
    fonts_text: String,
    search_text: String,
    name_text: String,
    /// Where the canvas was drawn last
    canvas: Rect,
    /// A footprint was just opened: find its datasheet (the Datasheet field's file, else the library search)
    want_search: bool,
    /// The library search running was started for a part just picked: its first hit opens by itself
    auto_open: bool,
    /// The library search in place of the settings' command (tests)
    searcher: Option<Searcher>,
    /// The SpaceMouse: its client, mapping, pie menu and calibrate step
    puck: Puck,
    /// The settings file the Save buttons write; none in tests
    settings_file: Option<PathBuf>,
    /// Each frame ends with the PDF reader done with what it asked (tests: its replies would otherwise come in on
    /// its thread over any number of frames), and the threads wake no frame: the UI asks for the next one itself
    /// when it takes a reply in, so how many frames a step takes never hangs on a thread's timing
    #[cfg(test)]
    sync_pdf: bool,
}

impl Default for Runtime {
    fn default() -> Self {
        Runtime {
            started: false,
            settings: FootprintReviewSettings::default(),
            settings_note: None,
            usage: record_usage(),
            sourced: None,
            items: Vec::new(),
            board_note: None,
            error: None,
            board: None,
            pdf: None,
            page_hits: None,
            auto_page: false,
            center_footprint: false,
            fit_page: false,
            query: String::new(),
            queries: Vec::new(),
            searching: None,
            search_result: None,
            ruler_mm: String::new(),
            ruler_end: None,
            ruler_placing: false,
            ruler_anchor: None,
            ruler_focus: None,
            snap: None,
            overview: None,
            length_box: false,
            typing_mm: false,
            fit_width: false,
            parked_ruler: None,
            fp_snaps: Vec::new(),
            measured: None,
            measure_count: 0,
            selected: Vec::new(),
            selected_page: 0,
            select_hover: None,
            select_box: None,
            select_box_end: None,
            dxf_written: None,
            note: String::new(),
            focus_note: false,
            note_written: None,
            doc_name: None,
            namer: None,
            accepted: None,
            confirm: None,
            kicad_running,
            symbols: None,
            target: None,
            symbol_note: None,
            config: better_erc::fp::kicad_config_dir(),
            tables: None,
            tables_filter: String::new(),
            written: None,
            words_text: String::new(),
            fonts_text: String::new(),
            search_text: String::new(),
            name_text: String::new(),
            canvas: Rect::NOTHING,
            want_search: false,
            auto_open: false,
            searcher: None,
            puck: Puck::default(),
            settings_file: Settings::file(),
            #[cfg(test)]
            sync_pdf: false,
        }
    }
}

impl TabUi for FootprintReview {
    fn init(&mut self, _cx: &Context) {
        if self.rt.started {
            return;
        }
        let (settings, note) = Settings::load();
        self.set_settings(settings.footprint_review);
        self.rt.puck.mapping = settings.spacemouse;
        self.rt.settings_note = note;
    }

    fn ui(&mut self, ui: &mut Ui, _cx: &mut Context, id: Id) {
        self.start(ui.ctx());
        self.poll(ui.ctx());
        let events = self.puck_poll(ui.ctx());
        egui::Panel::left(id.with("controls"))
            .resizable(true)
            .default_size(320.0)
            .max_size(520.0)
            .show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| self.controls(ui));
            });
        self.canvas(ui);
        self.confirm_ui(ui.ctx(), id);
        self.calibration_window(ui.ctx(), id);
        self.puck_pie(ui.ctx(), &events);
        #[cfg(test)]
        if self.rt.sync_pdf
            && let Some(pdf) = &mut self.rt.pdf
        {
            pdf.wait_idle(ui.ctx());
        }
    }
}

impl FootprintReview {
    /// Accept's confirmation, when it waits for one: each file, the field, its old and new value; Enter writes, Esc
    /// or Cancel writes nothing
    fn confirm_ui(&mut self, ctx: &egui::Context, id: Id) {
        let Some(c) = &self.rt.confirm else { return };
        let mut answer = None;
        let modal = egui::Modal::new(id.with("confirm accept")).show(ctx, |ui| {
            ui.set_max_width(640.0);
            ui.heading("Write the review hash?");
            ui.label(format!(
                "Accepting {} writes its review hash into these files:",
                c.review.lib_id
            ));
            for a in &c.run.symbols {
                ui.label(RichText::new(&a.symbol).strong());
                Grid::new(("confirm files", &a.symbol))
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        for w in &a.files {
                            let kind = match w.source {
                                HashSource::Library => "library",
                                HashSource::Schematic => "schematic copy",
                            };
                            ui.label(format!("{} ({kind})", file_name(&w.file)))
                                .on_hover_text(w.file.display().to_string());
                            ui.label(&c.run.field);
                            let change = match (&w.old, w.changed) {
                                (_, false) => format!("{}, already", a.hash),
                                (None, true) => format!("(none) → {}", a.hash),
                                (Some(old), true) => format!("{old} → {}", a.hash),
                            };
                            ui.label(RichText::new(change).monospace());
                            ui.end_row();
                        }
                    });
                if let Some(n) = &a.library_note {
                    ui.colored_label(Color32::ORANGE, format!("Library symbol not written: {n}"));
                }
            }
            for w in &c.warnings {
                ui.colored_label(Color32::from_rgb(230, 140, 0), format!("⚠ {w}"));
            }
            ui.horizontal(|ui| {
                if ui.button("Write (Enter)").clicked() {
                    answer = Some(true);
                }
                if ui.button("Cancel (Esc)").clicked() {
                    answer = Some(false);
                }
            });
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                answer = Some(true);
            }
        });
        if modal.should_close() {
            answer.get_or_insert(false);
        }
        if let Some(write) = answer {
            self.confirmed(ctx, write);
        }
    }

    /// A review tab with these settings, recording usage through `usage` (tests)
    #[cfg(test)]
    pub fn with(settings: FootprintReviewSettings, usage: Usage) -> FootprintReview {
        let mut r = FootprintReview::default();
        r.set_settings(settings);
        r.rt.usage = usage;
        // No global library tables: the test's own only
        r.rt.config = None;
        // Not this computer's KiCad
        r.rt.kicad_running = || false;
        // Not this computer's SpaceMouse or settings file
        r.rt.puck.socket = None;
        r.rt.settings_file = None;
        #[cfg(test)]
        {
            r.rt.sync_pdf = true;
        }
        r
    }

    fn set_settings(&mut self, s: FootprintReviewSettings) {
        self.rt.words_text = s.page_search_words.join("\n");
        self.rt.fonts_text = s
            .font_dirs
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        self.rt.search_text = s.library_search.join(" ");
        self.rt.name_text = s.library_name.join(" ");
        self.rt.settings = s;
    }

    /// Whether the worker threads (the PDF reader, the library search) wake the UI with their replies; not in the
    /// tests ([`Runtime::sync_pdf`])
    fn threads_wake(&self) -> bool {
        #[cfg(test)]
        return !self.rt.sync_pdf;
        #[cfg(not(test))]
        true
    }

    fn usage(&self, signal: &str) {
        (self.rt.usage)(&format!("app fp-review {signal}"));
    }

    /// The first frame: open again what was open
    fn start(&mut self, ctx: &egui::Context) {
        if self.rt.started {
            return;
        }
        self.rt.started = true;
        if let Some(b) = self.board.clone()
            && b.is_file()
        {
            self.open_board(&b);
        }
        if let Some(from) = self.symbols_from.clone() {
            self.open_symbols(from);
        }
        if let Some(sym) = self.symbol.clone() {
            if let Some(i) = self.symbol_row(&sym) {
                self.select_symbol(i, false);
            }
        } else if let Some(s) = self.source.clone() {
            self.load(s, false);
        }
        if let Some(d) = self.datasheet.clone()
            && d.is_file()
        {
            let page = self.page;
            self.open_datasheet(ctx, &d);
            self.page = page;
            self.rt.auto_page = false;
            self.rt.fit_page = false;
        }
    }

    fn poll(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.rt.want_search) {
            // A search still running for the last part has nothing for this one
            self.rt.searching = None;
            match self.datasheet_file() {
                Some(f) if self.datasheet.as_ref() != Some(&f) => self.open_datasheet(ctx, &f),
                Some(_) => {}
                None => {
                    // Not the last part's datasheet: the library search may find this one's
                    self.close_datasheet();
                    self.library_search(ctx, true);
                }
            }
        }
        if let Some(pdf) = &mut self.rt.pdf {
            pdf.poll(ctx);
            if self.rt.page_hits.is_none() && pdf.texts_done() {
                let hits = pdf.find_pages(&self.rt.settings.page_search_words);
                if self.rt.auto_page
                    && let Some(h) = hits.first()
                {
                    self.page = h.page;
                    self.rt.fit_page = true;
                }
                self.rt.auto_page = false;
                self.rt.page_hits = Some(hits);
            }
        }
        if let Some(rx) = &self.rt.searching
            && let Ok(r) = rx.try_recv()
        {
            if !self.threads_wake() {
                ctx.request_repaint();
            }
            self.rt.search_result = Some(r.map(|(hits, query)| {
                if !query.is_empty() {
                    self.rt.query = query;
                }
                hits
            }));
            self.rt.searching = None;
            // The search for a part just picked: its first hit opens by itself
            if std::mem::take(&mut self.rt.auto_open)
                && self.rt.pdf.is_none()
                && let Some(Ok(hits)) = &self.rt.search_result
                && let Some(first) = hits.first().cloned()
            {
                self.open_hit(ctx, &first);
            }
        }
    }

    /// Opens what `path` is: a KiCad project (its directory, `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`) for the
    /// library symbols its board uses, a symbol library (`.kicad_sym`, `.kicad_symdir`) for its symbols, or a
    /// footprint file alone, as a preview
    pub fn open_path(&mut self, ctx: &egui::Context, path: &Path) {
        let _ = ctx;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "kicad_sym" | "kicad_symdir" => {
                self.open_symbols(SymbolsFrom::Library {
                    library: path.display().to_string(),
                    project: path.parent().unwrap_or(Path::new(".")).to_path_buf(),
                });
                self.usage("library");
            }
            "kicad_mod" => {
                self.load(FootprintSource::File(path.to_path_buf()), true);
                self.usage("footprint");
                self.rt.want_search = true;
            }
            _ => {
                self.open_symbols(SymbolsFrom::Project(path.to_path_buf()));
                self.usage("project");
            }
        }
    }

    /// The search for libraries: the project's tables and, outside tests, the global ones; a library opened by
    /// path is searched first
    fn library_search_for(&self, from: &SymbolsFrom) -> LibrarySearch {
        let mut s = LibrarySearch {
            config: self.rt.config.clone(),
            ..LibrarySearch::default()
        };
        if let SymbolsFrom::Library { library, .. } = from
            && Path::new(library).exists()
        {
            s.symbols.push(PathBuf::from(library));
        }
        s
    }

    /// Opens the list of library symbols to review, and for a project the board's footprints to preview
    fn open_symbols(&mut self, from: SymbolsFrom) {
        let search = self.library_search_for(&from);
        let list = match &from {
            SymbolsFrom::Project(p) => better_erc::fp::todo(p, &search)
                .map(|t| {
                    let title = format!(
                        "{} library symbols on {}, {} to review",
                        t.symbols.len(),
                        file_name(&t.board),
                        t.to_review().count()
                    );
                    (title, t.symbols, Some(t.board))
                })
                .map_err(|e| e.to_string()),
            SymbolsFrom::Library { library, project } => {
                better_erc::fp::check_libraries(&LibCheckOptions {
                    libraries: vec![library.clone()],
                    project: project.clone(),
                    search,
                })
                .map(|c| {
                    let rows: Vec<TodoEntry> = c
                        .symbols
                        .into_iter()
                        .map(|e| TodoEntry {
                            symbol: e.symbol,
                            status: e.status,
                            footprint: e.footprint,
                            footprint_file: e.footprint_file,
                            hash: e.hash,
                            stored: e.stored,
                            datasheet: e.datasheet,
                            references: Vec::new(),
                            note: e.note,
                        })
                        .collect();
                    let names: Vec<String> =
                        c.libraries.iter().map(|l| l.nickname.clone()).collect();
                    let title = format!(
                        "{} symbols in {}, {} to review",
                        rows.len(),
                        names.join(", "),
                        rows.iter().filter(|r| r.needs_review()).count()
                    );
                    (title, rows, None)
                })
                .map_err(|e| e.to_string())
            }
        };
        match list {
            Ok((title, rows, board)) => {
                self.symbols_from = Some(from.clone());
                self.rt.symbols = Some(SymbolList {
                    from,
                    title,
                    rows,
                    filter: String::new(),
                });
                self.rt.error = None;
                if let Some(b) = board {
                    self.open_board(&b);
                }
            }
            Err(e) => self.rt.error = Some(e),
        }
    }

    fn symbol_row(&self, symbol: &str) -> Option<usize> {
        self.rt
            .symbols
            .as_ref()?
            .rows
            .iter()
            .position(|r| r.symbol == symbol)
    }

    /// The row after the current one that needs a review, from the top again at the end
    fn next_to_review(&self) -> Option<usize> {
        let rows = &self.rt.symbols.as_ref()?.rows;
        let at = self
            .rt
            .target
            .as_ref()
            .and_then(|t| self.symbol_row(&t.symbol));
        let start = at.map_or(0, |i| i + 1);
        (0..rows.len())
            .map(|k| (start + k) % rows.len())
            .find(|i| Some(*i) != at && rows[*i].needs_review())
    }

    /// Puts row `i` of the symbol list under review: its footprint from its `Footprint` field, its datasheet from
    /// its `Datasheet` field (`fresh`: looked for again), or the reason it has no footprint to review
    fn select_symbol(&mut self, i: usize, fresh: bool) {
        let Some(list) = &self.rt.symbols else { return };
        let row = list.rows[i].clone();
        let from = list.from.clone();
        self.symbol = Some(row.symbol.clone());
        self.source = None;
        self.rt.board_note = None;
        self.rt.target = Some(Target {
            symbol: row.symbol.clone(),
            from: from.clone(),
        });
        self.rt.symbol_note = None;
        self.rt.accepted = None;
        self.rt.written = None;
        let no_footprint = |why: String| (None, Some(why));
        let (sourced, note) = match (&row.footprint, &row.footprint_file) {
            (None, _) if row.status == SymbolStatus::Missing => no_footprint(format!(
                "the library symbol was not found: {}",
                row.note.as_deref().unwrap_or("")
            )),
            (None, _) => no_footprint(format!(
                "{} sets no footprint (its Footprint field is empty): nothing to review, and no review hash to \
                 give it",
                row.symbol
            )),
            (Some(fp), None) => no_footprint(format!(
                "its footprint {fp} was not found: {}",
                row.note.as_deref().unwrap_or("no such library footprint")
            )),
            (Some(_), Some(file)) => match FootprintSource::File(file.clone()).read() {
                Ok(s) => (Some(s), None),
                Err(e) => no_footprint(e.to_string()),
            },
        };
        match sourced {
            Some(mut s) => {
                // The symbol's fields first (Datasheet, part numbers, Footprint_Review), then the footprint's
                let search = self.library_search_for(&from);
                let mut fields = search
                    .open(&from.dir())
                    .ok()
                    .and_then(|(mut syms, _)| syms.fields(&row.symbol).ok())
                    .unwrap_or_else(|| {
                        row.datasheet
                            .iter()
                            .map(|d| ("Datasheet".to_string(), d.clone()))
                            .collect()
                    });
                fields.retain(|(k, v)| {
                    !v.trim().is_empty() && !(k == "Datasheet" && v.trim() == "~")
                });
                for (k, v) in std::mem::take(&mut s.fields) {
                    if !fields.iter().any(|f| f.0 == k) {
                        fields.push((k, v));
                    }
                }
                s.fields = fields;
                self.set_sourced(s, fresh);
                if fresh {
                    self.rt.want_search = true;
                }
            }
            None => {
                self.rt.symbol_note = note;
                self.rt.sourced = None;
                self.rt.items.clear();
            }
        }
        if fresh {
            self.usage("symbol");
        }
    }

    fn open_board(&mut self, path: &Path) {
        match better_erc::fp::read_board(path) {
            Ok(layout) => {
                let mut groups: Vec<(String, Vec<String>, usize)> = Vec::new();
                for (i, f) in layout.footprints.iter().enumerate() {
                    match groups.iter_mut().find(|g| g.0 == f.lib_id) {
                        Some(g) => g.1.push(f.reference.clone()),
                        None => groups.push((f.lib_id.clone(), vec![f.reference.clone()], i)),
                    }
                }
                for g in &mut groups {
                    g.1.sort_by(|a, b| better_erc::model::natural_cmp(a, b));
                }
                groups.sort_by(|a, b| a.0.cmp(&b.0));
                self.board = Some(path.to_path_buf());
                self.rt.board = Some(BoardList {
                    path: path.to_path_buf(),
                    layout,
                    groups,
                    filter: String::new(),
                });
                self.rt.error = None;
            }
            Err(e) => self.rt.error = Some(e.to_string()),
        }
    }

    /// Picks footprint `i` of the open board to preview: its library file when the project's tables give it, else
    /// the board copy
    fn pick(&mut self, i: usize) {
        let Some(b) = &self.rt.board else { return };
        let copy = Sourced::from_board(&b.path, &b.layout, FootprintId::from_index(i));
        let project = b.path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let library = FootprintSource::Library {
            lib_id: copy.lib_id().to_string(),
            project,
            config: self.rt.config.clone(),
        };
        let (reviewed, note) = match library.read() {
            Ok(lib) => {
                let note = if lib.hash == copy.hash {
                    format!(
                        "the board copy {} is the same",
                        copy.footprint.footprint.reference
                    )
                } else {
                    format!(
                        "the board copy {} differs from the library ({}): update the board from the library",
                        reference(&copy),
                        copy.hash
                    )
                };
                (lib, note)
            }
            Err(e) => (
                copy,
                format!("reviewing the board copy: the library footprint was not found ({e})"),
            ),
        };
        // The board copy's fields (Datasheet, Footprint_Review from its symbol) where the library file has none
        let mut reviewed = reviewed;
        let copy_fields =
            Sourced::from_board(&b.path, &b.layout, FootprintId::from_index(i)).fields;
        for (k, v) in copy_fields {
            if reviewed.field(&k).is_none() {
                reviewed.fields.push((k, v));
            }
        }
        self.source = Some(reviewed.source.clone());
        self.symbol = None;
        self.rt.target = None;
        self.rt.symbol_note = None;
        self.set_sourced(reviewed, true);
        self.rt.board_note = Some(note);
        self.usage("pick");
        self.rt.want_search = true;
    }

    fn load(&mut self, source: FootprintSource, fresh: bool) {
        match source.read() {
            Ok(s) => {
                self.source = Some(source);
                self.symbol = None;
                self.rt.target = None;
                self.rt.symbol_note = None;
                self.set_sourced(s, fresh);
                self.rt.board_note = None;
            }
            Err(e) => {
                self.rt.error = Some(e.to_string());
                self.rt.sourced = None;
                self.rt.items.clear();
            }
        }
    }

    /// The open board's mask and paste margins, for a footprint of that board or of its project's libraries
    fn board_margins(&self, s: &Sourced) -> outline::Margins {
        let Some(b) = &self.rt.board else {
            return outline::Margins::default();
        };
        let of_board = match &s.source {
            FootprintSource::Board { board, .. } => *board == b.path,
            FootprintSource::Library { project, .. } => Some(project.as_path()) == b.path.parent(),
            FootprintSource::File(_) => false,
        };
        if of_board {
            outline::Margins::of_setup(&b.layout.setup)
        } else {
            outline::Margins::default()
        }
    }

    fn set_sourced(&mut self, s: Sourced, fresh: bool) {
        self.rt.items = outline::items(&s.footprint, self.board_margins(&s));
        self.rt.fp_snaps = outline::snap_points(&s.footprint);
        self.rt.error = None;
        self.rt.accepted = None;
        self.rt.written = None;
        if fresh {
            self.rt.center_footprint = true;
            self.place.rotation = 0.0;
            self.place.mirror = false;
            self.rt.queries = search_queries(&s);
            self.rt.query = self.rt.queries.first().cloned().unwrap_or_default();
            self.rt.search_result = None;
        }
        self.rt.sourced = Some(s);
    }

    /// The footprint's `Datasheet` field as a local file, when it is one
    fn datasheet_file(&self) -> Option<PathBuf> {
        let ds = self.rt.sourced.as_ref()?.field("Datasheet")?;
        let p = PathBuf::from(ds.strip_prefix("file://").unwrap_or(ds));
        p.is_file().then_some(p)
    }

    /// Opens a datasheet the library search found; the page search picks the page (the hit's own page is where the
    /// part number matched, rarely the land pattern)
    fn open_hit(&mut self, ctx: &egui::Context, hit: &LibraryHit) {
        self.open_datasheet(ctx, &hit.path);
    }

    /// No datasheet open: the page, the ruler and the scale go with it
    fn close_datasheet(&mut self) {
        self.rt.pdf = None;
        self.datasheet = None;
        self.page = 0;
        self.rt.page_hits = None;
        self.rt.auto_page = false;
        self.ruler = None;
        self.scale = None;
        self.rt.accepted = None;
        self.clear_selection();
    }

    pub fn open_datasheet(&mut self, ctx: &egui::Context, path: &Path) {
        // A datasheet opened by hand is the one to show: no search for another
        self.rt.want_search = false;
        self.rt.auto_open = false;
        let fonts = &self.rt.settings.font_dirs;
        self.rt.pdf = Some(PdfDoc::open(ctx, path, fonts, self.threads_wake()));
        self.datasheet = Some(path.to_path_buf());
        self.page = 0;
        self.rt.page_hits = None;
        self.rt.auto_page = true;
        self.rt.fit_page = true;
        self.rt.accepted = None;
        self.ruler = None;
        self.scale = None;
        self.clear_selection();
        self.usage("datasheet");
    }

    /// Searches the library for the query typed, or (`auto`) for each query the footprint suggests until one finds
    /// something
    fn library_search(&mut self, ctx: &egui::Context, auto: bool) {
        let cmd = self.rt.settings.library_search.clone();
        let queries = if auto {
            self.rt.queries.clone()
        } else {
            vec![self.rt.query.clone()]
        };
        let queries: Vec<Vec<String>> = queries
            .iter()
            .map(|q| q.split_whitespace().map(String::from).collect::<Vec<_>>())
            .filter(|w| !w.is_empty())
            .collect();
        let search: Searcher = match &self.rt.searcher {
            Some(f) => f.clone(),
            None if cmd.is_empty() || find_program(&cmd[0]).is_none() => return,
            None => Arc::new(move |words: &[String]| run_library_search(&cmd, words)),
        };
        if queries.is_empty() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let wake = self.threads_wake().then(|| ctx.clone());
        std::thread::spawn(move || {
            let mut r = Ok((Vec::new(), String::new()));
            for words in &queries {
                r = search(words).map(|hits| (hits, words.join(" ")));
                if r.as_ref().is_ok_and(|(hits, _)| !hits.is_empty()) {
                    break;
                }
            }
            let _ = tx.send(r);
            if let Some(ctx) = wake {
                ctx.request_repaint();
            }
        });
        self.rt.searching = Some(rx);
        self.rt.search_result = None;
        self.rt.auto_open = auto;
        self.usage(if auto {
            "library-search auto"
        } else {
            "library-search"
        });
    }

    fn blend_mode(&self) -> BlendMode {
        self.blend.unwrap_or(self.rt.settings.blend)
    }

    fn layer_colors(&self) -> &LayerColors {
        &self.rt.settings.colors
    }

    /// Whether the page is shown inverted (white-ish on black)
    fn page_inverted(&self) -> bool {
        self.invert.unwrap_or(self.rt.settings.invert)
    }

    /// Cycles which of copper, mask or paste is shown, in that order
    fn cycle_cu_layer(&mut self) {
        let i = VIEWS.iter().position(|k| *k == self.cu_shown).unwrap_or(0);
        self.cu_shown = VIEWS[(i + 1) % VIEWS.len()];
        self.usage("layer");
    }

    /// The current page's picture for the blend pass, inverted when asked (the picture's colours, not the
    /// footprint's); none while a different page's picture is still loading
    fn page_image(&mut self, ctx: &egui::Context, invert: bool) -> Option<blend::PageImage> {
        let page = self.page;
        let pdf = self.rt.pdf.as_mut()?;
        if pdf.picture.as_ref()?.page != page {
            return None;
        }
        if invert {
            let (image, _, generation) = pdf.inverted(ctx)?;
            Some(blend::PageImage {
                generation,
                image: image.clone(),
            })
        } else {
            let pic = pdf.picture.as_ref()?;
            Some(blend::PageImage {
                generation: pic.generation,
                image: pic.image.clone(),
            })
        }
    }

    /// Millimetres per page point: the scale set with the ruler, else the PDF's own
    fn mm_per_point(&self) -> f64 {
        self.scale.as_ref().map_or(MM_PER_POINT, |s| s.mm_per_point)
    }

    /// Why Accept can't be used now, when it can't
    fn accept_blocker(&self) -> Option<String> {
        if self.rt.sourced.is_none() {
            return Some(match (&self.rt.target, &self.rt.symbol_note) {
                (Some(_), Some(n)) => n.clone(),
                _ => "pick a symbol first: its footprint and datasheet follow".into(),
            });
        }
        if self.rt.target.is_none() {
            return Some(
                "a footprint opened alone is a preview: there is no symbol to put the review hash on; pick its \
                 symbol from a library or a board's list"
                    .into(),
            );
        }
        if self.rt.pdf.as_ref().is_none_or(|p| p.page_count() == 0) {
            return Some("open the datasheet first".into());
        }
        match &self.scale {
            None => Some("set the drawing's scale with the ruler first".into()),
            Some(s) if s.page != self.page => Some(format!(
                "the scale was set on page {}: set it again on this page",
                s.page + 1
            )),
            Some(_) => None,
        }
    }

    /// Accept: the review recorded and its hash copied; for a symbol, first the question whether to write it there,
    /// with the dry run's files (when the dry run fails, the review is recorded with why nothing was written)
    fn accept(&mut self, ctx: &egui::Context) {
        let Some(review) = self.review() else { return };
        let Some(target) = self.rt.target.clone() else {
            self.finish_accept(ctx, review, None);
            return;
        };
        match self.write(&target, &review.hash, true) {
            Ok(run) => {
                let warnings = kicad_warnings(&run, (self.rt.kicad_running)());
                self.rt.confirm = Some(Confirm {
                    review,
                    target,
                    run,
                    warnings,
                });
                self.usage("confirm");
            }
            Err(e) => self.finish_accept(ctx, review, Some(Err(e))),
        }
    }

    /// The confirmation's answer: write (Enter), or nothing at all (Esc)
    fn confirmed(&mut self, ctx: &egui::Context, write: bool) {
        let Some(c) = self.rt.confirm.take() else {
            return;
        };
        if write {
            let written = self.write(&c.target, &c.review.hash, false);
            self.finish_accept(ctx, c.review, Some(written));
        } else {
            self.usage("cancel");
        }
    }

    /// The review of what is shown now
    fn review(&self) -> Option<Review> {
        let s = self.rt.sourced.as_ref()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let review = Review {
            lib_id: s.lib_id().to_string(),
            hash: s.hash.clone(),
            source: s.source.describe(),
            datasheet: self.datasheet.clone(),
            page: self.page + 1,
            mm_per_point: self.mm_per_point(),
            when: now,
        };
        Some(review)
    }

    /// The review accepted: its hash copied and kept, with what was written into the symbol
    fn finish_accept(
        &mut self,
        ctx: &egui::Context,
        review: Review,
        written: Option<Result<AcceptRun, String>>,
    ) {
        ctx.copy_text(review.hash.clone());
        self.rt.accepted = Some(review.hash.clone());
        self.rt.written = written;
        if matches!(self.rt.written, Some(Ok(_))) {
            self.usage("write");
            self.reviewed(&review.hash);
        }
        self.reviews.push(review);
        self.usage("accept");
    }

    /// Writes `hash` into the symbol under review; `dry_run` says what it would write
    fn write(&self, t: &Target, hash: &str, dry_run: bool) -> Result<AcceptRun, String> {
        match &t.from {
            SymbolsFrom::Project(p) => {
                let config = self.rt.config.clone();
                better_erc::fp::accept(
                    p,
                    &AcceptWhat::Symbol {
                        symbol: t.symbol.clone(),
                        hash: hash.to_string(),
                    },
                    &mut |dir| SymbolLibraries::new(dir, config.as_deref()),
                    dry_run,
                )
            }
            SymbolsFrom::Library { project, .. } => {
                let search = self.library_search_for(&t.from);
                let (mut syms, _) = search.open(project).map_err(|e| e.to_string())?;
                better_erc::fp::accept_library(&mut syms, &t.symbol, hash, dry_run)
            }
        }
        .map_err(|e| e.to_string())
    }

    /// The symbol under review holds `hash` now: its row and fields say so
    fn reviewed(&mut self, hash: &str) {
        let Some(t) = &self.rt.target else { return };
        if let Some(row) = self
            .rt
            .symbols
            .as_mut()
            .and_then(|l| l.rows.iter_mut().find(|r| r.symbol == t.symbol))
        {
            row.stored = Some(hash.to_string());
            if row.hash.as_deref() == Some(hash) {
                row.status = SymbolStatus::Ok;
            }
        }
        if let Some(s) = &mut self.rt.sourced {
            match s.fields.iter_mut().find(|f| f.0 == FIELD) {
                Some(f) => f.1 = hash.to_string(),
                None => s.fields.push((FIELD.to_string(), hash.to_string())),
            }
        }
    }

    fn set_scale_from_ruler(&mut self) {
        if self.tool.measures() {
            return;
        }
        let Some([a, b]) = self.ruler else { return };
        let Ok(mm) = self.rt.ruler_mm.trim().replace(',', ".").parse::<f64>() else {
            return;
        };
        let pts = ((b[0] - a[0]) as f64).hypot((b[1] - a[1]) as f64);
        if mm <= 0.0 || pts <= 0.0 {
            return;
        }
        self.scale = Some(Scale {
            mm_per_point: mm / pts,
            page: self.page,
            mm,
            note: None,
        });
        self.scale_set();
        self.usage("scale");
    }

    /// The scale is set: the ruler step is over, and the footprint is the one to move
    fn scale_set(&mut self) {
        self.rt.accepted = None;
        self.set_tool(Tool::Move);
    }

    /// Into mode `t`, out of any ruler step; the ruler shown is that mode's: the scale's in Move and Scale, the
    /// measuring one in the measuring modes
    fn set_tool(&mut self, t: Tool) {
        self.ruler_done();
        self.rt.length_box = false;
        if t.measures() != self.tool.measures() {
            std::mem::swap(&mut self.ruler, &mut self.rt.parked_ruler);
        }
        if t != self.tool {
            self.tool = t;
            self.usage(&format!("mode {}", t.name().to_lowercase()));
        }
    }

    /// A mode's key or button: into that mode, or back to Move when already in it
    fn tool_key(&mut self, t: Tool) {
        self.set_tool(if self.tool == t { Tool::Move } else { t });
    }

    /// Out of the ruler's steps: no end follows the pointer, and a view zoomed in on an end goes back
    fn ruler_done(&mut self) {
        self.rt.ruler_placing = false;
        self.rt.ruler_anchor = None;
        self.rt.ruler_focus = None;
        if let Some((zoom, pan)) = self.rt.overview.take() {
            self.zoom = zoom;
            self.pan = pan;
        }
    }

    /// The ruler key: the first end at the pointer (`at`, page points; snapped to the drawing's lines unless
    /// `lines` is false), the other following it; pressed again, the other end stays where the pointer is
    fn ruler_key(&mut self, at: [f32; 2], free: bool, lines: bool) {
        if self.rt.ruler_placing && self.ruler.is_some() {
            self.move_ruler_end(1, at, free, lines);
            self.rt.ruler_placing = false;
            self.rt.ruler_anchor = None;
            return;
        }
        self.ruler_done();
        if !self.tool.ruler() {
            self.set_tool(Tool::Scale);
        }
        let at = self.start_point(at, lines);
        self.ruler = Some([at, at]);
        self.rt.measured = None;
        self.rt.ruler_anchor = Some(at);
        self.rt.ruler_placing = true;
        self.usage("ruler key");
    }

    /// Ruler end `i` to `p` (page points). Snapped (the setting, unless `free`: Shift held), the ruler stays level:
    /// the other end follows on its axis, taking `p`'s Y on a horizontal ruler and its X on a vertical one. The axis
    /// is the ruler's own, or while a new ruler is laid, the larger of dx and dy from its first end. With `lines`
    /// (Alt not held), `p` first snaps to the drawing's lines ([`Self::line_snap`]), each coordinate on its own on
    /// a level ruler.
    fn move_ruler_end(&mut self, i: usize, p: [f32; 2], free: bool, lines: bool) {
        let snap = self.rt.settings.ruler_snap && !free && self.tool != Tool::MeasureFootprint;
        self.rt.snap = if lines { self.end_snap(p, snap) } else { None };
        let p = self.rt.snap.as_ref().map_or(p, |s| s.at);
        let anchor = self.rt.ruler_anchor;
        let Some(r) = &mut self.ruler else { return };
        let o = 1 - i;
        let (from, to) = match anchor {
            Some(a) => (a, p),
            None => (r[o], r[i]),
        };
        r[i] = p;
        if snap {
            let a = anchor.unwrap_or(r[o]);
            r[o] = if (to[0] - from[0]).abs() >= (to[1] - from[1]).abs() {
                [a[0], p[1]]
            } else {
                [p[0], a[1]]
            };
        } else if let Some(a) = anchor {
            r[o] = a;
        }
    }

    /// Where a ruler end at `p` (page points) snaps to the current page's lines, within the setting's pixels at
    /// the view's zoom; `level`: each coordinate on its own ([`geometry::PageGeometry::snap`])
    fn line_snap(&self, p: [f32; 2], level: bool) -> Option<Snap> {
        let g = self.rt.pdf.as_ref()?.geometry(self.page)?;
        g.snap(p, self.rt.settings.line_snap_px / self.zoom, level)
    }

    /// Where a ruler end at `p` snaps in the current mode: to the footprint's pads when measuring the footprint,
    /// else to the drawing's lines
    fn end_snap(&self, p: [f32; 2], level: bool) -> Option<Snap> {
        if self.tool == Tool::MeasureFootprint {
            self.pad_snap(p)
        } else {
            self.line_snap(p, level)
        }
    }

    /// The footprint's pad centre or pad side middle nearest `p` (page points), within twice the line snap's
    /// pixels (at least 20) at the view's zoom
    fn pad_snap(&self, p: [f32; 2]) -> Option<Snap> {
        let reach = self.rt.settings.line_snap_px.max(10.0) * 2.0 / self.zoom;
        self.rt
            .fp_snaps
            .iter()
            .map(|s| (self.fp_to_page(s.at), s.centre))
            .map(|(at, centre)| (at, centre, (at[0] - p[0]).hypot(at[1] - p[1])))
            .filter(|(_, _, d)| *d <= reach)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(at, centre, _)| Snap {
                at,
                kind: if centre {
                    SnapKind::PadCentre
                } else {
                    SnapKind::PadSide
                },
                segs: Vec::new(),
            })
    }

    /// A new ruler's first end at `p`: snapped (to the drawing's lines or the pads, by the mode) with `lines`,
    /// shown so
    fn start_point(&mut self, p: [f32; 2], lines: bool) -> [f32; 2] {
        self.rt.snap = if lines { self.end_snap(p, false) } else { None };
        self.rt.snap.as_ref().map_or(p, |s| s.at)
    }

    /// The number the dimension prints between the ruler's ends, on the current page
    fn dimension_number(&self) -> Option<DimNumber> {
        let [a, b] = self.ruler?;
        self.rt
            .pdf
            .as_ref()?
            .geometry(self.page)?
            .dimension_number(a, b)
    }

    /// The dimension's number sets the scale from the ruler: the dimension key or a click on its chip
    fn use_dimension_number(&mut self) {
        if self.tool.measures() {
            return;
        }
        let Some(n) = self.dimension_number() else {
            return;
        };
        self.rt.ruler_mm = n.text.replace(',', ".");
        self.rt.length_box = false;
        self.set_scale_from_ruler();
        self.usage("dimension number");
    }

    /// The ruler zoom key: in on the first end, then on the other, then back to the view before, the length field
    /// taking the keyboard
    fn ruler_zoom_key(&mut self, canvas: Rect) {
        let Some(r) = self.ruler else { return };
        self.rt.ruler_placing = false;
        let next = match self.rt.ruler_focus {
            None => Some(0),
            Some(0) => Some(1),
            Some(_) => None,
        };
        match next {
            Some(i) => {
                let (zoom, _) = *self.rt.overview.get_or_insert((self.zoom, self.pan));
                self.zoom = (zoom * self.rt.settings.ruler_zoom.max(1.0)).clamp(0.05, 400.0);
                let p = r[i];
                self.pan = [
                    canvas.width() / 2.0 - p[0] * self.zoom,
                    canvas.height() / 2.0 - p[1] * self.zoom,
                ];
                if !self.tool.ruler() {
                    self.set_tool(Tool::Scale);
                }
                self.rt.ruler_focus = Some(i);
                self.usage("ruler zoom");
            }
            None if self.tool.measures() => self.ruler_done(),
            None => self.scale_key(),
        }
    }

    /// The scale key: out of the ruler step, back to the view before any zoom on an end, the ruler's length asked
    /// in a box in the middle of the canvas, empty; Enter there sets the scale and makes the footprint the one to
    /// move, Esc leaves it
    fn scale_key(&mut self) {
        if self.ruler.is_none() || self.tool.measures() {
            return;
        }
        self.ruler_done();
        self.rt.ruler_mm.clear();
        self.rt.length_box = true;
        self.usage("scale key");
    }

    /// The box asking the ruler's length, in the middle of `canvas`: the number typed large, in mm; Enter sets the
    /// scale, Esc leaves the box without one
    fn length_box(&mut self, ctx: &egui::Context, canvas: Rect, id: Id) {
        let (mut set, mut cancel) = (false, false);
        egui::Area::new(id.with("ruler length box"))
            .order(egui::Order::Foreground)
            .fixed_pos(canvas.center())
            .pivot(Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("The ruler's length").size(16.0));
                        ui.horizontal(|ui| {
                            let r = ui.add(
                                TextEdit::singleline(&mut self.rt.ruler_mm)
                                    .font(FontId::monospace(40.0))
                                    .desired_width(200.0)
                                    .horizontal_align(egui::Align::RIGHT)
                                    .id(id.with("ruler length")),
                            );
                            ui.label(RichText::new("mm").size(40.0));
                            let (enter, esc) = ui.input(|i| {
                                (
                                    i.key_pressed(egui::Key::Enter),
                                    i.key_pressed(egui::Key::Escape),
                                )
                            });
                            set = enter;
                            cancel = esc;
                            // Only when it lacks the keyboard: each request interrupts the input method, which on
                            // Wayland loses what was typed and wakes the next frame
                            if !set && !cancel && !r.has_focus() {
                                r.request_focus();
                            }
                        });
                        ui.label(RichText::new("Enter sets the scale · Esc leaves it").weak());
                    });
            });
        self.rt.typing_mm = self.rt.length_box;
        if set {
            self.rt.length_box = false;
            self.set_scale_from_ruler();
        } else if cancel {
            self.rt.length_box = false;
        }
    }

    /// The keys as the settings name them; a name that is no key leaves its step without one
    fn keys(&self) -> KeyMap {
        KeyMap::from(&self.rt.settings.keys)
    }

    /// One line on the canvas' keys and mouse
    fn help_line(&self) -> String {
        let k = self.keys();
        let n = key_name;
        format!(
            "Mode {}: {} move · {} ruler, setting the scale · {} measure the PDF · {} measure the footprint · \
             {} select the drawing, {} saves it as DXF · a mode's key again, {} or Esc: back to Move\n\
             {} full width · {} ruler: one end, {} the other · {} zoom on a ruler end, again the other, again back · \
             {} type the ruler's length, Enter sets the scale · {} the dimension's number sets it · \
             {} · {}\n{} turn 90° · {} bottom view · {} blink · {} copper/mask/paste · {} invert the page · \
             arrows nudge (Shift: 10×) · PgUp/PgDn page · wheel zoom · right or middle drag pans\n\
             {} accept · {} next to review · {} write a review note · SpaceMouse: the puck {} ({} switches), push away zooms out · \
             tap the left button for the ruler, the right one to switch · hold one for its ring, both for Accept",
            self.tool.name(),
            n(k.move_tool),
            n(k.ruler),
            n(k.measure_pdf),
            n(k.measure_footprint),
            n(k.select),
            n(k.save_dxf),
            n(k.move_tool),
            n(k.full_width),
            n(k.ruler),
            n(k.ruler),
            n(k.ruler_zoom),
            n(k.scale),
            n(k.dimension),
            if self.rt.settings.ruler_snap {
                "the ruler snaps level, Shift: any angle"
            } else {
                "the ruler at any angle (Settings: snap)"
            },
            if self.rt.settings.line_snap_px > 0.0 {
                "its ends snap to the drawing's lines, Alt: free"
            } else {
                "its ends free of the drawing's lines (Settings: snap)"
            },
            n(k.rotate),
            n(k.mirror),
            n(k.blink),
            n(k.cu_layer),
            n(k.invert),
            n(k.accept),
            n(k.next),
            n(k.note),
            match self.puck {
                PuckMoves::Page => "moves the page",
                PuckMoves::Footprint => "moves the footprint",
            },
            n(k.puck),
        )
    }

    /// What the ruler step waits for, when it is on
    fn ruler_hint(&self) -> Option<String> {
        let k = self.keys();
        let what = match self.tool {
            Tool::MeasurePdf => "Measuring the PDF",
            Tool::MeasureFootprint => "Measuring the footprint",
            Tool::Select => return Some(self.select_hint()),
            Tool::Move | Tool::Scale => "Ruler",
        };
        if self.rt.ruler_placing {
            return Some(format!(
                "{what}: point at the other end and press {} or click",
                key_name(k.ruler)
            ));
        }
        match self.rt.ruler_focus {
            Some(0) => Some(format!(
                "{what}, first end: click or drag where it belongs exactly; {} for the other end",
                key_name(k.ruler_zoom)
            )),
            Some(_) if self.tool.measures() => Some(format!(
                "{what}, other end: click or drag where it belongs exactly; {} back",
                key_name(k.ruler_zoom)
            )),
            Some(_) => Some(format!(
                "Ruler, other end: click or drag where it belongs exactly; {} to type its length",
                key_name(k.scale)
            )),
            None if self.rt.typing_mm => {
                Some("Ruler: type its length in mm, Enter sets the scale, Esc leaves it".into())
            }
            None => None,
        }
    }

    fn controls(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        ui.heading("Symbol");
        ui.horizontal_wrapped(|ui| {
            if ui
                .button("Open board or project…")
                .on_hover_text("A KiCad project (.kicad_pro, .kicad_sch, .kicad_pcb): the library symbols its board uses")
                .clicked()
                && let Some(p) = rfd::FileDialog::new()
                    .add_filter("KiCad project", &["kicad_pro", "kicad_sch", "kicad_pcb"])
                    .pick_file()
            {
                self.open_path(&ctx, &p);
            }
            if ui
                .button("Open symbol library…")
                .on_hover_text("A .kicad_sym file: its symbols")
                .clicked()
                && let Some(p) = rfd::FileDialog::new()
                    .add_filter("KiCad symbol library", &["kicad_sym"])
                    .pick_file()
            {
                self.open_path(&ctx, &p);
            }
            if ui
                .button("Open symbol folder…")
                .on_hover_text("A .kicad_symdir directory (KiCad 9 and later): its symbols")
                .clicked()
                && let Some(p) = rfd::FileDialog::new().pick_folder()
            {
                self.open_path(&ctx, &p);
            }
        });
        if let Some(e) = &self.rt.error {
            ui.colored_label(Color32::RED, e);
        }
        self.table_libraries(ui);
        self.symbol_list(ui);
        self.symbol_info(ui);

        ui.separator();
        ui.heading("Footprint");
        if ui
            .button("Open a footprint alone (preview)…")
            .on_hover_text(
                "A library footprint (.kicad_mod) to look at over a datasheet; Accept needs a symbol to write into",
            )
            .clicked()
            && let Some(p) = rfd::FileDialog::new()
                .add_filter("KiCad footprint", &["kicad_mod"])
                .pick_file()
        {
            self.open_path(&ctx, &p);
        }
        self.board_list(ui);
        if let Some(s) = &self.rt.sourced {
            if self.rt.target.is_none() {
                ui.colored_label(
                    Color32::ORANGE,
                    "Preview: no symbol to write the review hash into",
                );
            }
            Grid::new("fp").num_columns(2).show(ui, |ui| {
                ui.label("Footprint");
                // Truncated: a long footprint name would widen the panel past its edge and hide the layer boxes
                ui.add(Label::new(RichText::new(s.lib_id()).strong()).truncate())
                    .on_hover_text(s.lib_id());
                ui.end_row();
                ui.label("From");
                ui.add(Label::new(short_source(&s.source)).truncate())
                    .on_hover_text(s.source.describe());
                ui.end_row();
                ui.label("Review hash");
                ui.label(RichText::new(&s.hash).monospace());
                ui.end_row();
                if let Some(stored) = s.field(FIELD)
                    && self.rt.target.is_some()
                {
                    ui.label(FIELD);
                    if stored == s.hash {
                        ui.colored_label(Color32::GREEN, format!("{stored}, the same: reviewed"));
                    } else {
                        ui.colored_label(
                            Color32::ORANGE,
                            format!("{stored}: not this footprint, review it again"),
                        );
                    }
                    ui.end_row();
                }
            });
            if let Some(n) = &self.rt.board_note {
                ui.label(RichText::new(n).small());
            }
        }

        ui.separator();
        ui.heading("Datasheet");
        self.datasheet_controls(ui, &ctx);

        ui.separator();
        ui.heading("Scale");
        self.scale_controls(ui);

        ui.separator();
        ui.heading("Overlay");
        self.overlay_controls(ui);

        ui.separator();
        ui.heading("Accept");
        let blocker = self.accept_blocker();
        let resp = ui.add_enabled(
            blocker.is_none(),
            Button::new("Accept: the footprint matches the drawing"),
        );
        let resp = match &blocker {
            Some(why) => resp.on_disabled_hover_text(why),
            None => match &self.rt.target {
                Some(Target {
                    symbol,
                    from: SymbolsFrom::Project(_),
                    ..
                }) => resp.on_hover_text(format!(
                    "Writes the review hash into the {FIELD} field of {symbol}: the library symbol and the \
                     schematic's copies, after showing what it writes where; copies it too"
                )),
                Some(t) => resp.on_hover_text(format!(
                    "Writes the review hash into the {FIELD} field of library symbol {}, after showing what it \
                     writes where; copies it too",
                    t.symbol
                )),
                None => resp,
            },
        };
        if let Some(why) = &blocker {
            ui.label(RichText::new(why).small().weak());
        }
        if resp.clicked() {
            self.accept(&ctx);
        }
        if let Some(h) = &self.rt.accepted {
            ui.label(format!("Accepted. {FIELD} = {h} (copied)"));
            ui.horizontal(|ui| {
                ui.label(RichText::new(h).monospace());
                if ui.small_button("Copy").clicked() {
                    ctx.copy_text(h.clone());
                }
            });
        }
        match &self.rt.written {
            Some(Ok(run)) => {
                for a in &run.symbols {
                    ui.label(format!("{}: written into", a.symbol));
                    for w in &a.files {
                        let kind = match w.source {
                            HashSource::Library => "library",
                            HashSource::Schematic => "schematic copy",
                        };
                        let what = if w.changed { "" } else { ", had it already" };
                        let name = w
                            .file
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        ui.add(
                            Label::new(RichText::new(format!("  {name} ({kind}{what})")).small())
                                .truncate(),
                        )
                        .on_hover_text(w.file.display().to_string());
                    }
                    if let Some(n) = &a.library_note {
                        ui.colored_label(
                            Color32::ORANGE,
                            format!("Library symbol not written: {n}"),
                        );
                    }
                }
            }
            Some(Err(e)) => {
                ui.colored_label(Color32::RED, format!("Not written into the symbols: {e}"));
            }
            None => {}
        }
        if self.rt.symbols.is_some() {
            let next = self.next_to_review();
            let b = ui.add_enabled(next.is_some(), Button::new("Next to review ▶"));
            let b = match next {
                Some(i) => b.on_hover_text(
                    self.rt
                        .symbols
                        .as_ref()
                        .map_or(String::new(), |l| l.rows[i].symbol.clone()),
                ),
                None => b.on_disabled_hover_text("no other symbol in the list needs a review"),
            };
            if b.clicked()
                && let Some(i) = next
            {
                self.select_symbol(i, true);
                self.usage("next");
            }
        }
        let n = self.reviews.len();
        egui::CollapsingHeader::new(format!("Accepted reviews ({n})"))
            .id_salt("reviews")
            .default_open(false)
            .show(ui, |ui| {
                for r in self.reviews.iter().rev() {
                    ui.label(format!(
                        "{}  {}  page {}  {}",
                        r.lib_id,
                        r.hash,
                        r.page,
                        r.datasheet.as_deref().map_or(String::new(), |d| d
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                            .unwrap_or_default())
                    ));
                }
            });

        ui.separator();
        self.puck_controls(ui);
        self.settings_controls(ui);
    }

    /// The symbol libraries the tables name, to open one
    fn table_libraries(&mut self, ui: &mut Ui) {
        let dir = self
            .rt
            .symbols
            .as_ref()
            .map_or_else(|| PathBuf::from("."), |l| l.from.dir());
        let mut open = None;
        egui::CollapsingHeader::new("Symbol libraries in the tables")
            .id_salt("table libraries")
            .default_open(false)
            .show(ui, |ui| {
                if self.rt.tables.as_ref().is_none_or(|t| t.0 != dir) {
                    let libs = SymbolLibraries::new(&dir, self.rt.config.as_deref());
                    let names = libs
                        .tables()
                        .libraries()
                        .iter()
                        .map(|e| e.name.clone())
                        .collect();
                    self.rt.tables = Some((dir.clone(), names));
                }
                let names = &self.rt.tables.as_ref().expect("read above").1;
                if names.is_empty() {
                    ui.label(
                        RichText::new("no sym-lib-table here or in KiCad's configuration")
                            .small()
                            .weak(),
                    );
                    return;
                }
                ui.add(
                    TextEdit::singleline(&mut self.rt.tables_filter).hint_text("filter: nickname"),
                );
                let f = self.rt.tables_filter.to_lowercase();
                ScrollArea::vertical()
                    .id_salt("table libraries list")
                    .max_height(160.0)
                    .show(ui, |ui| {
                        for n in names {
                            if (f.is_empty() || n.to_lowercase().contains(&f))
                                && ui.selectable_label(false, n).clicked()
                            {
                                open = Some(n.clone());
                            }
                        }
                    });
            });
        if let Some(n) = open {
            self.open_symbols(SymbolsFrom::Library {
                library: n,
                project: dir,
            });
            self.usage("library");
        }
    }

    /// The symbols to review, coloured by what each needs
    fn symbol_list(&mut self, ui: &mut Ui) {
        let mut picked = None;
        let current = self.rt.target.as_ref().map(|t| t.symbol.clone());
        let Some(l) = &mut self.rt.symbols else {
            ui.label(
                RichText::new(
                    "Open a board or a symbol library: the review is of a library symbol, which holds the hash",
                )
                .small()
                .weak(),
            );
            return;
        };
        egui::CollapsingHeader::new(&l.title)
            .id_salt("symbols")
            .default_open(true)
            .show(ui, |ui| {
                ui.add(
                    TextEdit::singleline(&mut l.filter)
                        .hint_text("filter: symbol, footprint or reference"),
                );
                let f = l.filter.to_lowercase();
                ScrollArea::vertical()
                    .id_salt("symbol rows")
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for (i, r) in l.rows.iter().enumerate() {
                            let hit = |t: &str| t.to_lowercase().contains(&f);
                            if !f.is_empty()
                                && !hit(&r.symbol)
                                && !r.footprint.as_deref().is_some_and(hit)
                                && !r.references.iter().any(|x| hit(x))
                            {
                                continue;
                            }
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    vec2(86.0, 14.0),
                                    Label::new(
                                        RichText::new(r.status.name())
                                            .small()
                                            .color(status_colour(r.status)),
                                    ),
                                );
                                let selected = current.as_deref() == Some(r.symbol.as_str());
                                let mut hover = r.symbol.clone();
                                if let Some(fp) = &r.footprint {
                                    hover.push_str(&format!("\nFootprint {fp}"));
                                }
                                if !r.references.is_empty() {
                                    hover.push_str(&format!("\non {}", r.references.join(", ")));
                                }
                                if let Some(n) = &r.note {
                                    hover.push_str(&format!("\n{n}"));
                                }
                                if ui
                                    .add(Button::selectable(selected, &r.symbol).truncate())
                                    .on_hover_text(hover)
                                    .clicked()
                                {
                                    picked = Some(i);
                                }
                            });
                        }
                    });
            });
        if let Some(i) = picked {
            self.select_symbol(i, true);
        }
    }

    /// The symbol under review: its footprint and datasheet fields, or why it has no footprint to review
    fn symbol_info(&self, ui: &mut Ui) {
        let Some(t) = &self.rt.target else { return };
        let row = self
            .symbol_row(&t.symbol)
            .map(|i| &self.rt.symbols.as_ref().expect("a row").rows[i]);
        Grid::new("symbol").num_columns(2).show(ui, |ui| {
            ui.label("Symbol");
            ui.label(RichText::new(&t.symbol).strong());
            ui.end_row();
            if let Some(r) = row {
                ui.label("Status");
                ui.colored_label(status_colour(r.status), r.status.name());
                ui.end_row();
                ui.label("Footprint");
                let fp = r.footprint.as_deref().unwrap_or("(none)");
                ui.add(Label::new(fp).truncate()).on_hover_text(fp);
                ui.end_row();
                if let Some(d) = &r.datasheet {
                    ui.label("Datasheet");
                    ui.add(Label::new(d).truncate()).on_hover_text(d);
                    ui.end_row();
                }
                if !r.references.is_empty() {
                    ui.label("On the board");
                    ui.add(Label::new(r.references.join(", ")).truncate());
                    ui.end_row();
                }
            }
        });
        if let Some(n) = &self.rt.symbol_note {
            ui.colored_label(Color32::ORANGE, n);
        }
    }

    fn board_list(&mut self, ui: &mut Ui) {
        let mut picked = None;
        let current = self.rt.sourced.as_ref().map(|s| s.lib_id().to_string());
        let Some(b) = &mut self.rt.board else { return };
        let title = format!(
            "{} library footprints on {} (preview)",
            b.groups.len(),
            b.path
                .file_name()
                .map(|f| f.to_string_lossy())
                .unwrap_or_default()
        );
        egui::CollapsingHeader::new(title)
            .id_salt("board footprints")
            .default_open(false)
            .show(ui, |ui| {
                ui.add(
                    TextEdit::singleline(&mut b.filter)
                        .hint_text("filter: library id or reference"),
                );
                let f = b.filter.to_lowercase();
                ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    for (lib_id, refs, i) in &b.groups {
                        if !f.is_empty()
                            && !lib_id.to_lowercase().contains(&f)
                            && !refs.iter().any(|r| r.to_lowercase().contains(&f))
                        {
                            continue;
                        }
                        let text = format!("{lib_id}  ({})", refs.join(", "));
                        let selected = current.as_deref() == Some(lib_id.as_str());
                        if ui
                            .add(Button::selectable(selected, text).truncate())
                            .on_hover_text(format!("{lib_id}\n{}", refs.join(", ")))
                            .clicked()
                        {
                            picked = Some(*i);
                        }
                    }
                });
            });
        if let Some(i) = picked {
            self.pick(i);
        }
    }

    fn datasheet_controls(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let field = self
            .rt
            .sourced
            .as_ref()
            .and_then(|s| s.field("Datasheet"))
            .map(String::from);
        let local = self.datasheet_file();
        match (&field, &local) {
            (Some(f), Some(p)) => {
                if ui
                    .button("Open the Datasheet field's file")
                    .on_hover_text(f)
                    .clicked()
                {
                    let p = p.clone();
                    self.open_datasheet(ctx, &p);
                }
            }
            (Some(f), None) if f.starts_with("http://") || f.starts_with("https://") => {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Datasheet field:");
                    ui.hyperlink_to(f, f);
                });
                ui.label(
                    RichText::new("A web address: find a local copy below, or save it and open it")
                        .small(),
                );
            }
            (Some(f), None) => {
                ui.label(format!("Datasheet field: {f} (no such file)"));
            }
            (None, _) if self.rt.sourced.is_some() => {
                ui.label(RichText::new("The footprint has no Datasheet field").small());
            }
            _ => {}
        }
        ui.horizontal(|ui| {
            if ui.button("Open PDF…").clicked()
                && let Some(p) = rfd::FileDialog::new()
                    .add_filter("PDF", &["pdf", "PDF"])
                    .pick_file()
            {
                self.open_datasheet(ctx, &p);
            }
        });
        // Library search
        let cmd = self.rt.settings.library_search.clone();
        let blocker = if self.rt.searcher.is_some() {
            None
        } else if cmd.is_empty() {
            Some("no library search command is set (Settings)".to_string())
        } else if find_program(&cmd[0]).is_none() {
            Some(format!(
                "{} is not on PATH (Settings: library search)",
                cmd[0]
            ))
        } else if self.rt.query.trim().is_empty() {
            Some("type the words to search for".into())
        } else if self.rt.searching.is_some() {
            Some("searching…".into())
        } else {
            None
        };
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut self.rt.query)
                    .hint_text("part number")
                    .desired_width(150.0),
            );
            let b = ui.add_enabled(blocker.is_none(), Button::new("Search library"));
            let b = match &blocker {
                Some(why) => b.on_disabled_hover_text(why),
                None => b.on_hover_text(cmd.join(" ")),
            };
            if b.clicked() {
                self.library_search(ctx, false);
            }
        });
        if let Some(why) = &blocker
            && self.rt.searching.is_none()
        {
            ui.label(RichText::new(why).small().weak());
        }
        let mut open = None;
        match &self.rt.search_result {
            Some(Ok(hits)) if hits.is_empty() => {
                ui.label("The library has nothing for that");
            }
            Some(Ok(hits)) => {
                // A panel of its own, so the found datasheets don't hide among the other controls
                let accent = Color32::from_rgb(0, 110, 200);
                egui::Frame::group(ui.style())
                    .stroke(Stroke::new(1.5, accent))
                    .fill(accent.gamma_multiply(0.12))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!(
                                "Found in the library ({}), the first opens by itself:",
                                hits.len()
                            ))
                            .strong(),
                        );
                        for h in hits {
                            let name = h
                                .path
                                .file_name()
                                .map(|f| f.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            let dir = h
                                .path
                                .parent()
                                .map(|d| d.display().to_string())
                                .unwrap_or_default();
                            let is_open = self.datasheet.as_ref() == Some(&h.path);
                            let text = RichText::new(format!(
                                "📄 {name}{}",
                                if is_open { "  (open)" } else { "" }
                            ))
                            .color(Color32::WHITE)
                            .strong();
                            let b = Button::new(text)
                                .fill(if is_open {
                                    accent
                                } else {
                                    accent.gamma_multiply(0.6)
                                })
                                .truncate();
                            if ui
                                .add_sized(vec2(ui.available_width(), 22.0), b)
                                .on_hover_text(format!("Open it\n{dir}"))
                                .clicked()
                            {
                                open = Some(h.clone());
                            }
                        }
                    });
            }
            Some(Err(e)) => {
                ui.colored_label(Color32::RED, e);
            }
            None => {}
        }
        if let Some(h) = open {
            self.open_hit(ctx, &h);
        }
        // The open datasheet and its pages
        let Some(pdf) = &self.rt.pdf else { return };
        ui.label(
            RichText::new(
                pdf.path
                    .file_name()
                    .map(|f| f.to_string_lossy())
                    .unwrap_or_default(),
            )
            .strong(),
        );
        if let Some(e) = &pdf.error {
            ui.colored_label(Color32::RED, e);
            return;
        }
        let pages = pdf.page_count();
        let mut page = self.page;
        ui.horizontal(|ui| {
            if ui.add_enabled(page > 0, Button::new("◀")).clicked() {
                page -= 1;
            }
            ui.label(format!("page {} of {pages}", page + 1));
            if ui.add_enabled(page + 1 < pages, Button::new("▶")).clicked() {
                page += 1;
            }
            if ui
                .button("Full width")
                .on_hover_text(format!(
                    "The page at the view's full width; {}",
                    key_name(KeyMap::from(&self.rt.settings.keys).full_width)
                ))
                .clicked()
            {
                self.rt.fit_width = true;
            }
            if ui.button("Whole page").clicked() {
                self.rt.fit_page = true;
            }
        });
        let words = &self.rt.settings.page_search_words;
        match &self.rt.page_hits {
            None => {
                ui.label(
                    RichText::new("finding the land pattern page…")
                        .small()
                        .weak(),
                );
            }
            Some(hits) if hits.is_empty() => {
                ui.label(RichText::new("no page has the search words (Settings)").small());
            }
            Some(hits) => {
                ui.label(RichText::new("Pages with the search words:").small());
                ui.horizontal_wrapped(|ui| {
                    for h in hits {
                        let w = h
                            .words
                            .iter()
                            .map(|i| words[*i].as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        if ui
                            .selectable_label(page == h.page, format!("p.{}", h.page + 1))
                            .on_hover_text(w)
                            .clicked()
                        {
                            page = h.page;
                            self.usage("page-search");
                        }
                    }
                });
            }
        }
        if page != self.page {
            self.page = page;
            self.rt.fit_page = true;
        }
    }

    fn scale_controls(&mut self, ui: &mut Ui) {
        let k = self.keys();
        let mut zoom_end = false;
        ui.horizontal(|ui| {
            let b = ui.add_enabled(self.ruler.is_some(), Button::new("Zoom on an end"));
            let b = if self.ruler.is_some() {
                b.on_hover_text(format!(
                    "In on the first end to place it exactly, again on the other, again back ({})",
                    key_name(k.ruler_zoom)
                ))
            } else {
                b.on_disabled_hover_text(format!("draw the ruler first ({})", key_name(k.ruler)))
            };
            zoom_end = b.clicked();
        });
        if zoom_end {
            let c = self.rt.canvas;
            self.ruler_zoom_key(c);
        }
        if let Some(h) = self.ruler_hint() {
            ui.colored_label(Color32::from_rgb(0, 140, 255), h);
        }
        match &self.scale {
            Some(s) => {
                ui.label(format!(
                    "Scale set on page {}: {:.4} mm per point ({:.2}:1){}",
                    s.page + 1,
                    s.mm_per_point,
                    MM_PER_POINT / s.mm_per_point,
                    s.note
                        .as_ref()
                        .map_or(String::new(), |n| format!(", from the page's \"{n}\""))
                ));
            }
            None => {
                ui.label(
                    RichText::new("Not set: the PDF's own scale (1:1), which drawings rarely are")
                        .small(),
                );
            }
        }
        // The page's own scale note, when it has one
        let note = self
            .rt
            .pdf
            .as_ref()
            .and_then(|p| p.texts.get(self.page)?.as_deref())
            .and_then(scale_note);
        if let Some((text, factor)) = note {
            let b = ui
                .button(format!("Use the page's \"{text}\""))
                .on_hover_text(
                    "The drawing says its scale; check it with the ruler on a dimension",
                );
            if b.clicked() {
                self.scale = Some(Scale {
                    mm_per_point: MM_PER_POINT / factor,
                    page: self.page,
                    mm: 0.0,
                    note: Some(text),
                });
                self.scale_set();
                self.usage("scale page-note");
            }
        }
        if self.tool.measures() {
            ui.label(
                RichText::new(format!(
                    "Measuring: the scale's ruler waits in the Scale mode ({})",
                    key_name(k.ruler)
                ))
                .small()
                .weak(),
            );
            return;
        }
        let Some([a, b]) = self.ruler else {
            ui.label(
                RichText::new("Ruler: drag along a dimension whose length the drawing gives")
                    .small()
                    .weak(),
            );
            return;
        };
        let pts = ((b[0] - a[0]) as f64).hypot((b[1] - a[1]) as f64);
        ui.label(format!(
            "Ruler: {:.3} mm at this scale",
            pts * self.mm_per_point()
        ));
        let mut set = false;
        ui.horizontal(|ui| {
            ui.label("It is");
            let r = ui.add(
                TextEdit::singleline(&mut self.rt.ruler_mm)
                    .desired_width(60.0)
                    .id_salt("ruler mm"),
            );
            ui.label("mm");
            self.rt.typing_mm = r.has_focus() || self.rt.length_box;
            set = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let ok = self
                .rt
                .ruler_mm
                .trim()
                .replace(',', ".")
                .parse::<f64>()
                .is_ok_and(|v| v > 0.0)
                && pts > 0.0;
            let b = ui.add_enabled(ok, Button::new("Set scale"));
            let b = if ok {
                b
            } else {
                b.on_disabled_hover_text("type the dimension's length in mm")
            };
            set |= b.clicked();
        });
        if set {
            self.set_scale_from_ruler();
        }
    }

    fn overlay_controls(&mut self, ui: &mut Ui) {
        let mut opacity = self.opacity.unwrap_or(self.rt.settings.opacity);
        ui.horizontal(|ui| {
            ui.label("Opacity");
            if ui.add(egui::Slider::new(&mut opacity, 0.0..=1.0)).changed() {
                self.opacity = Some(opacity);
            }
        });
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut self.blink, "Blink")
                .on_hover_text(format!(
                    "{} times a second (Settings); {}",
                    self.rt.settings.blink_hz,
                    key_name(self.keys().blink)
                ))
                .changed()
                && self.blink
            {
                self.usage("blink");
            }
            if ui
                .button("⟲ 90°")
                .on_hover_text(key_name(self.keys().rotate))
                .clicked()
            {
                self.place.rotation = (self.place.rotation + 90.0).rem_euclid(360.0);
            }
            ui.checkbox(&mut self.place.mirror, "Bottom view")
                .on_hover_text(format!(
                    "Mirrored, as seen from below; {}",
                    key_name(self.keys().mirror)
                ));
            let mut invert = self.page_inverted();
            if ui
                .checkbox(&mut invert, "Invert page")
                .on_hover_text(format!(
                    "White-ish on black; {}",
                    key_name(self.keys().invert)
                ))
                .changed()
            {
                self.invert = Some(invert);
                self.usage("invert");
            }
        });
        ui.horizontal(|ui| {
            ui.label("Angle");
            ui.add(
                DragValue::new(&mut self.place.rotation)
                    .speed(0.5)
                    .suffix("°")
                    .range(-360.0..=360.0),
            );
            if ui.button("Centre in view").clicked() {
                self.rt.center_footprint = true;
            }
        });
        // Blend mode: the modes other than normal need the GPU pass
        let gpu = blend::available(ui.ctx());
        let mut mode = self.blend_mode();
        ui.horizontal_wrapped(|ui| {
            ui.label("Blend");
            for m in BlendMode::ALL {
                let ok = m == BlendMode::Normal || gpu;
                let r = ui.add_enabled(ok, Button::selectable(mode == m, m.name()));
                let r = if ok {
                    r.on_hover_text(blend_hint(m))
                } else {
                    r.on_disabled_hover_text(
                        "needs the app's wgpu renderer, which this window does not have",
                    )
                };
                if r.clicked() {
                    mode = m;
                }
            }
        });
        if mode != self.blend_mode() {
            self.blend = Some(mode);
            self.usage(&format!("blend {}", mode.name().to_lowercase()));
        }
        // Copper, mask and paste: one view at a time, each in its own colour (Settings)
        ui.horizontal_wrapped(|ui| {
            for k in VIEWS {
                if ui
                    .add(Button::selectable(self.cu_shown == k, k.name()))
                    .clicked()
                {
                    self.cu_shown = k;
                }
            }
            ui.label(RichText::new(format!("({} cycles)", key_name(self.keys().cu_layer))).weak());
        });
        ui.horizontal_wrapped(|ui| {
            for name in std::iter::once(BACK).chain(Kind::OWN.map(Kind::name)) {
                let mut shown = self.group_shown(name);
                if ui.checkbox(&mut shown, name).changed() {
                    self.hidden.retain(|h| h != name);
                    if !shown {
                        self.hidden.push(name.to_string());
                    }
                }
            }
        });
    }

    fn settings_controls(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new("Settings")
            .id_salt("fp settings")
            .default_open(false)
            .show(ui, |ui| {
                let s = &mut self.rt.settings;
                Grid::new("fp settings grid").num_columns(2).show(ui, |ui| {
                    ui.label("Blink, times a second");
                    ui.add(DragValue::new(&mut s.blink_hz).range(0.2..=20.0).speed(0.1));
                    ui.end_row();
                    ui.label("Opacity at first");
                    ui.add(egui::Slider::new(&mut s.opacity, 0.0..=1.0));
                    ui.end_row();
                    ui.label("Finest drawing, px per point");
                    ui.add(
                        DragValue::new(&mut s.max_render_scale)
                            .range(1.0..=32.0)
                            .speed(0.1),
                    );
                    ui.end_row();
                    ui.label("Ruler end zoom, times the view");
                    ui.add(
                        DragValue::new(&mut s.ruler_zoom)
                            .range(1.0..=64.0)
                            .speed(0.1),
                    );
                    ui.end_row();
                    ui.label("Ruler snaps level");
                    ui.checkbox(&mut s.ruler_snap, "")
                        .on_hover_text("horizontal or vertical; Shift held: any angle");
                    ui.end_row();
                    ui.label("Ruler snaps to lines, px");
                    ui.add(
                        DragValue::new(&mut s.line_snap_px)
                            .range(0.0..=50.0)
                            .speed(0.2),
                    )
                    .on_hover_text(
                        "a line's end, two lines crossing, else onto the line, this near on screen; 0: never; \
                         Alt held: free",
                    );
                    ui.end_row();
                    ui.label("DXF curve tolerance, mm");
                    ui.add(
                        DragValue::new(&mut s.dxf_tolerance_mm)
                            .range(0.0001..=0.5)
                            .speed(0.001),
                    )
                    .on_hover_text(
                        "a curve of the drawing within this of a circle is an arc in the DXF, any other is made of \
                         straight pieces within it",
                    );
                    ui.end_row();
                });
                ui.label("Keys over the canvas, by name (F, Space, F5, …)");
                Grid::new("fp keys grid").num_columns(2).show(ui, |ui| {
                    let k = &mut s.keys;
                    for (what, name) in [
                        ("Page at full width", &mut k.full_width),
                        ("Move: the mouse moves the footprint", &mut k.move_tool),
                        ("Ruler: one end, then the other", &mut k.ruler),
                        ("Measure PDF", &mut k.measure_pdf),
                        ("Measure footprint", &mut k.measure_footprint),
                        ("Zoom on a ruler end, the other, back", &mut k.ruler_zoom),
                        ("Type the ruler's length", &mut k.scale),
                        ("The dimension's number sets the scale", &mut k.dimension),
                        ("Turn 90°", &mut k.rotate),
                        ("Bottom view", &mut k.mirror),
                        ("Blink", &mut k.blink),
                        ("Cycles copper/mask/paste", &mut k.cu_layer),
                        ("Invert the page", &mut k.invert),
                        ("What the SpaceMouse's puck moves", &mut k.puck),
                        ("Accept", &mut k.accept),
                        ("Next to review", &mut k.next),
                        ("Write a review note", &mut k.note),
                        ("Select the drawing's lines", &mut k.select),
                        ("Save the selection as DXF", &mut k.save_dxf),
                    ] {
                        ui.label(what);
                        let ok = egui::Key::from_name(name.trim()).is_some();
                        let mut edit = TextEdit::singleline(name).desired_width(70.0);
                        if !ok {
                            edit = edit.text_color(Color32::RED);
                        }
                        let r = ui.add(edit);
                        if !ok {
                            r.on_hover_text("not a key name: this step has no key");
                        }
                        ui.end_row();
                    }
                });
                ui.label("Page search words, best first, one per line");
                if ui
                    .add(TextEdit::multiline(&mut self.rt.words_text).desired_rows(4))
                    .changed()
                {
                    s.page_search_words = lines(&self.rt.words_text);
                    self.rt.page_hits = None;
                }
                ui.label("Font folders (Liberation fonts), one per line");
                if ui
                    .add(TextEdit::multiline(&mut self.rt.fonts_text).desired_rows(2))
                    .changed()
                {
                    s.font_dirs = lines(&self.rt.fonts_text)
                        .into_iter()
                        .map(PathBuf::from)
                        .collect();
                }
                ui.label("Library search command (search words are added)");
                if ui
                    .add(TextEdit::singleline(&mut self.rt.search_text))
                    .changed()
                {
                    s.library_search = self
                        .rt
                        .search_text
                        .split_whitespace()
                        .map(String::from)
                        .collect();
                }
                ui.label("Library name command, for the notes (the datasheet's file is added)");
                if ui.add(TextEdit::singleline(&mut self.rt.name_text)).changed() {
                    s.library_name = self
                        .rt
                        .name_text
                        .split_whitespace()
                        .map(String::from)
                        .collect();
                    self.rt.doc_name = None;
                }
                ui.label("Review notes file: a path, or a name beside the project or library");
                ui.add(TextEdit::singleline(&mut s.notes_file));
                let file = self.rt.settings_file.clone();
                ui.horizontal(|ui| {
                    let b = ui.add_enabled(file.is_some(), Button::new("Save"));
                    let b = match &file {
                        Some(f) => b.on_hover_text(f.display().to_string()),
                        None => b.on_disabled_hover_text(
                            "neither BERC_CONFIG, XDG_CONFIG_HOME nor HOME is set",
                        ),
                    };
                    if b.clicked()
                        && let Some(f) = &file
                    {
                        let (mut all, _) = Settings::load_from(f);
                        all.footprint_review = self.rt.settings.clone();
                        self.rt.settings_note = Some(match all.save_to(f) {
                            Ok(()) => format!("saved to {}", f.display()),
                            Err(e) => e,
                        });
                        self.usage("settings-save");
                    }
                    if ui.button("Defaults").clicked() {
                        self.set_settings(FootprintReviewSettings::default());
                        self.rt.page_hits = None;
                    }
                });
                if let Some(n) = &self.rt.settings_note {
                    ui.label(RichText::new(n).small());
                }
            });
    }

    /// Page points to screen
    fn to_screen(&self, canvas: Rect, p: [f32; 2]) -> Pos2 {
        canvas.min + vec2(self.pan[0], self.pan[1]) + vec2(p[0], p[1]) * self.zoom
    }

    fn to_page(&self, canvas: Rect, s: Pos2) -> [f32; 2] {
        let v = (s - canvas.min - vec2(self.pan[0], self.pan[1])) / self.zoom;
        [v.x, v.y]
    }

    /// Footprint millimetres to page points
    fn fp_to_page(&self, p: outline::P) -> [f32; 2] {
        let p = if self.place.mirror { [-p[0], p[1]] } else { p };
        let r = outline::rotate(p, self.place.rotation as f64);
        let k = 1.0 / self.mm_per_point();
        [
            self.place.origin[0] + (r[0] * k) as f32,
            self.place.origin[1] + (r[1] * k) as f32,
        ]
    }

    /// The modes above the canvas, the one on highlighted, each with its key: what the mouse does is never a hidden
    /// state. A mode's button or key again goes back to Move, as do the Move key and Esc.
    fn mode_bar(&mut self, ui: &mut Ui) {
        let k = self.keys();
        let pdf = self.rt.pdf.as_ref().is_some_and(|p| p.page_count() > 0);
        let fp = !self.rt.items.is_empty();
        let mut pick = None;
        ui.horizontal(|ui| {
            ui.label("Mode");
            for (t, key, why) in [
                (Tool::Move, k.move_tool, None),
                (
                    Tool::Scale,
                    k.ruler,
                    (!pdf).then_some("open the datasheet first: the ruler lies on its drawing"),
                ),
                (
                    Tool::MeasurePdf,
                    k.measure_pdf,
                    (!pdf).then_some("open the datasheet first: the ruler lies on its drawing"),
                ),
                (
                    Tool::MeasureFootprint,
                    k.measure_footprint,
                    (!fp).then_some("no footprint shown: pick a symbol"),
                ),
                (
                    Tool::Select,
                    k.select,
                    (!pdf).then_some("open the datasheet first: the pieces are its drawing's"),
                ),
            ] {
                let text = format!("{} · {}", t.name(), key_name(key));
                let r = ui.add_enabled(
                    why.is_none() || self.tool == t,
                    Button::selectable(self.tool == t, text),
                );
                let r = match why {
                    Some(why) => r.on_disabled_hover_text(why),
                    None => r.on_hover_text(t.what()),
                };
                if r.clicked() {
                    pick = Some(t);
                }
            }
            if self.tool != Tool::Move {
                ui.label(
                    RichText::new(format!("{} or Esc: back to Move", key_name(k.move_tool))).weak(),
                );
            }
        });
        if let Some(t) = pick {
            self.tool_key(t);
        }
        if self.tool == Tool::Select || !self.selection().is_empty() {
            self.dxf_bar(ui);
        }
    }

    /// The review note above the canvas: the text, measurements put in by the measuring rulers; Enter or Add
    /// appends it to the notes file, named beside it, with where the last one went
    fn note_bar(&mut self, ui: &mut Ui) {
        let k = self.keys();
        let file = self.notes_file();
        let mut add = false;
        ui.horizontal(|ui| {
            ui.label("Note");
            let r = ui.add(
                TextEdit::singleline(&mut self.rt.note)
                    .id_salt("review note")
                    .desired_width((ui.available_width() - 320.0).max(160.0))
                    .hint_text(format!(
                        "{} here; the measuring rulers add M1, M2, …; Enter adds it to the notes file",
                        key_name(k.note)
                    )),
            );
            if std::mem::take(&mut self.rt.focus_note) && !r.has_focus() {
                r.request_focus();
            }
            add = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let why = self.note_blocker();
            let b = ui.add_enabled(why.is_none(), Button::new("Add (Enter)"));
            let b = match &why {
                Some(w) => b.on_disabled_hover_text(w),
                None => b.on_hover_text(format!(
                    "appends the note with the symbol, the footprint, the datasheet and its page to {}",
                    file.as_ref().map_or(String::new(), |f| f.display().to_string())
                )),
            };
            add |= b.clicked();
            match (&self.rt.note_written, &file) {
                (Some(Err(e)), _) => {
                    ui.colored_label(Color32::RED, e);
                }
                (Some(Ok(f)), _) => {
                    ui.label(format!("added to {}", file_name(f)))
                        .on_hover_text(f.display().to_string());
                }
                (None, Some(f)) => {
                    ui.label(RichText::new(format!("→ {}", file_name(f))).weak())
                        .on_hover_text(f.display().to_string());
                }
                (None, None) => {}
            }
        });
        if add {
            self.add_note();
        }
    }

    /// Why a note can't be added now, when it can't
    fn note_blocker(&self) -> Option<String> {
        if self.rt.note.trim().is_empty() {
            return Some("write the note first".into());
        }
        if self.rt.sourced.is_none() {
            return Some("no footprint shown: pick a symbol first".into());
        }
        if self.notes_file().is_none() {
            return Some(format!(
                "no project or library to put \"{}\" beside: give the notes file a path of its own in Settings",
                self.rt.settings.notes_file
            ));
        }
        None
    }

    /// The folder of the project or library under review, where the notes file goes by default
    fn notes_dir(&self) -> Option<PathBuf> {
        let parent = |p: &Path| p.parent().map(Path::to_path_buf);
        if let Some(t) = &self.rt.target {
            return match &t.from {
                SymbolsFrom::Library { library, project } => {
                    let l = Path::new(library);
                    if l.exists() {
                        parent(l)
                    } else {
                        Some(project.clone())
                    }
                }
                from @ SymbolsFrom::Project(_) => Some(from.dir()),
            };
        }
        match &self.rt.sourced.as_ref()?.source {
            FootprintSource::File(p) => parent(p),
            FootprintSource::Board { board, .. } => parent(board),
            FootprintSource::Library { project, .. } => Some(project.clone()),
        }
    }

    /// The review notes file ([`notes::file`])
    fn notes_file(&self) -> Option<PathBuf> {
        notes::file(&self.rt.settings.notes_file, self.notes_dir().as_deref())
    }

    /// How the library names the datasheet open, asked once per file: as a command that opens it, else the file
    fn datasheet_name(&mut self) -> Option<String> {
        let path = self.datasheet.clone()?;
        if self.rt.doc_name.as_ref().is_none_or(|(p, _)| *p != path) {
            let cmd = self.rt.settings.library_name.clone();
            let name = match &self.rt.namer {
                Some(n) => n(&path),
                None if cmd.is_empty() => Ok(None),
                None => notes::run_library_name(&cmd, &path),
            };
            self.rt.doc_name = Some((path.clone(), name));
        }
        Some(match &self.rt.doc_name {
            Some((_, Ok(Some(n)))) => n.clone(),
            _ => format!("`{}`", path.display()),
        })
    }

    /// The footprint shown as the symbol's Footprint field names it; a footprint alone by its own name
    fn footprint_name(&self) -> Option<String> {
        let s = self.rt.sourced.as_ref()?;
        let field = self.rt.target.as_ref().and_then(|t| {
            let rows = &self.rt.symbols.as_ref()?.rows;
            rows.iter()
                .find(|r| r.symbol == t.symbol)?
                .footprint
                .clone()
        });
        Some(field.unwrap_or_else(|| match &s.source {
            FootprintSource::Library { lib_id, .. } => lib_id.clone(),
            _ => s.lib_id().to_string(),
        }))
    }

    /// The note into the notes file with what it is about; the field emptied for the next, whose measurements
    /// count from M1 again
    fn add_note(&mut self) {
        if self.note_blocker().is_some() {
            return;
        }
        let (Some(file), Some(footprint)) = (self.notes_file(), self.footprint_name()) else {
            return;
        };
        let datasheet = self.datasheet_name();
        let when = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        let note = notes::Note {
            symbol: self.rt.target.as_ref().map(|t| t.symbol.clone()),
            footprint,
            page: datasheet.as_ref().map(|_| self.page + 1),
            datasheet,
            when,
            text: self.rt.note.clone(),
        };
        let written = notes::append(&file, &note).map(|()| file);
        if written.is_ok() {
            self.rt.note.clear();
            self.rt.measure_count = 0;
            // The ruler shown is in no note now: moved, it is a new measurement
            if let Some(m) = &mut self.rt.measured {
                m.text.clear();
            }
            self.usage("note");
        }
        self.rt.note_written = Some(written);
    }

    /// The drawing's pieces on the page shown
    fn page_prims(&self) -> &[prims::Prim] {
        self.rt
            .pdf
            .as_ref()
            .and_then(|p| p.geometry(self.page))
            .map_or(&[], |g| g.prims.as_slice())
    }

    /// The pieces selected on the page shown
    fn selection(&self) -> &[usize] {
        if self.rt.selected_page == self.page {
            &self.rt.selected
        } else {
            &[]
        }
    }

    /// Nothing selected, as on a datasheet just opened
    fn clear_selection(&mut self) {
        self.rt.selected.clear();
        self.rt.select_hover = None;
        self.rt.select_box = None;
        self.rt.select_box_end = None;
    }

    /// How near (page points) a click picks a piece: the line snap's pixels, at least 6, at the view's zoom
    fn select_reach(&self) -> f32 {
        self.rt.settings.line_snap_px.max(6.0) / self.zoom
    }

    /// A click in the Select mode at `p` (page points): the piece there in or out of the selection; on nothing,
    /// the selection dropped unless `add` (Shift held)
    fn select_click(&mut self, p: [f32; 2], add: bool) {
        if self.rt.selected_page != self.page {
            self.clear_selection();
            self.rt.selected_page = self.page;
        }
        match prims::pick(self.page_prims(), p, self.select_reach()) {
            Some(i) => match self.rt.selected.binary_search(&i) {
                Ok(at) => {
                    self.rt.selected.remove(at);
                }
                Err(at) => self.rt.selected.insert(at, i),
            },
            None if !add => self.rt.selected.clear(),
            None => {}
        }
        self.usage("select click");
    }

    /// A box from `a` to `b` (page points) let go: the pieces wholly inside it are the selection, or are added to
    /// it with `add` (Shift held)
    fn select_in_box(&mut self, a: [f32; 2], b: [f32; 2], add: bool) {
        if self.rt.selected_page != self.page || !add {
            self.rt.selected.clear();
            self.rt.selected_page = self.page;
        }
        let mut inside = prims::in_box(self.page_prims(), a, b);
        inside.extend_from_slice(&self.rt.selected);
        inside.sort_unstable();
        inside.dedup();
        self.rt.selected = inside;
        self.usage("select box");
    }

    /// What the Select mode waits for
    fn select_hint(&self) -> String {
        let k = self.keys();
        let n = self.selection().len();
        if self.rt.select_box.is_some() {
            return "Select: let go to pick what is wholly inside the box (Shift: added to the selection)".into();
        }
        let save = match self.dxf_blocker() {
            None => format!("{} saves the {n} as DXF", key_name(k.save_dxf)),
            Some(why) => format!("Save: {why}"),
        };
        format!(
            "Select: click a line, arc or shape (again: drops it) · drag a box around some · Shift adds · {n} \
             selected · {save}"
        )
    }

    /// Why the selection can't be saved as DXF now, when it can't
    fn dxf_blocker(&self) -> Option<String> {
        let k = self.keys();
        if self.selection().is_empty() {
            return Some(format!(
                "select pieces of the drawing first (Select, {})",
                key_name(k.select)
            ));
        }
        if self.rt.sourced.is_none() {
            return Some("no footprint shown: pick a symbol first, the DXF is in its frame".into());
        }
        match &self.scale {
            None => {
                return Some(format!(
                    "set the drawing's scale first (Scale, {}): the DXF is in mm",
                    key_name(k.ruler)
                ));
            }
            Some(s) if s.page != self.page => {
                return Some(format!(
                    "the scale was set on page {}: set it again on this page, the DXF is in mm",
                    s.page + 1
                ));
            }
            Some(_) => {}
        }
        if self.notes_file().is_none() {
            return Some(format!(
                "no project or library to put the DXF beside: give the notes file \"{}\" a path of its own in \
                 Settings",
                self.rt.settings.notes_file
            ));
        }
        None
    }

    /// The next DXF's file: beside the notes file, `<symbol>-p<page>-<n>.dxf`, the first n not taken
    fn dxf_file(&self) -> Option<PathBuf> {
        let dir = self.notes_file()?.parent()?.to_path_buf();
        let name = match (&self.rt.target, &self.rt.sourced) {
            (Some(t), _) => t.symbol.clone(),
            (None, Some(s)) => s.lib_id().to_string(),
            (None, None) => return None,
        };
        let stem: String = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '+') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        (1..)
            .map(|n| dir.join(format!("{stem}-p{}-{n}.dxf", self.page + 1)))
            .find(|f| !f.exists())
    }

    /// The selection as a DXF in the footprint's mm (through the scale and the footprint's placement) beside the
    /// notes file; its name goes into the note
    fn save_dxf(&mut self) {
        if self.dxf_blocker().is_some() {
            return;
        }
        let Some(file) = self.dxf_file() else { return };
        let to = dxf::ToFootprint {
            origin: [self.place.origin[0] as f64, self.place.origin[1] as f64],
            rotation: self.place.rotation as f64,
            mirror: self.place.mirror,
            mm_per_point: self.mm_per_point(),
        };
        let all = self.page_prims();
        let picked: Vec<&prims::Prim> = self
            .selection()
            .iter()
            .filter_map(|&i| all.get(i))
            .collect();
        let entities = dxf::entities(&picked, &to, self.rt.settings.dxf_tolerance_mm);
        let footprint = self.footprint_name().unwrap_or_default();
        let mut comments = vec![format!(
            "better_erc footprint review: {} pieces of the datasheet drawing",
            picked.len()
        )];
        if let Some(t) = &self.rt.target {
            comments.push(format!("symbol {}", t.symbol));
        }
        comments.push(format!("footprint {footprint}"));
        if let Some(d) = &self.datasheet {
            comments.push(format!(
                "datasheet {}, page {}",
                file_name(d),
                self.page + 1
            ));
        }
        comments.push(format!(
            "mm in the footprint's frame, its origin at 0,0, Y up (KiCad: Import Graphics at 0,0); {:.6} mm per PDF \
             point",
            to.mm_per_point
        ));
        let text = dxf::write(&entities, &comments);
        let written = std::fs::write(&file, text)
            .map(|()| file.clone())
            .map_err(|e| format!("can't write {}: {e}", file.display()));
        if written.is_ok() {
            let name = file_name(&file);
            let note = self.rt.note.trim_end().to_string();
            let add = format!("DXF `{name}`");
            self.rt.note = if note.is_empty() {
                format!("{add} ")
            } else {
                format!("{note} {add} ")
            };
            self.usage("dxf");
        }
        self.rt.dxf_written = Some(written);
    }

    /// Under the mode bar while selecting or with a selection: how many, Save and Clear, and where the last DXF
    /// went
    fn dxf_bar(&mut self, ui: &mut Ui) {
        let k = self.keys();
        let n = self.selection().len();
        let (mut save, mut clear) = (false, false);
        ui.horizontal(|ui| {
            ui.label(format!("{n} selected"));
            let why = self.dxf_blocker();
            let b = ui.add_enabled(
                why.is_none(),
                Button::new(format!("Save DXF · {}", key_name(k.save_dxf))),
            );
            let b = match &why {
                Some(w) => b.on_disabled_hover_text(w),
                None => b.on_hover_text(format!(
                    "the selection in the footprint's mm, its origin at 0,0, as {}; its name goes into the note",
                    self.dxf_file()
                        .map_or(String::new(), |f| f.display().to_string())
                )),
            };
            save = b.clicked();
            clear = ui
                .add_enabled(n > 0, Button::new("Clear"))
                .on_disabled_hover_text("nothing selected")
                .clicked();
            match &self.rt.dxf_written {
                Some(Ok(f)) => {
                    ui.label(format!("saved {}", file_name(f)))
                        .on_hover_text(f.display().to_string());
                }
                Some(Err(e)) => {
                    ui.colored_label(Color32::RED, e);
                }
                None => {}
            }
        });
        if save {
            self.save_dxf();
        }
        if clear {
            self.clear_selection();
        }
    }

    /// The selection over the page, and the piece a click would pick
    fn draw_selection(&self, painter: &egui::Painter, canvas: Rect) {
        let all = self.page_prims();
        let tol = prims::screen_tolerance(self.zoom);
        let draw = |i: usize, stroke: Stroke| {
            if let Some(p) = all.get(i) {
                let pts: Vec<Pos2> = p
                    .polyline(tol)
                    .into_iter()
                    .map(|q| self.to_screen(canvas, q))
                    .collect();
                painter.add(Shape::line(pts, stroke));
            }
        };
        for &i in self.selection() {
            draw(i, Stroke::new(2.5, SELECT_COLOUR));
        }
        if self.tool == Tool::Select
            && let Some(i) = self.rt.select_hover
        {
            draw(i, Stroke::new(2.0, SELECT_COLOUR.gamma_multiply(0.5)));
        }
        if let (Some(a), Some(b)) = (self.rt.select_box, self.rt.select_box_end) {
            let r = Rect::from_two_pos(self.to_screen(canvas, a), self.to_screen(canvas, b));
            painter.rect_filled(r, 0.0, SELECT_COLOUR.gamma_multiply(0.1));
            painter.rect_stroke(
                r,
                0.0,
                Stroke::new(1.0, SELECT_COLOUR),
                egui::StrokeKind::Inside,
            );
        }
    }

    fn canvas(&mut self, ui: &mut Ui) {
        self.mode_bar(ui);
        self.note_bar(ui);
        let (canvas, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        self.rt.canvas = canvas;
        let painter = ui.painter_at(canvas);
        painter.rect_filled(canvas, 0.0, CANVAS_BACKGROUND);
        let ctx = ui.ctx().clone();
        let page_size = self.rt.pdf.as_ref().and_then(|p| p.page_size(self.page));
        if self.rt.fit_page
            && let Some([w, h]) = page_size
            && canvas.width() > 1.0
        {
            self.zoom = (canvas.width() / w).min(canvas.height() / h) * 0.98;
            self.pan = [
                (canvas.width() - w * self.zoom) / 2.0,
                (canvas.height() - h * self.zoom) / 2.0,
            ];
            self.rt.fit_page = false;
        }
        if self.rt.fit_width
            && let Some([w, _]) = page_size
            && canvas.width() > 1.0
        {
            // Full width, the page point in the middle of the view staying at the same height
            let mid = self.to_page(canvas, canvas.center());
            self.zoom = canvas.width() / w;
            self.pan = [0.0, canvas.height() / 2.0 - mid[1] * self.zoom];
            self.rt.fit_width = false;
            self.rt.overview = None;
        }
        if self.rt.center_footprint && canvas.width() > 1.0 {
            // The middle of the footprint's extent to the middle of the view
            let target = self.to_page(canvas, canvas.center());
            self.place.origin = target;
            if let Some((lo, hi)) = outline::extent(&self.rt.items) {
                let mid = self.fp_to_page([(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0]);
                self.place.origin = [2.0 * target[0] - mid[0], 2.0 * target[1] - mid[1]];
            }
            self.rt.center_footprint = false;
        }
        // Zoom at the pointer, pan with the secondary or middle button
        if resp.hovered() {
            let (scroll, zoom_delta, pointer) = ui.input(|i| {
                (
                    i.smooth_scroll_delta.y,
                    i.zoom_delta(),
                    i.pointer.hover_pos(),
                )
            });
            let factor = zoom_delta * (scroll / 400.0).exp();
            if factor != 1.0
                && let Some(at) = pointer
            {
                let before = self.to_page(canvas, at);
                self.zoom = (self.zoom * factor).clamp(0.05, 400.0);
                let after = self.to_screen(canvas, before);
                self.pan[0] += at.x - after.x;
                self.pan[1] += at.y - after.y;
            }
        }
        if resp.dragged_by(egui::PointerButton::Secondary)
            || resp.dragged_by(egui::PointerButton::Middle)
        {
            let d = resp.drag_delta();
            self.pan[0] += d.x;
            self.pan[1] += d.y;
        }
        // The SpaceMouse's puck
        self.puck_nav(&ctx, canvas, resp.hover_pos());
        // The page, inverted (white-ish on black) when asked: the picture's colours, not the footprint's
        let invert = self.page_inverted();
        if let (Some(pdf), Some([w, h])) = (&mut self.rt.pdf, page_size) {
            let ppp = ctx.pixels_per_point();
            let want = (self.zoom * ppp)
                .max(0.5)
                .log2()
                .ceil()
                .exp2()
                .min(self.rt.settings.max_render_scale);
            pdf.want(&ctx, self.page, want);
            let rect = Rect::from_min_size(
                canvas.min + vec2(self.pan[0], self.pan[1]),
                vec2(w, h) * self.zoom,
            );
            let inverted_tex: Option<egui::TextureId> = invert
                .then(|| pdf.inverted(&ctx).map(|(_, id, _)| id))
                .flatten();
            match &pdf.picture {
                Some(pic) if pic.page == self.page => {
                    let tex = inverted_tex.unwrap_or(pic.texture.id());
                    painter.image(
                        tex,
                        rect,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                _ => {
                    let (bg, fg) = if invert {
                        (Color32::from_gray(15), Color32::DARK_GRAY)
                    } else {
                        (Color32::WHITE, Color32::GRAY)
                    };
                    painter.rect_filled(rect, 0.0, bg);
                    painter.text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "drawing the page…",
                        FontId::proportional(14.0),
                        fg,
                    );
                }
            }
        } else {
            painter.text(
                canvas.center() - vec2(0.0, 40.0),
                Align2::CENTER_CENTER,
                if self.rt.sourced.is_some() {
                    "Open its datasheet (left): the Datasheet field's file, the library search or a PDF"
                } else {
                    "Pick a symbol (left): its footprint and datasheet follow"
                },
                FontId::proportional(16.0),
                Color32::LIGHT_GRAY,
            );
        }
        // Primary drag: the footprint or the ruler
        let pointer_page = resp.interact_pointer_pos().map(|p| self.to_page(canvas, p));
        let hover_page = resp.hover_pos().map(|p| self.to_page(canvas, p));
        // The ruler key's other end follows the pointer; a click leaves it there
        // Shift: the ruler at any angle, not snapped level; Alt: free of the drawing's lines
        let (free, lines) = ui.input(|i| (i.modifiers.shift, !i.modifiers.alt));
        self.rt.snap = None;
        if self.rt.ruler_placing
            && self.ruler.is_some()
            && let Some(p) = hover_page
        {
            self.move_ruler_end(1, p, free, lines);
            if resp.clicked() {
                self.rt.ruler_placing = false;
                self.rt.ruler_anchor = None;
            }
        }
        // An end zoomed in on goes where a click or a drag puts it
        if let (true, Some(i), true, Some(p)) = (
            self.tool.ruler(),
            self.rt.ruler_focus,
            self.ruler.is_some(),
            pointer_page,
        ) && (resp.clicked() || resp.dragged_by(egui::PointerButton::Primary))
        {
            self.move_ruler_end(i, p, free, lines);
        } else if resp.dragged_by(egui::PointerButton::Primary) {
            match self.tool {
                Tool::Move => {
                    let d = resp.drag_delta() / self.zoom;
                    self.place.origin[0] += d.x;
                    self.place.origin[1] += d.y;
                }
                Tool::Select => {
                    if resp.drag_started() {
                        let start = resp
                            .interact_pointer_pos()
                            .map(|s| s - resp.drag_delta())
                            .unwrap_or_default();
                        self.rt.select_box = Some(self.to_page(canvas, start));
                    }
                    self.rt.select_box_end = pointer_page;
                }
                Tool::Scale | Tool::MeasurePdf | Tool::MeasureFootprint => {
                    if let Some(p) = pointer_page {
                        if resp.drag_started() {
                            let start = resp
                                .interact_pointer_pos()
                                .map(|s| s - resp.drag_delta())
                                .unwrap_or_default();
                            let near = self.ruler.and_then(|r| {
                                (0..2).find(|i| {
                                    (self.to_screen(canvas, r[*i]) - start).length() < 8.0
                                })
                            });
                            match near {
                                Some(i) => self.rt.ruler_end = Some(i),
                                None => {
                                    let s = self.start_point(self.to_page(canvas, start), lines);
                                    self.ruler = Some([s, s]);
                                    self.rt.measured = None;
                                    self.rt.ruler_anchor = Some(s);
                                    self.rt.ruler_end = Some(1);
                                }
                            }
                        }
                        if let Some(i) = self.rt.ruler_end {
                            self.move_ruler_end(i, p, free, lines);
                        }
                    }
                }
            }
        }
        // The Select mode: a box let go selects what is in it, a click the piece at the pointer
        if self.tool == Tool::Select {
            if resp.drag_stopped()
                && let (Some(a), Some(b)) = (
                    self.rt.select_box.take(),
                    pointer_page.or(self.rt.select_box_end),
                )
            {
                self.rt.select_box_end = None;
                self.select_in_box(a, b, free);
            } else if resp.clicked()
                && let Some(p) = pointer_page
            {
                self.select_click(p, free);
            }
            self.rt.select_hover = hover_page
                .filter(|_| self.rt.select_box.is_none())
                .and_then(|p| prims::pick(self.page_prims(), p, self.select_reach()));
        } else {
            self.rt.select_box = None;
            self.rt.select_box_end = None;
            self.rt.select_hover = None;
        }
        if resp.drag_stopped() {
            self.rt.ruler_end = None;
            if !self.rt.ruler_placing {
                self.rt.ruler_anchor = None;
            }
        }
        // Before an end is placed: where it would snap, at the pointer
        if self.tool.ruler()
            && self.rt.snap.is_none()
            && !resp.dragged_by(egui::PointerButton::Primary)
            && lines
            && let Some(p) = hover_page
        {
            self.rt.snap = self.end_snap(p, false);
        }
        // A measuring ruler let go: its length into the note
        if !self.rt.ruler_placing
            && self.rt.ruler_end.is_none()
            && !resp.dragged_by(egui::PointerButton::Primary)
        {
            self.note_measurement();
        }
        // Keys while the pointer is over the canvas
        if resp.hovered() && !ctx.egui_wants_keyboard_input() {
            let km = self.keys();
            let (pressed, shift) = ui.input(|i| {
                let pressed = |k: Option<egui::Key>| k.is_some_and(|k| i.key_pressed(k));
                let p = Pressed {
                    left: pressed(Some(egui::Key::ArrowLeft)),
                    right: pressed(Some(egui::Key::ArrowRight)),
                    up: pressed(Some(egui::Key::ArrowUp)),
                    down: pressed(Some(egui::Key::ArrowDown)),
                    page_up: pressed(Some(egui::Key::PageUp)),
                    page_down: pressed(Some(egui::Key::PageDown)),
                    escape: pressed(Some(egui::Key::Escape)),
                    full_width: pressed(km.full_width),
                    move_tool: pressed(km.move_tool),
                    ruler: pressed(km.ruler),
                    measure_pdf: pressed(km.measure_pdf),
                    measure_footprint: pressed(km.measure_footprint),
                    ruler_zoom: pressed(km.ruler_zoom),
                    scale: pressed(km.scale),
                    dimension: pressed(km.dimension),
                    rotate: pressed(km.rotate),
                    mirror: pressed(km.mirror),
                    blink: pressed(km.blink),
                    cu_layer: pressed(km.cu_layer),
                    invert: pressed(km.invert),
                    puck: pressed(km.puck),
                    accept: pressed(km.accept),
                    next: pressed(km.next),
                    note: pressed(km.note),
                    select: pressed(km.select),
                    save_dxf: pressed(km.save_dxf),
                };
                (p, i.modifiers.shift)
            });
            let step = if shift { 10.0 } else { 1.0 } / self.zoom;
            let p = pressed;
            if p.left {
                self.place.origin[0] -= step;
            }
            if p.right {
                self.place.origin[0] += step;
            }
            if p.up {
                self.place.origin[1] -= step;
            }
            if p.down {
                self.place.origin[1] += step;
            }
            for (on, action) in [
                (p.page_up, Action::PageUp),
                (p.page_down, Action::PageDown),
                (p.rotate, Action::Rotate),
                (p.mirror, Action::Mirror),
                (p.blink, Action::Blink),
                (p.cu_layer, Action::CuLayer),
                (p.invert, Action::Invert),
                (p.full_width, Action::FullWidth),
                (p.escape, Action::EndRuler),
                (p.move_tool, Action::Mode(Tool::Move)),
                (p.ruler, Action::Ruler),
                (p.measure_pdf, Action::Mode(Tool::MeasurePdf)),
                (p.measure_footprint, Action::Mode(Tool::MeasureFootprint)),
                (p.ruler_zoom, Action::RulerZoom),
                (p.scale, Action::Scale),
                (p.dimension, Action::Dimension),
                (p.puck, Action::Puck),
                (p.accept, Action::Accept),
                (p.next, Action::NextToReview),
                (p.note, Action::Note),
                (p.select, Action::Mode(Tool::Select)),
                (p.save_dxf, Action::SaveDxf),
            ] {
                if on {
                    self.act(&ctx, action, hover_page, shift, lines);
                }
            }
        }
        // The footprint
        let shown = if self.blink {
            let hz = self.rt.settings.blink_hz.max(0.1) as f64;
            let t = ui.input(|i| i.time);
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(0.5 / hz));
            ((t * hz * 2.0).floor() as i64) % 2 == 0
        } else {
            true
        };
        if shown && !self.rt.items.is_empty() {
            let rect = page_size.map(|[w, h]| {
                Rect::from_min_size(
                    canvas.min + vec2(self.pan[0], self.pan[1]),
                    vec2(w, h) * self.zoom,
                )
            });
            let page = rect.zip(self.page_image(&ctx, invert));
            self.draw_footprint(&painter, canvas, page, ui.id());
        }
        self.draw_selection(&painter, canvas);
        // The ruler
        if let Some([a, b]) = self.ruler {
            let (sa, sb) = (self.to_screen(canvas, a), self.to_screen(canvas, b));
            let (line, fill) = ruler_colours(self.tool);
            let stroke = Stroke::new(1.5, line);
            painter.line_segment([sa, sb], stroke);
            let n = (sb - sa).normalized().rot90() * 6.0;
            for s in [sa, sb] {
                painter.line_segment([s - n, s + n], stroke);
                painter.circle_stroke(s, 3.0, stroke);
            }
            let (mm, dx, dy) = self.ruler_mm([a, b]);
            let label = if self.tool.measures() {
                let name = match &self.rt.measured {
                    Some(m) if m.ends == [a, b] => format!("M{} = ", m.n),
                    _ => String::new(),
                };
                format!("{name}{mm:.3} mm · dx {dx:.3} · dy {dy:.3}")
            } else {
                format!("{mm:.3} mm")
            };
            let at = sa + (sb - sa) / 2.0 + n * 2.0;
            let galley = painter.layout_no_wrap(label, FontId::monospace(13.0), Color32::WHITE);
            let r = Align2::CENTER_CENTER
                .anchor_size(at, galley.size())
                .expand(2.0);
            painter.rect_filled(r, 2.0, fill);
            painter.galley(r.min + vec2(2.0, 2.0), galley, Color32::WHITE);
            if !self.tool.measures() {
                self.dimension_chip(ui, &painter, canvas, r);
            }
        }
        self.draw_snap(&painter, canvas);
        if self.rt.length_box {
            self.length_box(&ctx, canvas, ui.id());
        }
        // The keys, under the canvas; the ruler step's hint above them
        let mut y = canvas.max.y - 4.0;
        for (text, bg) in [
            (Some(self.help_line()), Color32::from_black_alpha(170)),
            (self.ruler_hint(), Color32::from_rgb(0, 90, 180)),
        ] {
            let Some(text) = text else { continue };
            let galley = painter.layout_no_wrap(text, FontId::proportional(12.0), Color32::WHITE);
            let r = Rect::from_min_size(
                pos2(canvas.min.x + 4.0, y - galley.size().y - 4.0),
                galley.size() + vec2(8.0, 4.0),
            );
            painter.rect_filled(r, 3.0, bg);
            painter.galley(r.min + vec2(4.0, 2.0), galley, Color32::WHITE);
            y = r.min.y - 2.0;
        }
    }

    /// The ruler's length and its x and y spans, mm, at the scale set
    fn ruler_mm(&self, [a, b]: [[f32; 2]; 2]) -> (f64, f64, f64) {
        let k = self.mm_per_point();
        let (dx, dy) = ((b[0] - a[0]) as f64 * k, (b[1] - a[1]) as f64 * k);
        (dx.hypot(dy), dx.abs(), dy.abs())
    }

    /// A measuring ruler laid or moved, and let go: its length into the review note as `M1 = 1.234 mm`. A new
    /// ruler takes the next number; a ruler adjusted keeps its number, its value changed in the note (added again
    /// when the note no longer has it).
    fn note_measurement(&mut self) {
        let Some(ends) = self.ruler.filter(|_| self.tool.measures()) else {
            return;
        };
        let (mm, _, _) = self.ruler_mm(ends);
        if mm <= 0.0 || self.rt.measured.as_ref().is_some_and(|m| m.ends == ends) {
            return;
        }
        let n = match &self.rt.measured {
            Some(m) if !m.text.is_empty() => m.n,
            _ => {
                self.rt.measure_count += 1;
                self.rt.measure_count
            }
        };
        let text = format!("M{n} = {mm:.3} mm");
        let old = self
            .rt
            .measured
            .take()
            .map(|m| m.text)
            .filter(|t| !t.is_empty());
        match old.and_then(|o| self.rt.note.rfind(&o).map(|i| (i, o.len()))) {
            Some((i, len)) => self.rt.note.replace_range(i..i + len, &text),
            None => {
                let note = self.rt.note.trim_end().to_string();
                self.rt.note = if note.is_empty() {
                    format!("{text} ")
                } else {
                    format!("{note} {text} ")
                };
            }
        }
        self.rt.measured = Some(Measured { n, ends, text });
        self.usage("measure");
    }

    /// Where a ruler end snaps, at the pointer: the lines it snaps to drawn over, a square on a line's end, a cross
    /// where two lines cross, a ring on a line
    fn draw_snap(&self, painter: &egui::Painter, canvas: Rect) {
        let Some(snap) = &self.rt.snap else { return };
        let geometry = self.rt.pdf.as_ref().and_then(|p| p.geometry(self.page));
        let stroke = Stroke::new(1.5, SNAP_COLOUR);
        if let Some(g) = geometry {
            for s in snap.segs.iter().filter_map(|&i| g.segs.get(i)) {
                let w = (s.width * self.zoom).max(1.5);
                painter.line_segment(
                    [self.to_screen(canvas, s.a), self.to_screen(canvas, s.b)],
                    Stroke::new(w, SNAP_COLOUR.gamma_multiply(0.6)),
                );
            }
        }
        let at = self.to_screen(canvas, snap.at);
        match snap.kind {
            SnapKind::End => {
                painter.rect_stroke(
                    Rect::from_center_size(at, vec2(9.0, 9.0)),
                    0.0,
                    stroke,
                    egui::StrokeKind::Middle,
                );
            }
            SnapKind::Crossing => {
                for d in [vec2(5.0, 5.0), vec2(5.0, -5.0)] {
                    painter.line_segment([at - d, at + d], stroke);
                }
            }
            SnapKind::Line => {
                painter.circle_stroke(at, 4.5, stroke);
            }
            SnapKind::PadCentre => {
                painter.circle_stroke(at, 5.0, stroke);
                for d in [vec2(7.0, 0.0), vec2(0.0, 7.0)] {
                    painter.line_segment([at - d, at + d], stroke);
                }
            }
            SnapKind::PadSide => {
                let d = 5.0;
                painter.add(egui::Shape::closed_line(
                    vec![
                        at + vec2(0.0, -d),
                        at + vec2(d, 0.0),
                        at + vec2(0.0, d),
                        at + vec2(-d, 0.0),
                    ],
                    stroke,
                ));
            }
        }
    }

    /// The dimension's number between the ruler's ends, as a chip beside the ruler's own label (`label`, on
    /// screen) with its key; a click or the key sets the scale from it. Its box on the page is outlined.
    fn dimension_chip(&mut self, ui: &Ui, painter: &egui::Painter, canvas: Rect, label: Rect) {
        let Some(n) = self.dimension_number() else {
            return;
        };
        let word = Rect::from_two_pos(self.to_screen(canvas, n.min), self.to_screen(canvas, n.max))
            .expand(2.0);
        painter.rect_stroke(
            word,
            2.0,
            Stroke::new(1.0, SNAP_COLOUR),
            egui::StrokeKind::Outside,
        );
        let text = format!("{} mm · {}", n.text, key_name(self.keys().dimension));
        let galley = painter.layout_no_wrap(text.clone(), FontId::monospace(13.0), Color32::BLACK);
        let r = Align2::LEFT_CENTER
            .anchor_size(label.right_center() + vec2(6.0, 0.0), galley.size())
            .expand(2.0);
        let resp = ui
            .interact(r, ui.id().with("dimension chip"), Sense::click())
            .on_hover_text("the number the drawing prints for this dimension: sets the scale");
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &text));
        let bg = if resp.hovered() {
            Color32::from_rgb(255, 200, 80)
        } else {
            SNAP_COLOUR
        };
        painter.rect_filled(r, 2.0, bg);
        painter.galley(r.min + vec2(2.0, 2.0), galley, Color32::BLACK);
        if resp.clicked() {
            self.use_dimension_number();
        }
    }

    /// Whether a group with a checkbox (the back side, courtyard, fab, silk, board edge) is shown
    fn group_shown(&self, name: &str) -> bool {
        !self.hidden.iter().any(|h| h == name)
    }

    /// Whether `item` is drawn: copper, mask and paste by the view picked, the back side's also by its checkbox, the
    /// others by their checkbox; holes and pad numbers over every view
    fn item_shown(&self, item: &Item) -> bool {
        match (item, item.kind()) {
            (Item::Number { back, .. }, _) => !back || self.group_shown(BACK),
            (_, None) => true,
            (_, Some(k)) => match k.view() {
                Some(v) => v == self.cu_shown && (!k.back() || self.group_shown(BACK)),
                None => self.group_shown(k.name()),
            },
        }
    }

    /// What of the footprint is drawn: the items shown
    fn shown_items(&self) -> impl Iterator<Item = &Item> {
        self.rt.items.iter().filter(|i| self.item_shown(i))
    }

    /// Draws the footprint: by egui's painter in the normal mode, else by the blend pass over `page` (its rectangle
    /// on screen and its picture); holes, pad numbers and the origin by the painter on top
    fn draw_footprint(
        &self,
        painter: &egui::Painter,
        canvas: Rect,
        page: Option<(Rect, blend::PageImage)>,
        id: Id,
    ) {
        let a = self
            .opacity
            .unwrap_or(self.rt.settings.opacity)
            .clamp(0.0, 1.0);
        let alpha = |c: Color32| c.gamma_multiply(a);
        let colors = self.layer_colors();
        let colour = |k: Kind| {
            let [r, g, b] = layer_colour(colors, k).0;
            Color32::from_rgb(r, g, b)
        };
        let mode = self.blend_mode();
        let gpu = mode != BlendMode::Normal && blend::available(painter.ctx());
        let px_per_mm = (self.zoom as f64 / self.mm_per_point()) as f32;
        let screen = |p: outline::P| self.to_screen(canvas, self.fp_to_page(p));
        // The blend pass gets the shapes at full colour; the shader mixes them in by the opacity
        let mut blended: Vec<Shape> = Vec::new();
        for item in self.shown_items() {
            let c = item.kind().map_or(Color32::WHITE, colour);
            match item {
                Item::Area { points, convex, .. } => {
                    let pts: Vec<Pos2> = points.iter().map(|p| screen(*p)).collect();
                    let shape = match (*convex, gpu) {
                        (true, true) => Shape::convex_polygon(pts, c, Stroke::NONE),
                        (true, false) => Shape::convex_polygon(pts, alpha(c), Stroke::new(1.0, c)),
                        (false, _) => Shape::closed_line(pts, Stroke::new(1.5, c)),
                    };
                    if gpu {
                        blended.push(shape);
                    } else {
                        painter.add(shape);
                    }
                }
                Item::Line {
                    points,
                    closed,
                    width,
                    ..
                } => {
                    let pts: Vec<Pos2> = points.iter().map(|p| screen(*p)).collect();
                    // A mask or paste line: thick enough to read on the page
                    let least = if matches!(
                        item.kind().and_then(Kind::view),
                        Some(Kind::Mask | Kind::Paste)
                    ) {
                        2.0
                    } else {
                        1.0
                    };
                    let w = ((*width as f32) * px_per_mm).max(least);
                    let stroke = if gpu {
                        Stroke::new(w, c)
                    } else {
                        Stroke::new(w, alpha(c).to_opaque().gamma_multiply(a.max(0.6)))
                    };
                    let shape = if *closed {
                        Shape::closed_line(pts, stroke)
                    } else {
                        Shape::line(pts, stroke)
                    };
                    if gpu {
                        blended.push(shape);
                    } else {
                        painter.add(shape);
                    }
                }
                Item::Hole {
                    center,
                    size,
                    rotation,
                    ..
                } => {
                    let c0 = screen(*center);
                    let r = (size[0].min(size[1]) as f32) * px_per_mm / 2.0;
                    let stroke = Stroke::new(1.0, Color32::WHITE);
                    if (size[0] - size[1]).abs() < 1e-9 {
                        painter.circle(c0, r, Color32::from_black_alpha((a * 200.0) as u8), stroke);
                    } else {
                        // An oval drill along its long side, turned with the pad
                        let long = (size[0].max(size[1]) - size[0].min(size[1])) / 2.0;
                        let dir = if size[0] > size[1] {
                            [long, 0.0]
                        } else {
                            [0.0, long]
                        };
                        let d = outline::rotate(dir, *rotation);
                        let (p1, p2) = (
                            screen([center[0] - d[0], center[1] - d[1]]),
                            screen([center[0] + d[0], center[1] + d[1]]),
                        );
                        painter.line_segment(
                            [p1, p2],
                            Stroke::new(2.0 * r, Color32::from_black_alpha((a * 200.0) as u8)),
                        );
                    }
                }
                Item::Number { at, text, size, .. } => {
                    let h = (*size as f32 * px_per_mm * 0.5).min(18.0);
                    if h >= 7.0 {
                        painter.text(
                            screen(*at),
                            Align2::CENTER_CENTER,
                            text,
                            FontId::proportional(h),
                            Color32::WHITE,
                        );
                    }
                }
            }
        }
        if gpu && let Some((page_rect, image)) = page {
            let mut mesh = egui::Mesh::default();
            let mut tess = egui::epaint::Tessellator::new(
                painter.ctx().pixels_per_point(),
                egui::epaint::TessellationOptions::default(),
                [1, 1],
                Vec::new(),
            );
            for shape in blended {
                tess.tessellate_shape(shape, &mut mesh);
            }
            painter.add(eframe::egui_wgpu::Callback::new_paint_callback(
                canvas,
                blend::BlendPass {
                    id,
                    mesh,
                    page: image,
                    page_rect,
                    around: CANVAS_BACKGROUND,
                    opacity: a,
                    mode,
                    viewport: canvas,
                },
            ));
        }
        // The origin
        let o = self.to_screen(canvas, self.place.origin);
        let s = Stroke::new(1.0, Color32::from_rgb(0, 160, 0));
        painter.line_segment([o - vec2(6.0, 0.0), o + vec2(6.0, 0.0)], s);
        painter.line_segment([o - vec2(0.0, 6.0), o + vec2(0.0, 6.0)], s);
    }
}

/// The canvas' keys, resolved from the settings' key names
#[derive(Clone, Copy)]
struct KeyMap {
    full_width: Option<egui::Key>,
    move_tool: Option<egui::Key>,
    ruler: Option<egui::Key>,
    measure_pdf: Option<egui::Key>,
    measure_footprint: Option<egui::Key>,
    ruler_zoom: Option<egui::Key>,
    scale: Option<egui::Key>,
    dimension: Option<egui::Key>,
    rotate: Option<egui::Key>,
    mirror: Option<egui::Key>,
    blink: Option<egui::Key>,
    cu_layer: Option<egui::Key>,
    invert: Option<egui::Key>,
    puck: Option<egui::Key>,
    accept: Option<egui::Key>,
    next: Option<egui::Key>,
    note: Option<egui::Key>,
    select: Option<egui::Key>,
    save_dxf: Option<egui::Key>,
}

impl From<&Keys> for KeyMap {
    fn from(k: &Keys) -> Self {
        let key = |name: &str| egui::Key::from_name(name.trim());
        KeyMap {
            full_width: key(&k.full_width),
            move_tool: key(&k.move_tool),
            ruler: key(&k.ruler),
            measure_pdf: key(&k.measure_pdf),
            measure_footprint: key(&k.measure_footprint),
            ruler_zoom: key(&k.ruler_zoom),
            scale: key(&k.scale),
            dimension: key(&k.dimension),
            rotate: key(&k.rotate),
            mirror: key(&k.mirror),
            blink: key(&k.blink),
            cu_layer: key(&k.cu_layer),
            invert: key(&k.invert),
            puck: key(&k.puck),
            accept: key(&k.accept),
            next: key(&k.next),
            note: key(&k.note),
            select: key(&k.select),
            save_dxf: key(&k.save_dxf),
        }
    }
}

/// The keys pressed this frame
struct Pressed {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    page_up: bool,
    page_down: bool,
    escape: bool,
    full_width: bool,
    move_tool: bool,
    ruler: bool,
    measure_pdf: bool,
    measure_footprint: bool,
    ruler_zoom: bool,
    scale: bool,
    dimension: bool,
    rotate: bool,
    mirror: bool,
    blink: bool,
    cu_layer: bool,
    invert: bool,
    puck: bool,
    accept: bool,
    next: bool,
    note: bool,
    select: bool,
    save_dxf: bool,
}

/// The ruler's line and label colours in a mode: blue setting the scale, orange measuring the drawing, green
/// measuring the footprint
fn ruler_colours(t: Tool) -> (Color32, Color32) {
    match t {
        Tool::Move | Tool::Scale => (
            Color32::from_rgb(0, 140, 255),
            Color32::from_rgb(0, 90, 180),
        ),
        Tool::MeasurePdf => (
            Color32::from_rgb(255, 140, 0),
            Color32::from_rgb(190, 90, 0),
        ),
        Tool::MeasureFootprint => (
            Color32::from_rgb(40, 200, 90),
            Color32::from_rgb(20, 130, 60),
        ),
        // No ruler of its own: the scale's is shown
        Tool::Select => ruler_colours(Tool::Scale),
    }
}

/// A key as the help shows it; a setting that names no key shows so
fn key_name(k: Option<egui::Key>) -> String {
    k.map_or_else(|| "(no key: see Settings)".into(), |k| k.name().to_string())
}

fn status_colour(s: SymbolStatus) -> Color32 {
    match s {
        SymbolStatus::Ok => Color32::from_rgb(0, 170, 60),
        SymbolStatus::Changed | SymbolStatus::Malformed | SymbolStatus::Missing => {
            Color32::from_rgb(220, 50, 50)
        }
        SymbolStatus::Unreviewed | SymbolStatus::OtherVersion => Color32::from_rgb(230, 140, 0),
        SymbolStatus::Refused => Color32::from_rgb(0, 160, 190),
        SymbolStatus::NoFootprint => Color32::GRAY,
    }
}

fn file_name(p: &Path) -> String {
    p.file_name().map_or_else(
        || p.display().to_string(),
        |f| f.to_string_lossy().into_owned(),
    )
}

fn blend_hint(m: BlendMode) -> &'static str {
    match m {
        BlendMode::Normal => "The footprint over the page, by its opacity",
        BlendMode::Multiply => {
            "Page times footprint: the drawing's lines stay dark through the footprint"
        }
        BlendMode::Screen => "The inverse of multiply: light shapes stay light",
        BlendMode::Difference => {
            "The difference of page and footprint: with white copper (Settings), white paper under it goes black and \
             the drawing's pads show light where the footprint misses them"
        }
    }
}

fn layer_colour(c: &LayerColors, k: Kind) -> Rgb {
    match k {
        Kind::Copper | Kind::BackCopper => c.copper,
        Kind::Mask | Kind::BackMask => c.mask,
        Kind::Paste | Kind::BackPaste => c.paste,
        Kind::Courtyard => c.courtyard,
        Kind::Fab => c.fab,
        Kind::Silk => c.silk,
        Kind::Edge => c.edge,
    }
}

/// The source by file name: `Mixed_Pads.kicad_mod`, `U3 on b153a.kicad_pcb`, `Lib:Name (library)`
fn short_source(s: &FootprintSource) -> String {
    let name = |p: &Path| {
        p.file_name().map_or_else(
            || p.display().to_string(),
            |f| f.to_string_lossy().into_owned(),
        )
    };
    match s {
        FootprintSource::File(p) => name(p),
        FootprintSource::Board { board, reference } => format!("{reference} on {}", name(board)),
        FootprintSource::Library { .. } => s.describe(),
    }
}

fn reference(s: &Sourced) -> String {
    match &s.source {
        FootprintSource::Board { reference, .. } => reference.clone(),
        _ => String::new(),
    }
}

/// What to search the library for, best first: the datasheet's PDF file name, a part name in its web address, the
/// part numbers (`Mpn1`, `MPN`, ...; packing suffixes after a comma cut off), the value
fn search_queries(s: &Sourced) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |q: &str| {
        let q = q.trim();
        let good = q.len() >= 3
            && q.chars()
                .all(|c| c.is_alphanumeric() || "-_.+ ".contains(c));
        if good && !out.iter().any(|o| o == q) {
            out.push(q.to_string());
        }
    };
    if let Some(ds) = s.field("Datasheet") {
        let (path, query) = ds.split_once('?').unwrap_or((ds, ""));
        let file = path.rsplit('/').next().unwrap_or("");
        if let Some(stem) = file
            .strip_suffix(".pdf")
            .or_else(|| file.strip_suffix(".PDF"))
        {
            add(stem);
        }
        for kv in query.split('&') {
            if let Some((k, v)) = kv.split_once('=') {
                let k = k.to_lowercase();
                if ["prodname", "part", "partnumber", "pn", "mpn", "product"].contains(&k.as_str())
                {
                    add(v);
                }
            }
        }
    }
    let mut mpns: Vec<&(String, String)> = s
        .fields
        .iter()
        .filter(|(k, _)| {
            let k = k.to_lowercase();
            k.starts_with("mpn") || k == "manufacturer_part_number" || k == "mfr_pn"
        })
        .collect();
    mpns.sort_by(|a, b| better_erc::model::natural_cmp(&a.0, &b.0));
    for (_, v) in mpns {
        add(v.split([',', '/']).next().unwrap_or(""));
    }
    if let Some(v) = s.field("Value") {
        add(v);
    }
    out
}

/// A drawing's own scale note in a page's text (`SCALE: 20X`, `SCALE 10:1`): the note and its factor
fn scale_note(text: &str) -> Option<(String, f64)> {
    let lower = text.to_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find("scale") {
        let start = from + i;
        let rest = &lower[start + 5..];
        let skip = rest.len() - rest.trim_start_matches([':', ' ', '=', '\t']).len();
        let num: String = rest[skip..]
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let after = &rest[skip + num.len()..];
        let factor = num.parse::<f64>().ok().filter(|f| *f > 0.0);
        let end = if after.starts_with('x') {
            Some(1)
        } else if after.starts_with(":1") {
            Some(2)
        } else {
            None
        };
        if let (Some(f), Some(e)) = (factor, end) {
            let len = 5 + skip + num.len() + e;
            return Some((text[start..start + len].to_string(), f));
        }
        from = start + 5;
    }
    None
}

fn lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

/// The program on PATH (or the path itself, when it names a file)
fn find_program(name: &str) -> Option<PathBuf> {
    let p = Path::new(name);
    if p.components().count() > 1 {
        return p.is_file().then(|| p.to_path_buf());
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|d| d.join(name))
        .find(|f| f.is_file())
}

/// Runs the library search command with `words` added; it prints a JSON list of `{"path", "page"}`
pub fn run_library_search(cmd: &[String], words: &[String]) -> Result<Vec<LibraryHit>, String> {
    let (prog, args) = cmd.split_first().ok_or("no library search command")?;
    let out = std::process::Command::new(prog)
        .args(args)
        .args(words)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("can't run {prog}: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() && text.trim().is_empty() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("{} failed: {}", cmd.join(" "), err.trim()));
    }
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str::<Vec<LibraryHit>>(&text)
        .map_err(|e| format!("{} printed no JSON list of paths: {e}", cmd.join(" ")))
}
