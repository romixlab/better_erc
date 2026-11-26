//! Where a probe reaches each net of a board, by side: the library side of `berc probe-access` (`TOOL-16`).
//!
//! Test points and connectors, fitted or not, are what the DBG and TEST checks count ([`erc_analysis::access`]),
//! by their part class; a pad KiCad marks as a test point (`pad_prop_testpoint`) counts too, for a board-only
//! footprint. Not every connector is somewhere a probe or a wire goes: one of a kind that is shielded or fine
//! pitch by nature (USB, HDMI, RJ45, card socket, FPC, coax, from its part class) never counts, and any other only
//! with a pad pitch of at least [`ProbeOptions::connector_pitch_mm_min`].
//!
//! A hand probe also touches the pads of other parts ([`AccessKind::PartPad`]): the ends of a two-terminal
//! passive of at least [`ProbeOptions::passive_size_min`], and the leads of any other part with a pitch of at
//! least [`ProbeOptions::part_pitch_mm_min`]; never a pad under the part's body.
//!
//! A pad that does not count is still listed, with why ([`NotCounted`]). A pad is on a side when it has copper and
//! a mask opening there: an SMD pad on its footprint's side, a through-hole pad on both. Behind one series part (a
//! resistor, ferrite, net tie or jumper, as [`erc_analysis::access::series_neighbours`] finds it) counts apart, as
//! [`Reach::Series`]. Each net lists its vias with the outer sides they reach, for a tool that opens the solder
//! mask on them.

mod part_pad;

use erc_analysis::access::{is_access, series_neighbours};
pub use erc_analysis::package::CaseSize;
use erc_checks::{CheckError, Config, Ctx, Project};
use erc_model::class::ConnectorKind;
use erc_model::layout::{FootprintId, Layout, Pad, PadId, Pt};
use erc_model::{BoardId, Design, NetId, PartClass};
use part_pad::PartFootprint;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// The version of the JSON shape; a reader tolerates fields it does not know
pub const SCHEMA_VERSION: u32 = 1;

/// The pad property KiCad gives a test point pad
const TESTPOINT_PROPERTY: &str = "pad_prop_testpoint";

/// The narrowest connector pad pitch a probe or a soldered wire still goes to by default, mm: 1.27 mm headers
/// count, 1.0 mm and finer do not
pub const CONNECTOR_PITCH_MM_MIN: f64 = 1.27;

/// The smallest case size of a two-terminal passive whose pads count by default: 0402 (1005 metric), 0201 does not
pub const PASSIVE_SIZE_MIN: CaseSize = CaseSize::IMPERIAL_0402;

/// The narrowest lead pitch of any other part a probe still touches by default, mm: SOT, SOIC and 0.65 mm TSSOP
/// count, 0.5 mm and finer do not
pub const PART_PITCH_MM_MIN: f64 = 0.65;

/// What counts as access
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProbeOptions {
    /// A connector counts only with its pads at least this far apart, mm (centre to centre, the nearest two)
    pub connector_pitch_mm_min: f64,
    /// A two-terminal passive's pads count only when its footprint is of this case size or larger
    pub passive_size_min: CaseSize,
    /// The pads of any other part count only with its leads at least this far apart, mm (centre to centre, the
    /// nearest two of different numbers)
    pub part_pitch_mm_min: f64,
}

impl Default for ProbeOptions {
    fn default() -> ProbeOptions {
        ProbeOptions {
            connector_pitch_mm_min: CONNECTOR_PITCH_MM_MIN,
            passive_size_min: PASSIVE_SIZE_MIN,
            part_pitch_mm_min: PART_PITCH_MM_MIN,
        }
    }
}

/// Why a pad does not count as access
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum NotCounted {
    /// A kind of connector that is shielded or fine pitch by nature (`UsbC`, `Fpc`, `Coax`)
    Kind { kind: String },
    /// Its pads are closer than the minimum pitch
    Pitch { pitch_mm: f64, min_mm: f64 },
    /// A passive of a case size below the minimum
    Size { size: String, min: String },
    /// Under the part's body: a bottom terminal, a ball, an exposed thermal pad
    UnderBody,
}

impl fmt::Display for NotCounted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotCounted::Kind { kind } => write!(f, "{kind}"),
            NotCounted::Pitch { pitch_mm, min_mm } => write!(f, "pitch {pitch_mm} < {min_mm} mm"),
            NotCounted::Size { size, min } => write!(f, "size {size} < {min}"),
            NotCounted::UnderBody => f.write_str("under the body"),
        }
    }
}

