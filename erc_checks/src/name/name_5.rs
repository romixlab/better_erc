//! `NAME-5` Gaps in designator series on rev A or standalone

use super::{Numbering, numbered_parts, numbering};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::{PartId, Provenance};
use std::collections::BTreeMap;

pub struct Name5;

static META: CheckMeta = CheckMeta {
    id: "NAME-5",
    category: "NAME",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

/// Whether a title block revision is a first one: `A`, `A1`, `A.2`, `Rev A`, `1`, `1.0`, `V1`, `0.3`
pub(crate) fn first_revision(rev: &str) -> bool {
    let r = rev.trim().to_uppercase();
    let r = r
        .strip_prefix("REV")
        .unwrap_or(&r)
        .trim_start_matches(['.', ' ', ':']);
    let r = r.strip_prefix('V').unwrap_or(r);
    if let Some(rest) = r.strip_prefix('A') {
        return rest.chars().all(|c| c.is_ascii_digit() || c == '.');
    }
    r.parse::<f64>().is_ok_and(|v| v < 2.0)
}

impl Check for Name5 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Missing numbers inside each designator series (per sheet block when the board numbers per sheet, NAME-6),
    /// on a board whose root title block gives a first revision or none; later revisions keep their gaps
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let rev = b.sheets.first().and_then(|s| s.rev.clone());
        if rev.as_deref().is_some_and(|r| !first_revision(r)) {
            return Vec::new();
        }
        let scheme = numbering(cx);
        let parts = numbered_parts(cx);
        // Numbered flat but every number of a prefix 100 or above: blocks of a hundred all the same (a sheet of a
        // per-sheet design read alone)
        let lowest = |prefix: &str| {
            parts
                .iter()
                .filter(|(_, x, _, _)| *x == prefix)
                .map(|(_, _, n, _)| *n)
                .min()
                .unwrap_or(0)
        };
        let mut series: BTreeMap<(String, u32), BTreeMap<u32, PartId>> = BTreeMap::new();
        for (p, prefix, n, _) in parts.iter().copied() {
            let block = match scheme {
                Numbering::Flat if lowest(prefix) >= 100 => n / 100,
                Numbering::Flat => 0,
                Numbering::PerSheet { base } => n / base,
            };
            series
                .entry((prefix.to_string(), block))
                .or_default()
                .insert(n, p);
        }
        let mut out = Vec::new();
        for ((prefix, _), parts) in series {
            let numbers: Vec<u32> = parts.keys().copied().collect();
            let mut missing = Vec::new();
            let mut around = Vec::new();
            for w in numbers.windows(2) {
                if w[1] > w[0] + 1 {
                    missing.push(if w[1] == w[0] + 2 {
                        format!("{prefix}{}", w[0] + 1)
                    } else {
                        format!("{prefix}{}-{prefix}{}", w[0] + 1, w[1] - 1)
                    });
                    around.extend([parts[&w[0]], parts[&w[1]]]);
                }
            }
            if missing.is_empty() {
                continue;
            }
            let first = numbers[0];
            let last = numbers[numbers.len() - 1];
            out.push(
                cx.finding(&META)
                    .parts(around)
                    .message(format!(
                        "{prefix}{first}-{prefix}{last}: {} missing",
                        missing.join(", ")
                    ))
                    .fact(
                        "revision",
                        rev.as_deref().unwrap_or("not stated"),
                        Provenance::Schematic {
                            file: b.sheets.first().map(|s| s.file.clone()).unwrap_or_default(),
                        },
                    )
                    .fix("re-annotate before the first release; afterwards keep gaps and note removals in the change list")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Name5));

#[cfg(test)]
mod tests {
    use super::first_revision;

    #[test]
    fn revisions() {
        for r in [
            "A", "a", "A1", "A.2", "Rev A", "REV.A", "1", "1.0", "V1", "v1.2", "0.3",
        ] {
            assert!(first_revision(r), "{r}");
        }
        for r in ["B", "Rev C", "2", "2.0", "V3", "B1"] {
            assert!(!first_revision(r), "{r}");
        }
    }

    #[test]
    fn gaps() {
        assert_eq!(
            crate::testing::fingerprints(&super::Name5, "name-5_designator_gaps"),
            ["NAME-5:part R2|part R4"]
        );
    }
}
