//! Compares two layouts' placements: each footprint by its reference, its position, rotation, side, value and
//! package. The layouts can come from a board file or from a pick-and-place file (`erc_import`'s PnP importer), so
//! two PnP files, two boards, or a board and the PnP file made from it compare the same way. It is the PnP compare of
//! `berc diff --pnp`.

use crate::canon::natural_cmp;
use crate::design::Board;
use crate::layout::{Footprint, Layout, Pt, Side};
use std::collections::HashMap;

/// How close two placements must be to count as equal
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlaceOptions {
    /// Largest difference in x or y, in nanometres
    pub position_tolerance: i64,
    /// Largest difference in rotation, in degrees
    pub rotation_tolerance: f64,
}

/// One attribute of a placement that differs, as `a` and `b` have it
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
pub struct Moved {
    pub reference: String,
    pub changes: Vec<PlaceChange>,
}

/// The differences between two placements; every list in natural order of the references
#[derive(Clone, Debug, Default, PartialEq)]
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

/// Compares the placements of two boards (from a board file or a PnP file); `None` when one has no layout
pub fn compare_boards(a: &Board, b: &Board, opts: &PlaceOptions) -> Option<PlaceDiff> {
    Some(compare(a.layout.as_ref()?, b.layout.as_ref()?, opts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::FootprintAttrs;

    fn fp(reference: &str, x: f64, rot: f64, side: Side, value: &str, lib: &str) -> Footprint {
        Footprint {
            reference: reference.into(),
            part: None,
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
            rotation_tolerance: 0.0,
        };
        assert_eq!(compare(&a, &b, &loose).changed[0].changes.len(), 4);
        assert!(compare(&a, &a, &PlaceOptions::default()).is_empty());
    }
}