/// An outer side of the board
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Top,
    Bottom,
}

impl Side {
    /// `top` or `bottom`
    pub fn name(self) -> &'static str {
        match self {
            Side::Top => "top",
            Side::Bottom => "bottom",
        }
    }

    fn layers(self) -> (&'static str, &'static str) {
        match self {
            Side::Top => ("F.Cu", "F.Mask"),
            Side::Bottom => ("B.Cu", "B.Mask"),
        }
    }
}

/// What a probe touches
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessKind {
    TestPoint,
    Connector,
    /// A pad of any other part
    PartPad,
}

/// How a probe reaches a net on one side
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reach {
    /// A pad that counts on the net itself
    Direct,
    /// Only one series part away
    Series,
    /// Neither
    None,
}

/// A rail or a ground, as the power analysis finds it
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerKind {
    Rail,
    Ground,
}

/// A pad of a footprint on the net, as a probe reaches it
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Access {
    /// The footprint's reference (`TP3`, `J1`)
    pub reference: String,
    /// The pad number
    pub pad: String,
    pub kind: AccessKind,
    /// The sides the pad has copper and a mask opening on
    pub sides: Vec<Side>,
    /// The series part between the pad's net and this one; none for a pad on the net itself
    pub series: Option<String>,
    /// The pitch the pad was judged by, mm, centre to centre: a connector's nearest two pads, a part's nearest two
    /// leads of different numbers; none for a test point, a one-pad footprint, a pad under the body and a passive
    /// judged by its size
    pub pitch_mm: Option<f64>,
    /// The case size a passive was judged by (`0402`), from its footprint name
    pub size: Option<String>,
    /// Why the pad does not count as access; none when it counts
    pub not_counted: Option<NotCounted>,
}

/// A via of the net
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViaAccess {
    /// Position in mm, KiCad's board frame (y down)
    pub at_mm: [f64; 2],
    pub drill_mm: f64,
    /// `through`, `blind`, `micro`
    pub kind: String,
    /// The outer sides its copper reaches
    pub sides: Vec<Side>,
}

/// One net of the board and where a probe reaches it
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetAccess {
    /// The board's net name (`/SDA`, `GND`)
    pub net: String,
    pub power: Option<PowerKind>,
    /// Its pads on the board, of any footprint: a net of one pad has nothing to probe between
    pub pads: usize,
    pub top: Reach,
    pub bottom: Reach,
    pub access: Vec<Access>,
    pub vias: Vec<ViaAccess>,
}

impl NetAccess {
    /// How a probe reaches the net on `side`
    pub fn reach(&self, side: Side) -> Reach {
        match side {
            Side::Top => self.top,
            Side::Bottom => self.bottom,
        }
    }
}

/// Every net of a board with where a probe reaches it
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProbeAccess {
    pub schema: u32,
    /// The board file, relative to the project directory
    pub board: String,
    /// Whether part classes came from a schematic; without one only `pad_prop_testpoint` pads count
    pub schematic: bool,
    /// Sorted by name
    pub nets: Vec<NetAccess>,
}

/// Why the probe access could not be found
#[derive(Debug)]
pub enum ProbeError {
    /// The design could not be read
    Open(CheckError),
    /// The design has no board file
    NoBoard(PathBuf),
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProbeError::Open(e) => e.fmt(f),
            ProbeError::NoBoard(p) => write!(f, "{}: no board file (.kicad_pcb)", p.display()),
        }
    }
}

impl std::error::Error for ProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProbeError::Open(e) => Some(e),
            ProbeError::NoBoard(_) => None,
        }
    }
}

/// The probe access of the design at `path` (a directory, a `.kicad_pro`, `.kicad_sch` or `.kicad_pcb`), on its
/// first board with a board file
pub fn probe_access(path: &Path, opts: &ProbeOptions) -> Result<ProbeAccess, ProbeError> {
    let project = Project::open_without_parts(path).map_err(ProbeError::Open)?;
    let config = project.config(&Config::default());
    let design = &project.imported.design;
    (0..design.boards.len())
        .find_map(|bi| of(design, BoardId::from_index(bi), &config, opts))
        .ok_or_else(|| ProbeError::NoBoard(path.to_path_buf()))
}

