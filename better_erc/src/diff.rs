//! Comparisons of two designs on disk, the library side of `berc diff`, each side any path [`Project::open`] reads:
//! the placements ([`pnp`]: two PnP files, two boards, or a board and a PnP file) and the nets ([`nets`]: any two
//! designs with connectivity, a KiCad schematic and an Altium Smart PDF of the same board as well as two revisions).
//! The netlist, BOM and findings deltas between two git refs come with RPT-16 and RPT-17.

use crate::{CheckError, Project};
use erc_model::Board;
use erc_model::layout::Layout;
use erc_model::layout::{Nm, Pt, Side};
use erc_model::netcmp::{self, NetDiff};
use erc_model::placecmp::{self, PlaceChange, PlaceDiff, PlaceOptions};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// One side of a placement comparison
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlaceSide {
    /// The design file read (`board.kicad_pcb`, `pos.csv`)
    pub file: PathBuf,
    /// The footprints compared (a board's without those excluded from position files, when the other side is a
    /// PnP file)
    pub footprints: usize,
    /// Of them, test points: references starting with `TP`
    pub test_points: usize,
    /// Read from a board file, in the board's frame
    pub board_file: bool,
}

/// The placements of two designs compared
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PnpDiff {
    pub a: PlaceSide,
    pub b: PlaceSide,
    /// The PnP origin a board was brought to, when one side is a board and the other a PnP file
    pub origin: Option<placecmp::Origin>,
    pub diff: PlaceDiff,
}

/// Why a comparison could not run (exit code 2)
#[derive(Debug)]
pub enum DiffError {
    /// One side could not be opened
    Open(CheckError),
    /// One side has no placements: neither a PnP file nor a board file
    NoLayout(PathBuf),
    /// One side has no nets (a PnP file)
    NoNets(PathBuf),
}

impl DiffError {
    pub const EXIT_CODE: u8 = CheckError::EXIT_CODE;
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffError::Open(e) => e.fmt(f),
            DiffError::NoLayout(p) => write!(
                f,
                "{}: no placements (a PnP file, a .kicad_pcb, or a KiCad project with its board)",
                p.display()
            ),
            DiffError::NoNets(p) => write!(
                f,
                "{}: no nets (a schematic, a netlist or a Smart PDF; a PnP file has none)",
                p.display()
            ),
        }
    }
}

impl std::error::Error for DiffError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DiffError::Open(e) => Some(e),
            DiffError::NoLayout(_) | DiffError::NoNets(_) => None,
        }
    }
}

/// Compares the placements of the designs at `a` and `b` ([`placecmp::compare_boards`]): the first board of each
/// that has a layout
pub fn pnp(a: &Path, b: &Path, opts: &PlaceOptions) -> Result<PnpDiff, DiffError> {
    let (pa, pb) = (open(a)?, open(b)?);
    let (ba, bb) = (placed(&pa).expect("opened"), placed(&pb).expect("opened"));
    let diff = placecmp::compare_boards(ba, bb, opts).expect("both have a layout");
    let (la, lb) = (layout(ba), layout(bb));
    let mixed = placecmp::is_board_file(la) != placecmp::is_board_file(lb);
    let side = |p: &Project, l: &Layout| {
        let compared = if mixed && placecmp::is_board_file(l) {
            placecmp::to_placement_frame(l, opts.origin)
        } else {
            l.clone()
        };
        PlaceSide {
            file: p.dir.join(&p.main),
            footprints: compared.footprints.len(),
            test_points: compared
                .footprints
                .iter()
                .filter(|f| f.reference.starts_with("TP"))
                .count(),
            board_file: placecmp::is_board_file(l),
        }
    };
    Ok(PnpDiff {
        a: side(&pa, la),
        b: side(&pb, lb),
        origin: mixed.then_some(opts.origin),
        diff,
    })
}

/// One side of a net comparison
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetSide {
    /// The design file read
    pub file: PathBuf,
    /// The importer's tool name (`kicad`, `altium-pdf`)
    pub tool: String,
    /// The nets compared: those with a pin of a part on the board ([`netcmp::pin_sets`])
    pub nets: usize,
    /// The pins on them
    pub pins: usize,
}

/// The nets of two designs compared, each net as the pins it connects
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetsDiff {
    pub a: NetSide,
    pub b: NetSide,
    pub diff: NetDiff,
}

/// Compares the nets of the designs at `a` and `b` ([`netcmp::compare`]): the first board of each that has nets
pub fn nets(a: &Path, b: &Path) -> Result<NetsDiff, DiffError> {
    let open = |path: &Path| -> Result<Project, DiffError> {
        let p = Project::open(path).map_err(DiffError::Open)?;
        match netted(&p) {
            Some(_) => Ok(p),
            None => Err(DiffError::NoNets(path.to_path_buf())),
        }
    };
    let (pa, pb) = (open(a)?, open(b)?);
    let (ba, bb) = (netted(&pa).expect("opened"), netted(&pb).expect("opened"));
    let side = |p: &Project, board: &Board| {
        let sets = netcmp::pin_sets(board);
        NetSide {
            file: p.dir.join(&p.main),
            tool: board.source.tool.clone(),
            nets: sets.len(),
            pins: sets.iter().map(|(_, s)| s.len()).sum(),
        }
    };
    Ok(NetsDiff {
        a: side(&pa, ba),
        b: side(&pb, bb),
        diff: netcmp::compare(ba, bb),
    })
}

/// The first board with a net
fn netted(p: &Project) -> Option<&Board> {
    p.imported
        .design
        .boards
        .iter()
        .find(|b| !netcmp::pin_sets(b).is_empty())
}

