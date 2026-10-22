//! The board layout: footprints with their pads, courtyards, silk and fab graphics, tracks, vias, zones, the
//! board outline and the stackup. Coordinates are integer nanometres (KiCad's internal unit), so geometry
//! compares exactly and the dump is deterministic. The whole layout comes from one board file, named in
//! [`Layout::file`]; attributes another source can contradict (a footprint's fields) are facts.

use crate::design::{Field, NetId, PartId};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A point in nanometres (schematic and board coordinates, y down)
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct Pt {
    pub x: i64,
    pub y: i64,
}

impl Pt {
    pub const fn new(x: i64, y: i64) -> Pt {
        Pt { x, y }
    }

    /// From millimetres as KiCad files write them, rounded to the nanometre
    pub fn from_mm(x: f64, y: f64) -> Pt {
        Pt {
            x: mm_to_nm(x),
            y: mm_to_nm(y),
        }
    }
}

impl fmt::Display for Pt {
    /// `10.16,-2.54` in millimetres
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", Nm(self.x), Nm(self.y))
    }
}

/// Millimetres to nanometres, rounded
pub fn mm_to_nm(mm: f64) -> i64 {
    (mm * 1e6).round() as i64
}

/// A length in nanometres that prints as millimetres without trailing zeros (`2540000` as `2.54`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nm(pub i64);

impl fmt::Display for Nm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let a = self.0.unsigned_abs();
        let int = a / 1_000_000;
        let frac = a % 1_000_000;
        if frac == 0 {
            write!(f, "{sign}{int}")
        } else {
            let s = format!("{frac:06}");
            write!(f, "{sign}{int}.{}", s.trim_end_matches('0'))
        }
    }
}

impl Nm {
    /// Parses `2.54`, `-0.1`, `3` (millimetres) exactly, without going through a float
    pub fn parse(text: &str) -> Option<i64> {
        let (neg, t) = match text.strip_prefix('-') {
            Some(t) => (true, t),
            None => (false, text),
        };
        let (int, frac) = t.split_once('.').unwrap_or((t, ""));
        if int.is_empty() && frac.is_empty() || frac.len() > 6 {
            return None;
        }
        if !int.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let int: i64 = if int.is_empty() { 0 } else { int.parse().ok()? };
        let frac: i64 = format!("{frac:0<6}").parse().ok()?;
        let v = int.checked_mul(1_000_000)?.checked_add(frac)?;
        Some(if neg { -v } else { v })
    }
}

macro_rules! id {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(pub u32);

        impl $name {
            pub fn index(self) -> usize {
                self.0 as usize
            }

            pub fn from_index(i: usize) -> Self {
                Self(i as u32)
            }
        }
    };
}

id!(
    /// A footprint on the board
    FootprintId
);
id!(
    /// A pad of a footprint
    PadId
);

/// A drawn shape on one layer
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    pub layer: String,
    pub geom: Geom,
    /// Stroke width; 0 for none
    pub width: i64,
    pub filled: bool,
}

/// Geometry of a shape
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Geom {
    Line(Pt, Pt),
    Rect(Pt, Pt),
    Circle {
        center: Pt,
        radius: i64,
    },
    Arc {
        start: Pt,
        mid: Pt,
        end: Pt,
    },
    Poly(Vec<Pt>),
    /// A Bézier curve by its four control points
    Bezier([Pt; 4]),
}

/// Front or back of the board
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Side {
    Front,
    Back,
}

/// A board layer from the layer table
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayerDef {
    /// KiCad's layer number (changed between KiCad 8 and 9; kept as the file writes it)
    pub ordinal: i32,
    /// Canonical name (`F.Cu`, `In1.Cu`, `F.SilkS`)
    pub name: String,
    /// `signal`, `power`, `mixed`, `jumper`, `user`
    pub kind: String,
    /// User-given name, when set
    pub user_name: Option<String>,
}

/// One layer of the stackup (copper, dielectric, mask, silk, paste)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StackupLayer {
    pub name: String,
    /// `copper`, `core`, `prepreg`, `Top Solder Mask`, ...
    pub kind: String,
    pub thickness: Option<i64>,
    pub material: Option<String>,
    pub epsilon_r: Option<f64>,
}

/// The footprint attributes KiCad writes in `(attr ...)` and as flags
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FootprintAttrs {
    /// `smd`, `through_hole`, or none
    pub mount: Option<String>,
    pub board_only: bool,
    pub exclude_from_pos_files: bool,
    pub exclude_from_bom: bool,
    pub allow_missing_courtyard: bool,
    pub dnp: bool,
    pub allow_soldermask_bridges: bool,
}

/// A footprint placed on the board
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Footprint {
    pub reference: String,
    /// The schematic part with this designator, when the board belongs to a schematic
    pub part: Option<PartId>,
    pub lib_id: String,
    pub value: String,
    pub side: Side,
    pub at: Pt,
    /// Degrees, counter-clockwise
    pub rotation: f64,
    pub attrs: FootprintAttrs,
    /// Fields other than Reference and Value (`Footprint`, `Datasheet`, `Mpn1`, ...)
    pub fields: Vec<Field>,
    pub pads: Vec<PadId>,
    /// Courtyard, silk, fab and other graphics of the footprint, absolute coordinates
    pub graphics: Vec<Shape>,
    /// Texts of the footprint (reference and value included): layer, text, position
    pub texts: Vec<BoardText>,
    /// Pad groups connected inside the footprint (net ties)
    pub net_tie_pad_groups: Vec<Vec<String>>,
    /// 3D model paths
    pub models: Vec<String>,
}

