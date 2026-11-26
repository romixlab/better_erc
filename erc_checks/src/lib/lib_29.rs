//! `LIB-29` Placed symbol carrying a `ki_` field

use crate::meta::*;
use crate::{Ctx, Finding};

pub struct Lib29;

static META: CheckMeta = CheckMeta {
    id: "LIB-29",
    category: "LIB",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

/// The prefix of the properties KiCad keeps in the library symbol (`ki_keywords`, `ki_fp_filters`,
/// `ki_description`, `ki_locked`)
const LIBRARY_PREFIX: &str = "ki_";

impl Check for Lib29 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A part whose placed symbol has fields named `ki_...`: the library copy's own properties are not part of
    /// the placed symbol, so the netlist carries them as ordinary fields and the board's footprint never matches.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        cx.components()
            .filter_map(|p| {
                let part = b.part(p);
                let names: Vec<&str> = part
                    .fields
                    .iter()
                    .map(|f| f.name.as_str())
                    .filter(|n| n.starts_with(LIBRARY_PREFIX))
                    .collect();
                if names.is_empty() {
                    return None;
                }
                Some(
                    cx.finding(&META)
                        .part(p)
                        .message(format!(
                            "{} carries the field{} {}: a `ki_` field belongs only in the library symbol, and on a \
                             placed symbol KiCad's schematic parity reports it as missing in the footprint \
                             (footprint_symbol_field_mismatch), which Update PCB from Schematic never clears",
                            part.designator,
                            if names.len() == 1 { "" } else { "s" },
                            names.join(", ")
                        ))
                        .fix(
                            "delete the field in Symbol Properties on the placed symbol (the library symbol keeps \
                             it), then Update PCB from Schematic",
                        )
                        .build(),
                )
            })
            .collect()
    }
}

inventory::submit!(Registered(&Lib29));

#[cfg(test)]
mod tests {
    // U1: no `ki_` field. U2: `ki_keywords` and `ki_fp_filters`. U3: `ki_description` and a user field. All three
    // library symbols carry `ki_keywords` and `ki_fp_filters`, which never count.
    const SCHEMATIC: &str = "lib-29_placed_ki_fields";

    #[test]
    fn placed_ki_fields() {
        assert_eq!(
            crate::testing::fingerprints(&super::Lib29, SCHEMATIC),
            ["LIB-29:part U2", "LIB-29:part U3"]
        );
    }
}
