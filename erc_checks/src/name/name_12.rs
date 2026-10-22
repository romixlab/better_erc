//! `NAME-12` Unannotated or duplicate reference designators

use crate::meta::*;
use crate::{Ctx, Finding};
use std::collections::BTreeMap;

pub struct Name12;

static META: CheckMeta = CheckMeta {
    id: "NAME-12",
    category: "NAME",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

impl Check for Name12 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Designators ending in `?` (never annotated), and a designator placed twice for one unit: the importer
    /// joins symbols of one designator into one part, so two placements of the same unit are two parts sharing it
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for p in cx.components() {
            let part = b.part(p);
            if part.designator.ends_with('?') {
                out.push(
                    cx.finding(&META)
                        .part(p)
                        .message(format!(
                            "{} ({}) is not annotated",
                            part.designator,
                            b.field(p, "Value", cx.variant).unwrap_or(&part.lib_id)
                        ))
                        .fix("annotate the schematic (keep existing numbers on later revisions)")
                        .build(),
                );
                continue;
            }
            let mut units: BTreeMap<u32, Vec<String>> = BTreeMap::new();
            for u in &part.units {
                units.entry(u.unit).or_default().push(b.sheet_path(u.sheet));
            }
            for (unit, sheets) in units {
                if sheets.len() < 2 {
                    continue;
                }
                let unit = if part.units.iter().any(|u| u.unit != unit) {
                    format!(" unit {unit}")
                } else {
                    String::new()
                };
                out.push(
                    cx.finding(&META)
                        .part(p)
                        .message(format!(
                            "{}{unit} is placed {} times (on {}): two parts share one designator",
                            part.designator,
                            sheets.len(),
                            sheets.join(", ")
                        ))
                        .fix("re-annotate the duplicates; a netlist or BOM merges them into one part")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Name12));

#[cfg(test)]
mod tests {
    #[test]
    fn unannotated_and_duplicates() {
        assert_eq!(
            crate::testing::fingerprints(&super::Name12, "name-12_designators"),
            ["NAME-12:part C5", "NAME-12:part R?"]
        );
    }
}