/// A text on the board or in a footprint
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoardText {
    pub layer: String,
    pub text: String,
    pub at: Pt,
    /// Character height
    pub size: i64,
    pub hidden: bool,
    pub mirrored: bool,
}

/// A pad of a footprint
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pad {
    pub footprint: FootprintId,
    pub number: String,
    /// `smd`, `thru_hole`, `np_thru_hole`, `connect`
    pub kind: String,
    /// `rect`, `circle`, `oval`, `roundrect`, `trapezoid`, `chamfered_rect`, `custom`
    pub shape: String,
    /// Absolute position
    pub at: Pt,
    /// Absolute rotation, degrees
    pub rotation: f64,
    pub size: Pt,
    /// Drill diameter (x, y for an oval drill)
    pub drill: Option<Pt>,
    pub layers: Vec<String>,
    /// The board's net name for the pad
    pub net_name: Option<String>,
    /// The schematic net with that name, when the board belongs to a schematic
    pub net: Option<NetId>,
    /// `pad_prop_testpoint`, `pad_prop_fiducial_glob`, `pad_prop_heatsink`, ...
    pub property: Option<String>,
    pub pin_function: Option<String>,
    pub pin_type: Option<String>,
    pub solder_mask_margin: Option<i64>,
    pub solder_paste_margin: Option<i64>,
    pub solder_paste_ratio: Option<f64>,
    /// Zone connection override: 0 none, 1 thermal, 2 solid, 3 through-hole thermal only
    pub zone_connect: Option<i32>,
    /// Extra shapes of a custom pad, relative to the pad
    pub primitives: Vec<Shape>,
}

/// A track segment or arc
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub layer: String,
    pub start: Pt,
    /// The middle point of an arc track
    pub mid: Option<Pt>,
    pub end: Pt,
    pub width: i64,
    pub net_name: Option<String>,
}

/// A via
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Via {
    pub at: Pt,
    pub size: i64,
    pub drill: i64,
    /// The two copper layers it spans
    pub layers: Vec<String>,
    /// `through`, `blind`, `micro`
    pub kind: String,
    pub net_name: Option<String>,
}

/// A copper zone or a rule area
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub name: Option<String>,
    pub net_name: Option<String>,
    pub layers: Vec<String>,
    pub outline: Vec<Pt>,
    pub priority: u32,
    /// Pad connection: `thermal` (default), `solid`, `none`, `thru_hole_only`
    pub connect_pads: String,
    pub clearance: Option<i64>,
    pub min_thickness: Option<i64>,
    pub thermal_gap: Option<i64>,
    pub thermal_bridge_width: Option<i64>,
    /// A rule area: the keepouts it sets (`tracks`, `vias`, `pads`, `copperpour`, `footprints`), `not_allowed`
    pub keepout: Vec<(String, String)>,
    /// Whether it is filled (the fill itself is not kept)
    pub filled: bool,
}

/// The board as read from a `.kicad_pcb`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    /// The board file, relative to the project directory
    pub file: String,
    /// Board thickness
    pub thickness: Option<i64>,
    pub layers: Vec<LayerDef>,
    pub stackup: Vec<StackupLayer>,
    /// The board's net table (name per net code), as the board stores it
    pub net_names: Vec<String>,
    pub footprints: Vec<Footprint>,
    pub pads: Vec<Pad>,
    pub tracks: Vec<Track>,
    pub vias: Vec<Via>,
    pub zones: Vec<Zone>,
    /// Board-level graphics: outline (`Edge.Cuts`), silk, fab, user layers
    pub graphics: Vec<Shape>,
    pub texts: Vec<BoardText>,
    /// Design settings of the board file (`pad_to_mask_clearance`, ...), as text
    pub setup: Vec<(String, String)>,
}

impl Layout {
    pub fn footprint(&self, id: FootprintId) -> &Footprint {
        &self.footprints[id.index()]
    }

    pub fn pad(&self, id: PadId) -> &Pad {
        &self.pads[id.index()]
    }

    /// Board outline shapes (`Edge.Cuts`)
    pub fn outline(&self) -> impl Iterator<Item = &Shape> {
        self.graphics.iter().filter(|s| s.layer == "Edge.Cuts")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nm_round_trip() {
        for (nm, text) in [
            (2_540_000, "2.54"),
            (-100_000, "-0.1"),
            (3_000_000, "3"),
            (1, "0.000001"),
            (0, "0"),
        ] {
            assert_eq!(Nm(nm).to_string(), text);
            assert_eq!(Nm::parse(text), Some(nm));
        }
        assert_eq!(Nm::parse("1.0000001"), None);
        assert_eq!(Nm::parse("x"), None);
        assert_eq!(Pt::from_mm(10.16, -2.54).to_string(), "10.16,-2.54");
    }
}
