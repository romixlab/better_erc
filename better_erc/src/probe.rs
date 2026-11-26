//! Where a probe reaches each net of a board, by side: the library side of `berc probe-access` (`TOOL-16`).
//!
//! The access parts are what the DBG and TEST checks count ([`erc_analysis::access`]): test points and connectors,
//! fitted or not, by their part class; a pad KiCad marks as a test point (`pad_prop_testpoint`) counts too, for a
//! board-only footprint. A pad is on a side when it has copper and a mask opening there: an SMD pad on its
//! footprint's side, a through-hole pad on both. Behind one series part (a resistor, ferrite, net tie or jumper,
//! as [`erc_analysis::access::series_neighbours`] finds it) counts apart, as [`Reach::Series`]. Each net lists its
//! vias with the outer sides they reach, for a tool that opens the solder mask on them.

use erc_analysis::access::{is_access, series_neighbours};
use erc_checks::{CheckError, Config, Ctx, Project};
use erc_model::layout::{Layout, Pad};
use erc_model::{BoardId, Design, NetId, PartClass};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// The version of the JSON shape; a reader tolerates fields it does not know
pub const SCHEMA_VERSION: u32 = 1;

/// The pad property KiCad gives a test point pad
const TESTPOINT_PROPERTY: &str = "pad_prop_testpoint";

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
}

/// How a probe reaches a net on one side
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reach {
    /// A test point or connector pad on the net itself
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

/// A test point or connector pad a probe reaches the net at
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
pub fn probe_access(path: &Path) -> Result<ProbeAccess, ProbeError> {
    let project = Project::open_without_parts(path).map_err(ProbeError::Open)?;
    let config = project.config(&Config::default());
    let design = &project.imported.design;
    (0..design.boards.len())
        .find_map(|bi| of(design, BoardId::from_index(bi), &config))
        .ok_or_else(|| ProbeError::NoBoard(path.to_path_buf()))
}

/// The probe access of one board of a design, in its base variant; `None` without a board file
pub fn of(design: &Design, board: BoardId, config: &Config) -> Option<ProbeAccess> {
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
    let direct = direct_access(&cx, layout);
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
            for a in direct.get(far_name.as_str()).into_iter().flatten() {
                behind.push(Access {
                    series: Some(b.part(part).designator.clone()),
                    ..a.clone()
                });
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
            n.access.sort();
            n.access.dedup();
            let reach = |side: Side| {
                let on: Vec<&Access> = n
                    .access
                    .iter()
                    .filter(|a| a.sides.contains(&side))
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

/// The test point and connector pads of each board net, on the net itself
fn direct_access<'l>(cx: &Ctx, layout: &'l Layout) -> BTreeMap<&'l str, Vec<Access>> {
    let mut out: BTreeMap<&str, Vec<Access>> = BTreeMap::new();
    for pad in &layout.pads {
        let Some(name) = pad.net_name.as_deref().filter(|n| !n.is_empty()) else {
            continue;
        };
        let fp = layout.footprint(pad.footprint);
        let class = fp.part.map(|p| (p, cx.classes().get(p)));
        let kind = match class {
            Some((p, c)) if is_access(cx.classes(), p) => {
                if c == PartClass::TestPoint {
                    AccessKind::TestPoint
                } else {
                    AccessKind::Connector
                }
            }
            _ if pad.property.as_deref() == Some(TESTPOINT_PROPERTY) => AccessKind::TestPoint,
            _ => continue,
        };
        let sides = pad_sides(pad);
        if sides.is_empty() {
            continue;
        }
        out.entry(name).or_default().push(Access {
            reference: fp.reference.clone(),
            pad: pad.number.clone(),
            kind,
            sides,
            series: None,
        });
    }
    out
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
                format!("{}.{} ({}){series}", a.reference, a.pad, sides.join("+"))
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
    use erc_model::layout::Pt;

    fn pad(layers: &[&str]) -> Pad {
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