/// The probe access of one board of a design, in its base variant; `None` without a board file
pub fn of(
    design: &Design,
    board: BoardId,
    config: &Config,
    opts: &ProbeOptions,
) -> Option<ProbeAccess> {
    let cx = Ctx::new(design, board, None, config);
    let b = cx.board();
    let layout = b.layout.as_ref()?;
    let names = NetNames::of(layout);
    let mut nets: BTreeMap<String, NetAccess> = BTreeMap::new();
    for name in &layout.net_names {
        if !name.is_empty() {
            entry(&mut nets, name);
        }
    }
    for pad in &layout.pads {
        if let Some(name) = pad.net_name.as_deref().filter(|n| !n.is_empty()) {
            entry(&mut nets, name).pads += 1;
        }
    }
    let direct = direct_access(&cx, layout, opts);
    for (name, access) in &direct {
        entry(&mut nets, name).access.extend(access.iter().cloned());
    }
    for via in &layout.vias {
        let Some(name) = via.net_name.as_deref().filter(|n| !n.is_empty()) else {
            continue;
        };
        let sides = [Side::Top, Side::Bottom]
            .into_iter()
            .filter(|s| via.layers.iter().any(|l| l == s.layers().0))
            .collect();
        entry(&mut nets, name).vias.push(ViaAccess {
            at_mm: [mm(via.at.x), mm(via.at.y)],
            drill_mm: mm(via.drill),
            kind: via.kind.clone(),
            sides,
        });
    }
    if b.parts.is_empty() {
        return Some(finish(layout, nets, false));
    }
    let power = cx.power();
    for (net, name) in &names.board_of {
        let n = entry(&mut nets, name);
        n.power = if power.is_ground(*net) {
            Some(PowerKind::Ground)
        } else if power.is_rail(*net) {
            Some(PowerKind::Rail)
        } else {
            n.power
        };
        let mut behind = Vec::new();
        for (part, far) in series_neighbours(cx.view(), cx.classes(), power, *net) {
            let Some(far_name) = names.board_of.get(&far) else {
                continue;
            };
            let through = &b.part(part).designator;
            for a in direct.get(far_name.as_str()).into_iter().flatten() {
                if a.reference != *through {
                    behind.push(Access {
                        series: Some(through.clone()),
                        ..a.clone()
                    });
                }
            }
        }
        entry(&mut nets, name).access.extend(behind);
    }
    Some(finish(layout, nets, true))
}

/// The net of that name, added empty when new
fn entry<'n>(nets: &'n mut BTreeMap<String, NetAccess>, name: &str) -> &'n mut NetAccess {
    nets.entry(name.to_owned()).or_insert_with(|| NetAccess {
        net: name.to_owned(),
        power: None,
        pads: 0,
        top: Reach::None,
        bottom: Reach::None,
        access: Vec::new(),
        vias: Vec::new(),
    })
}

/// Sorts each net's access and works out its reach per side
fn finish(layout: &Layout, nets: BTreeMap<String, NetAccess>, schematic: bool) -> ProbeAccess {
    let nets = nets
        .into_values()
        .map(|mut n| {
            n.access.sort_by(listing_order);
            n.access.dedup();
            let reach = |side: Side| {
                let on: Vec<&Access> = n
                    .access
                    .iter()
                    .filter(|a| a.not_counted.is_none() && a.sides.contains(&side))
                    .collect();
                if on.iter().any(|a| a.series.is_none()) {
                    Reach::Direct
                } else if on.is_empty() {
                    Reach::None
                } else {
                    Reach::Series
                }
            };
            n.top = reach(Side::Top);
            n.bottom = reach(Side::Bottom);
            n
        })
        .collect();
    ProbeAccess {
        schema: SCHEMA_VERSION,
        board: layout.file.clone(),
        schematic,
        nets,
    }
}

/// The order a net lists its access in, the best places first: pads that count, on the net itself, test points
/// before connectors before other parts' pads
fn listing_order(a: &Access, b: &Access) -> Ordering {
    let rank = |x: &Access| (x.not_counted.is_some(), x.series.is_some(), x.kind);
    rank(a)
        .cmp(&rank(b))
        .then_with(|| a.partial_cmp(b).unwrap_or(Ordering::Equal))
}

