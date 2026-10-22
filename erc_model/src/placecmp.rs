//! Compares two layouts' placements: each footprint by its reference, its position, rotation, side, value and
//! package. The layouts can come from a board file or from a pick-and-place file (`erc_import`'s PnP importer), so
//! two PnP files, two boards, or a board and the PnP file made from it compare the same way. It is the PnP compare of
//! `berc diff --pnp`.
//!
//! A board file and a PnP file count in different frames: the board from the page's top left corner with y down,
//! a PnP file (KiCad's `pcb export pos`, Altium's and Allegro's) from an origin with y up. When one side is a board
//! and the other is not, [`compare_boards`] brings the board into the PnP frame first ([`to_placement_frame`]): from
//! the [`Origin`] the PnP file was exported with, y negated, rotations as they are, and without the footprints
//! marked `exclude_from_pos_files` (KiCad never writes those).

use crate::canon::natural_cmp;
use crate::design::Board;
use crate::layout::{Footprint, Layout, Nm, Pt, Side};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How close two placements must be to count as equal, and how a board meets a PnP file
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlaceOptions {
    /// Largest difference in x or y, in nanometres
    pub position_tolerance: i64,
    /// Largest difference in rotation, in degrees
    pub rotation_tolerance: f64,
    /// The origin the PnP file was exported from, when a board is compared with one
    pub origin: Origin,
}

/// Where a PnP file's coordinates start
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// The page's top left corner (KiCad's default)
    #[default]
    Page,
    /// The board's drill and place file origin (`aux_axis_origin`; KiCad's `--use-drill-file-origin`)
    Aux,
}

/// One attribute of a placement that differs, as `a` and `b` have it
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "attribute", rename_all = "snake_case")]
pub enum PlaceChange {
    Position {
        a: Pt,
        b: Pt,
    },
    /// Degrees, as the files give them (compared modulo 360)
    Rotation {
        a: f64,
        b: f64,
    },
    Side {
        a: Side,
        b: Side,
    },
    Value {
        a: String,
        b: String,
    },
    /// The footprint name, without a `library:` prefix
    Package {
        a: String,
        b: String,
    },
}

/// A footprint on both sides that is placed differently
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Moved {
    pub reference: String,
    pub changes: Vec<PlaceChange>,
}

/// The differences between two placements; every list in natural order of the references
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlaceDiff {
    pub changed: Vec<Moved>,
    /// In `a` only
    pub removed: Vec<String>,
    /// In `b` only
    pub added: Vec<String>,
}

impl PlaceDiff {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.removed.is_empty() && self.added.is_empty()
    }
}

/// The footprint name without its library (`Resistor_SMD:R_0603` → `R_0603`): a pick-and-place file names the
/// footprint only
fn package(f: &Footprint) -> &str {
    f.lib_id.rsplit(':').next().unwrap_or(&f.lib_id)
}

/// Compares the placements of `a` and `b`. Value and package are compared only where both sides have one (some
/// PnP files leave them out). A reference that repeats is paired with the same occurrence on the other side.
pub fn compare(a: &Layout, b: &Layout, opts: &PlaceOptions) -> PlaceDiff {
    let index = |l: &Layout| {
        let mut seen: HashMap<&str, usize> = HashMap::new();
        let mut map: HashMap<(String, usize), usize> = HashMap::new();
        for (i, f) in l.footprints.iter().enumerate() {
            let n = seen.entry(f.reference.as_str()).or_insert(0);
            map.insert((f.reference.clone(), *n), i);
            *n += 1;
        }
        map
    };
    let (ia, ib) = (index(a), index(b));
    let mut diff = PlaceDiff::default();
    for (key, &i) in &ia {
        let fa = &a.footprints[i];
        let Some(&j) = ib.get(key) else {
            diff.removed.push(fa.reference.clone());
            continue;
        };
        let fb = &b.footprints[j];
        let mut changes = Vec::new();
        if (fa.at.x - fb.at.x).abs() > opts.position_tolerance
            || (fa.at.y - fb.at.y).abs() > opts.position_tolerance
        {
            changes.push(PlaceChange::Position { a: fa.at, b: fb.at });
        }
        let d = (fa.rotation - fb.rotation).rem_euclid(360.0);
        if d.min(360.0 - d) > opts.rotation_tolerance {
            changes.push(PlaceChange::Rotation {
                a: fa.rotation,
                b: fb.rotation,
            });
        }
        if fa.side != fb.side {
            changes.push(PlaceChange::Side {
                a: fa.side,
                b: fb.side,
            });
        }
        if !fa.value.is_empty() && !fb.value.is_empty() && fa.value != fb.value {
            changes.push(PlaceChange::Value {
                a: fa.value.clone(),
                b: fb.value.clone(),
            });
        }
        let (pa, pb) = (package(fa), package(fb));
        if !pa.is_empty() && !pb.is_empty() && pa != pb {
            changes.push(PlaceChange::Package {
                a: pa.to_string(),
                b: pb.to_string(),
            });
        }
        if !changes.is_empty() {
            diff.changed.push(Moved {
                reference: fa.reference.clone(),
                changes,
            });
        }
    }
    for (key, &j) in &ib {
        if !ia.contains_key(key) {
            diff.added.push(b.footprints[j].reference.clone());
        }
    }
    diff.changed
        .sort_by(|x, y| natural_cmp(&x.reference, &y.reference));
    diff.removed.sort_by(|x, y| natural_cmp(x, y));
    diff.added.sort_by(|x, y| natural_cmp(x, y));
    diff
}

