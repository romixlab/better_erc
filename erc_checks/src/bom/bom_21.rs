//! `BOM-21` MPN whose maker is not settled: several parts to choose from and no `MfgN`, or an `MfgN` the part source
//! has no part of the MPN by

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::{PartId, natural_cmp};
use erc_parts::lock::{LockedPart, PartKey, PartMatch};
use erc_parts::mpn::mpn_entries;
use std::collections::BTreeMap;

pub struct Bom21;

static META: CheckMeta = CheckMeta {
    id: "BOM-21",
    category: "BOM",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartsLock],
    overlaps: &[],
    params: &[],
};

/// The parts carrying one MPN and maker, with the fields that name them on the first
struct Carriers<'a> {
    mpn_field: &'a str,
    maker_field: Option<&'a str>,
    parts: Vec<PartId>,
}

/// What is wrong with the snapshot's entry, for the message; `None` when the maker is settled
fn problem(key: &PartKey, c: &Carriers, part: &LockedPart) -> Option<(String, String)> {
    let makers = part.maker_names().join(", ");
    let mpn = &key.mpn;
    let mpn_field = c.mpn_field;
    let maker_field = c.maker_field.unwrap_or("Mfg");
    match (part.matched, &key.maker) {
        (PartMatch::NotUnique, None) => Some((
            format!(
                "MPN {mpn} ({mpn_field}) matches {} parts in the part source ({makers}): which one is meant is not \
                 said, so its facts go unchecked",
                part.makers.len()
            ),
            format!(
                "name the maker in a `{maker_field}` field beside `{mpn_field}`, or merge a maker-less stub into its part"
            ),
        )),
        (PartMatch::NotUnique, Some(m)) => Some((
            format!(
                "MPN {mpn} by `{m}` ({maker_field}) matches several parts in the part source: its facts go unchecked"
            ),
            "merge the duplicate parts in the part source".to_owned(),
        )),
        (PartMatch::NotFound, Some(m)) if !part.makers.is_empty() => Some((
            format!(
                "`{m}` ({maker_field}) makes no part {mpn} in the part source, which has it by {makers}"
            ),
            format!("write one of these makers in `{maker_field}`, or add the part by `{m}`"),
        )),
        (PartMatch::UnknownMaker, Some(m)) => {
            let known = if makers.is_empty() {
                String::new()
            } else {
                format!(" (MPN {mpn} is by {makers})")
            };
            Some((
                format!("`{m}` ({maker_field}) is no maker the part source knows{known}"),
                format!(
                    "write the maker's name as the part source has it in `{maker_field}`, or add it as a synonym"
                ),
            ))
        }
        _ => None,
    }
}

impl Check for Bom21 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Every MPN field of the fitted parts in the variant checked, with the maker beside it, against the
    /// snapshot's entry of that MPN and maker: one finding per MPN and maker whose part is not settled, on the
    /// parts carrying it. An MPN the snapshot lacks is `BOM-20`'s.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let Some(lock) = cx.parts() else {
            return Vec::new();
        };
        let b = cx.board();
        let view = cx.view();
        let mut by_key: BTreeMap<PartKey, Carriers> = BTreeMap::new();
        for p in view.fitted_components() {
            for e in mpn_entries(view, p) {
                let c = by_key.entry(e.key()).or_insert_with(|| Carriers {
                    mpn_field: e.field,
                    maker_field: e.maker.map(|(f, _)| f),
                    parts: Vec::new(),
                });
                if !c.parts.contains(&p) {
                    c.parts.push(p);
                }
            }
        }
        let mut out = Vec::new();
        for (key, mut c) in by_key {
            let Some(part) = lock.part_of(&key) else {
                continue;
            };
            let Some((message, fix)) = problem(&key, &c, part) else {
                continue;
            };
            c.parts
                .sort_by(|a, d| natural_cmp(&b.part(*a).designator, &b.part(*d).designator));
            let names: Vec<&str> = c
                .parts
                .iter()
                .map(|p| b.part(*p).designator.as_str())
                .collect();
            out.push(
                cx.finding(&META)
                    .parts(c.parts.iter().copied())
                    .message(format!("{}: {message}", names.join(", ")))
                    .fix(fix)
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Bom21));