/// The board net name of each schematic net, from the pads that carry both
struct NetNames {
    board_of: BTreeMap<NetId, String>,
}

impl NetNames {
    fn of(layout: &Layout) -> NetNames {
        let board_of = layout
            .pads
            .iter()
            .filter_map(|p| Some((p.net?, p.net_name.clone()?)))
            .filter(|(_, n)| !n.is_empty())
            .collect();
        NetNames { board_of }
    }
}

/// The pads of each board net a probe may touch, on the net itself, counted or not
fn direct_access<'l>(
    cx: &Ctx,
    layout: &'l Layout,
    opts: &ProbeOptions,
) -> BTreeMap<&'l str, Vec<Access>> {
    let pitch = pitches(layout);
    let mut parts: BTreeMap<FootprintId, PartFootprint> = BTreeMap::new();
    let mut out: BTreeMap<&str, Vec<Access>> = BTreeMap::new();
    for (pad_index, pad) in layout.pads.iter().enumerate() {
        let Some(name) = pad.net_name.as_deref().filter(|n| !n.is_empty()) else {
            continue;
        };
        let fp = layout.footprint(pad.footprint);
        let class = fp.part.map(|p| (p, cx.classes().get(p)));
        let sides = pad_sides(pad);
        if sides.is_empty() || pad.kind == "np_thru_hole" {
            continue;
        }
        let marked = pad.property.as_deref() == Some(TESTPOINT_PROPERTY);
        let access = |kind, pitch_mm, size, not_counted| Access {
            reference: fp.reference.clone(),
            pad: pad.number.clone(),
            kind,
            sides: sides.clone(),
            series: None,
            pitch_mm,
            size,
            not_counted,
        };
        let found = match class {
            Some((_, PartClass::TestPoint)) => access(AccessKind::TestPoint, None, None, None),
            Some((p, c)) if is_access(cx.classes(), p) => {
                let pitch_mm = pitch[pad.footprint.index()];
                access(
                    AccessKind::Connector,
                    pitch_mm,
                    None,
                    connector_not_counted(c, pitch_mm, opts),
                )
            }
            _ if marked => access(AccessKind::TestPoint, None, None, None),
            Some((_, c)) => {
                let part = parts
                    .entry(pad.footprint)
                    .or_insert_with(|| PartFootprint::of(layout, pad.footprint));
                let v = part.judge(PadId::from_index(pad_index), c, opts);
                access(AccessKind::PartPad, v.pitch_mm, v.size, v.not_counted)
            }
            None => continue,
        };
        out.entry(name).or_default().push(found);
    }
    out
}

/// Why a connector's pads do not count, if they do not
fn connector_not_counted(
    class: PartClass,
    pitch_mm: Option<f64>,
    opts: &ProbeOptions,
) -> Option<NotCounted> {
    if let PartClass::Connector { kind: Some(k) } = class
        && never_counts(k)
    {
        return Some(NotCounted::Kind {
            kind: format!("{k:?}"),
        });
    }
    let min_mm = opts.connector_pitch_mm_min;
    pitch_mm
        .filter(|p| *p < min_mm)
        .map(|pitch_mm| NotCounted::Pitch { pitch_mm, min_mm })
}

/// A connector kind a probe or wire never goes to: shielded, or fine pitch by nature
fn never_counts(kind: ConnectorKind) -> bool {
    use ConnectorKind as K;
    match kind {
        K::UsbA
        | K::UsbB
        | K::UsbMicro
        | K::UsbC
        | K::Hdmi
        | K::MicroHdmi
        | K::Rj45
        | K::Card
        | K::Fpc
        | K::Coax => true,
        K::Barrel | K::TerminalBlock | K::Header => false,
    }
}

/// Each footprint's pad pitch, mm: the nearest two of its numbered pads, centre to centre, rounded to the µm;
/// none with fewer than two
fn pitches(layout: &Layout) -> Vec<Option<f64>> {
    layout
        .footprints
        .iter()
        .map(|fp| {
            let at: Vec<Pt> = fp
                .pads
                .iter()
                .map(|p| layout.pad(*p))
                .filter(|p| !p.number.is_empty() && p.kind != "np_thru_hole")
                .map(|p| p.at)
                .collect();
            let mut nearest: Option<f64> = None;
            for (i, a) in at.iter().enumerate() {
                for b in &at[i + 1..] {
                    let d = ((a.x - b.x) as f64).hypot((a.y - b.y) as f64);
                    if d > 0.0 && nearest.is_none_or(|n| d < n) {
                        nearest = Some(d);
                    }
                }
            }
            nearest.map(|d| (d / 1e3).round() / 1e3)
        })
        .collect()
}