/// A design with placements
fn open(path: &Path) -> Result<Project, DiffError> {
    let project = Project::open(path).map_err(DiffError::Open)?;
    match placed(&project) {
        Some(_) => Ok(project),
        None => Err(DiffError::NoLayout(path.to_path_buf())),
    }
}

/// The first board with a layout
fn placed(p: &Project) -> Option<&Board> {
    p.imported.design.boards.iter().find(|b| b.layout.is_some())
}

fn layout(b: &Board) -> &Layout {
    b.layout.as_ref().expect("a board with a layout")
}

fn mm(nm: i64) -> String {
    Nm(nm).to_string()
}

fn point(p: Pt) -> String {
    format!("{}, {}", mm(p.x), mm(p.y))
}

fn side_name(s: Side) -> &'static str {
    match s {
        Side::Front => "front",
        Side::Back => "back",
    }
}

impl PnpDiff {
    /// Whether the placements are the same
    pub fn is_empty(&self) -> bool {
        self.diff.is_empty()
    }

    /// The comparison as text: the two sides, then each changed, removed and added reference on a line
    pub fn to_text(&self) -> String {
        self.text(false)
    }

    /// [`PnpDiff::to_text`] with colours: changed yellow, removed red, added green
    pub fn to_text_color(&self) -> String {
        self.text(true)
    }

    /// The comparison as JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialises")
    }

    fn text(&self, color: bool) -> String {
        let style = |c: anstyle::AnsiColor| {
            if color {
                c.on_default().bold()
            } else {
                anstyle::Style::new()
            }
        };
        let (yellow, red, green) = (
            style(anstyle::AnsiColor::Yellow),
            style(anstyle::AnsiColor::Red),
            style(anstyle::AnsiColor::Green),
        );
        let mut out = String::new();
        for (name, s) in [("A", &self.a), ("B", &self.b)] {
            let frame = match (s.board_file, self.origin) {
                (true, Some(placecmp::Origin::Page)) => ", as a PnP file from the page origin",
                (true, Some(placecmp::Origin::Aux)) => ", as a PnP file from the aux origin",
                _ => "",
            };
            let _ = writeln!(
                out,
                "{name}: {} ({} footprints, {} test points{frame})",
                s.file.display(),
                s.footprints,
                s.test_points
            );
        }
        if self.is_empty() {
            out.push_str("same placements\n");
            return out;
        }
        for m in &self.diff.changed {
            let what: Vec<String> = m
                .changes
                .iter()
                .map(|c| match c {
                    PlaceChange::Position { a, b } => {
                        format!("position {} -> {}", point(*a), point(*b))
                    }
                    PlaceChange::Rotation { a, b } => format!("rotation {a} -> {b}"),
                    PlaceChange::Side { a, b } => {
                        format!("side {} -> {}", side_name(*a), side_name(*b))
                    }
                    PlaceChange::Value { a, b } => format!("value {a} -> {b}"),
                    PlaceChange::Package { a, b } => format!("package {a} -> {b}"),
                })
                .collect();
            let _ = writeln!(
                out,
                "{yellow}changed{yellow:#} {}: {}",
                m.reference,
                what.join("; ")
            );
        }
        for r in &self.diff.removed {
            let _ = writeln!(out, "{red}removed{red:#} {r}");
        }
        for r in &self.diff.added {
            let _ = writeln!(out, "{green}added{green:#}   {r}");
        }
        let d = &self.diff;
        let _ = writeln!(
            out,
            "{} changed, {} removed, {} added",
            d.changed.len(),
            d.removed.len(),
            d.added.len()
        );
        out
    }
}

impl NetsDiff {
    /// Whether the nets are the same, names and classes included
    pub fn is_empty(&self) -> bool {
        self.diff.is_empty()
    }

    /// The comparison as text: the two sides, then each renamed, removed and added net and each class change on a
    /// line
    pub fn to_text(&self) -> String {
        self.text(false)
    }

    /// [`NetsDiff::to_text`] with colours: renamed and class changes yellow, removed red, added green
    pub fn to_text_color(&self) -> String {
        self.text(true)
    }

    /// The comparison as JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialises")
    }

    fn text(&self, color: bool) -> String {
        let style = |c: anstyle::AnsiColor| {
            if color {
                c.on_default().bold()
            } else {
                anstyle::Style::new()
            }
        };
        let (yellow, red, green) = (
            style(anstyle::AnsiColor::Yellow),
            style(anstyle::AnsiColor::Red),
            style(anstyle::AnsiColor::Green),
        );
        let mut out = String::new();
        for (name, s) in [("A", &self.a), ("B", &self.b)] {
            let _ = writeln!(
                out,
                "{name}: {} ({}: {} nets, {} pins)",
                s.file.display(),
                s.tool,
                s.nets,
                s.pins
            );
        }
        let d = &self.diff;
        if self.is_empty() {
            let _ = writeln!(out, "same nets ({})", d.same);
            return out;
        }
        let pins = |s: &netcmp::PinSet| s.iter().cloned().collect::<Vec<_>>().join(" ");
        for (a, b) in &d.renamed {
            let _ = writeln!(out, "{yellow}renamed{yellow:#} {a} -> {b}");
        }
        for (n, a, b) in &d.classes {
            let _ = writeln!(out, "{yellow}class{yellow:#}   {n}: {a} -> {b}");
        }
        for (n, s) in &d.only_a {
            let _ = writeln!(out, "{red}removed{red:#} {n}: {}", pins(s));
        }
        for (n, s) in &d.only_b {
            let _ = writeln!(out, "{green}added{green:#}   {n}: {}", pins(s));
        }
        let _ = writeln!(
            out,
            "{} same, {} renamed, {} class changes, {} removed, {} added",
            d.same,
            d.renamed.len(),
            d.classes.len(),
            d.only_a.len(),
            d.only_b.len()
        );
        out
    }
}
