//! `BOM-20` MPN on the board but not in the parts snapshot

use super::mpns;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::{PartId, natural_cmp};
use std::collections::BTreeMap;

pub struct Bom20;

static META: CheckMeta = CheckMeta {
    id: "BOM-20",
    category: "BOM",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartsLock],
    overlaps: &[],
    params: &[],
};

impl Check for Bom20 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Every MPN field of the fitted parts in the variant checked, against the snapshot's parts: one finding per
    /// MPN the snapshot lacks, on the parts carrying it. The checks that read part facts judge such an MPN on
    /// nothing, so it would slip through unreviewed.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let Some(lock) = cx.parts() else {
            return Vec::new();
        };
        let b = cx.board();
        let view = cx.view();
        let mut missing: BTreeMap<&str, Vec<PartId>> = BTreeMap::new();
        for p in view.fitted_components() {
            for (_, mpn) in mpns(view, p) {
                if lock.part(mpn).is_none() {
                    let parts = missing.entry(mpn).or_default();
                    if !parts.contains(&p) {
                        parts.push(p);
                    }
                }
            }
        }
        missing
            .into_iter()
            .map(|(mpn, mut parts)| {
                parts.sort_by(|a, c| natural_cmp(&b.part(*a).designator, &b.part(*c).designator));
                let names: Vec<&str> = parts
                    .iter()
                    .map(|p| b.part(*p).designator.as_str())
                    .collect();
                cx.finding(&META)
                    .parts(parts.iter().copied())
                    .message(format!(
                        "MPN {mpn} on {} is not in the parts snapshot: its facts and footprint reviews are not checked",
                        names.join(", ")
                    ))
                    .fix("run `berc parts refresh` and commit parts.lock.toml")
                    .build()
            })
            .collect()
    }
}

inventory::submit!(Registered(&Bom20));