/// The sides a pad has copper and a mask opening on
fn pad_sides(pad: &Pad) -> Vec<Side> {
    let has = |l: &str| {
        pad.layers
            .iter()
            .any(|x| x == l || (x.starts_with("*.") && l.ends_with(&x[1..])))
    };
    [Side::Top, Side::Bottom]
        .into_iter()
        .filter(|s| {
            let (cu, mask) = s.layers();
            has(cu) && has(mask)
        })
        .collect()
}

fn mm(nm: i64) -> f64 {
    nm as f64 / 1e6
}

/// The nets as a table: net, top, bottom, vias, where; then the counts of nets without access per side
pub fn render(p: &ProbeAccess) -> String {
    let reach = |r: Reach| match r {
        Reach::Direct => "yes",
        Reach::Series => "series",
        Reach::None => "-",
    };
    let mut rows = vec![[
        "net".to_owned(),
        "top".to_owned(),
        "bottom".to_owned(),
        "vias".to_owned(),
        "access".to_owned(),
    ]];
    for n in &p.nets {
        let at: Vec<String> = n
            .access
            .iter()
            .map(|a| {
                let sides: Vec<&str> = a.sides.iter().map(|s| s.name()).collect();
                let series = a
                    .series
                    .as_ref()
                    .map_or(String::new(), |s| format!(" via {s}"));
                let not = a
                    .not_counted
                    .as_ref()
                    .map_or(String::new(), |n| format!(" not counted: {n}"));
                format!(
                    "{}.{} ({}){series}{not}",
                    a.reference,
                    a.pad,
                    sides.join("+")
                )
            })
            .collect();
        rows.push([
            n.net.clone(),
            reach(n.top).to_owned(),
            reach(n.bottom).to_owned(),
            n.vias.len().to_string(),
            at.join(", "),
        ]);
    }
    let width: Vec<usize> = (0..4)
        .map(|i| rows.iter().map(|r| r[i].chars().count()).max().unwrap_or(0))
        .collect();
    let mut out = String::new();
    for r in &rows {
        let mut line = String::new();
        for (i, w) in width.iter().enumerate() {
            line += &format!("{:<w$}  ", r[i]);
        }
        line += &r[4];
        out += line.trim_end();
        out.push('\n');
    }
    for side in [Side::Top, Side::Bottom] {
        let none = p
            .nets
            .iter()
            .filter(|n| n.reach(side) == Reach::None)
            .count();
        out += &format!(
            "{}: {none} of {} nets without access\n",
            side.name(),
            p.nets.len()
        );
    }
    if !p.schematic {
        out += "No schematic: only pads marked as test points count.\n";
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn pad(layers: &[&str]) -> Pad {
        Pad {
            footprint: erc_model::layout::FootprintId(0),
            number: "1".into(),
            kind: "smd".into(),
            shape: "circle".into(),
            at: Pt::default(),
            rotation: 0.0,
            size: Pt::default(),
            drill: None,
            layers: layers.iter().map(|s| (*s).to_owned()).collect(),
            net_name: None,
            net: None,
            property: None,
            pin_function: None,
            pin_type: None,
            solder_mask_margin: None,
            solder_paste_margin: None,
            solder_paste_ratio: None,
            zone_connect: None,
            primitives: Vec::new(),
            details: Default::default(),
        }
    }

    #[test]
    fn pad_sides_need_copper_and_mask() {
        assert_eq!(pad_sides(&pad(&["F.Cu", "F.Mask"])), [Side::Top]);
        assert_eq!(
            pad_sides(&pad(&["B.Cu", "B.Paste", "B.Mask"])),
            [Side::Bottom]
        );
        assert_eq!(
            pad_sides(&pad(&["*.Cu", "*.Mask"])),
            [Side::Top, Side::Bottom]
        );
        assert!(pad_sides(&pad(&["F.Cu"])).is_empty());
    }

    #[test]
    fn mm_from_nm() {
        assert_eq!(mm(1_250_000), 1.25);
        assert_eq!(mm(-300_000), -0.3);
    }
}