/// Compares the placements of two boards (from a board file or a PnP file), a board in the PnP frame when the
/// other is a PnP file; `None` when one has no layout
pub fn compare_boards(a: &Board, b: &Board, opts: &PlaceOptions) -> Option<PlaceDiff> {
    let (la, lb) = (a.layout.as_ref()?, b.layout.as_ref()?);
    Some(match (is_board_file(la), is_board_file(lb)) {
        (true, false) => compare(&to_placement_frame(la, opts.origin), lb, opts),
        (false, true) => compare(la, &to_placement_frame(lb, opts.origin), opts),
        _ => compare(la, lb, opts),
    })
}

/// Whether a layout was read from a board file (`.kicad_pcb`), in the board's frame, rather than a PnP file
pub fn is_board_file(layout: &Layout) -> bool {
    layout.file.ends_with(".kicad_pcb")
}

/// The board's drill and place file origin (`aux_axis_origin` in its setup; the page corner when not set)
pub fn aux_origin(layout: &Layout) -> Pt {
    layout
        .setup
        .iter()
        .find(|(k, _)| k == "aux_axis_origin")
        .and_then(|(_, v)| {
            let (x, y) = v.split_once(' ')?;
            Some(Pt {
                x: Nm::parse(x)?,
                y: Nm::parse(y)?,
            })
        })
        .unwrap_or_default()
}

/// A board's placements as a PnP file exported from it has them: relative to `origin`, y up, rotations as they are,
/// without the footprints excluded from position files. Only the footprints are kept.
pub fn to_placement_frame(layout: &Layout, origin: Origin) -> Layout {
    let o = match origin {
        Origin::Page => Pt::default(),
        Origin::Aux => aux_origin(layout),
    };
    let footprints = layout
        .footprints
        .iter()
        .filter(|f| !f.attrs.exclude_from_pos_files)
        .map(|f| Footprint {
            at: Pt {
                x: f.at.x - o.x,
                y: o.y - f.at.y,
            },
            ..f.clone()
        })
        .collect();
    Layout {
        file: layout.file.clone(),
        footprints,
        ..Layout::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::FootprintAttrs;

    fn fp(reference: &str, x: f64, rot: f64, side: Side, value: &str, lib: &str) -> Footprint {
        Footprint {
            reference: reference.into(),
            part: None,
            schematic_path: None,
            lib_id: lib.into(),
            value: value.into(),
            side,
            at: Pt::from_mm(x, 1.0),
            rotation: rot,
            attrs: FootprintAttrs::default(),
            fields: Vec::new(),
            pads: Vec::new(),
            graphics: Vec::new(),
            texts: Vec::new(),
            net_tie_pad_groups: Vec::new(),
            models: Vec::new(),
        }
    }

    fn layout(fps: Vec<Footprint>) -> Layout {
        Layout {
            footprints: fps,
            ..Layout::default()
        }
    }

    #[test]
    fn changes_by_reference() {
        let a = layout(vec![
            fp("R1", 1.0, 270.0, Side::Front, "10k", "Resistor_SMD:R_0603"),
            fp("R2", 2.0, 0.0, Side::Front, "10k", "R_0603"),
            fp("C1", 3.0, 0.0, Side::Back, "", ""),
        ]);
        let b = layout(vec![
            fp("R1", 1.0, -90.0, Side::Front, "10k", "R_0603"),
            fp("R2", 2.0001, 180.0, Side::Back, "22k", "R_0402"),
            fp("J1", 0.0, 0.0, Side::Front, "", ""),
        ]);
        let d = compare(&a, &b, &PlaceOptions::default());
        assert_eq!(d.removed, ["C1"]);
        assert_eq!(d.added, ["J1"]);
        assert_eq!(
            d.changed.len(),
            1,
            "R1: -90 is 270, the library prefix is not compared"
        );
        assert_eq!(d.changed[0].reference, "R2");
        assert_eq!(d.changed[0].changes.len(), 5);
        let loose = PlaceOptions {
            position_tolerance: 1000,
            ..PlaceOptions::default()
        };
        assert_eq!(compare(&a, &b, &loose).changed[0].changes.len(), 4);
        assert!(compare(&a, &a, &PlaceOptions::default()).is_empty());
    }

    /// What KiCad 10's `pcb export pos` writes for a board with its aux origin at (100, 50)
    #[test]
    fn board_in_the_placement_frame() {
        let mut board = layout(vec![
            fp("R1", 110.0, 90.0, Side::Front, "10k", "Resistor_SMD:R_0603"),
            fp("TP1", 130.0, 0.0, Side::Front, "", ""),
        ]);
        board.file = "b.kicad_pcb".into();
        board.footprints[0].at = Pt::from_mm(110.0, 100.0);
        board.footprints[1].attrs.exclude_from_pos_files = true;
        board
            .setup
            .push(("aux_axis_origin".into(), "100 50".into()));
        let page = to_placement_frame(&board, Origin::Page);
        assert_eq!(page.footprints.len(), 1, "TP1 is not in position files");
        assert_eq!(page.footprints[0].at, Pt::from_mm(110.0, -100.0));
        let aux = to_placement_frame(&board, Origin::Aux);
        assert_eq!(aux.footprints[0].at, Pt::from_mm(10.0, -50.0));
        assert_eq!(aux.footprints[0].rotation, 90.0);
    }
}
