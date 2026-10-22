//! `BOM-17` One MPN with two different values (a copy-paste part)

use super::mpns;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::{PartId, natural_cmp};
use std::collections::BTreeMap;

pub struct Bom17;

static META: CheckMeta = CheckMeta {
    id: "BOM-17",
    category: "BOM",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Values],
    overlaps: &[],
    params: &[],
};

impl Check for Bom17 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Parts grouped by every MPN they carry (`Mpn1`, `Mpn2`, ...); a group whose values differ once read (`4k7`
    /// and `4.7k` are one value) is a finding. DNP parts count: another variant may fit them.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let view = cx.view();
        let values = cx.values();
        let mut by_mpn: BTreeMap<String, (String, Vec<PartId>)> = BTreeMap::new();
        for p in cx.components() {
            for (_, mpn) in mpns(view, p) {
                let e = by_mpn
                    .entry(mpn.to_ascii_uppercase())
                    .or_insert_with(|| (mpn.to_string(), Vec::new()));
                if !e.1.contains(&p) {
                    e.1.push(p);
                }
            }
        }
        let mut out = Vec::new();
        for (mpn, parts) in by_mpn.into_values() {
            // value key -> (as written first, parts)
            let mut by_value: BTreeMap<String, (String, Vec<PartId>)> = BTreeMap::new();
            for p in &parts {
                let v = values.get(*p);
                let key = match v.quantity() {
                    Some(q) => format!("{:.6e} {}", q.value, q.unit.name()),
                    None => v
                        .text
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .to_uppercase(),
                };
                by_value
                    .entry(key)
                    .or_insert_with(|| (v.text.trim().to_string(), Vec::new()))
                    .1
                    .push(*p);
            }
            if by_value.len() < 2 {
                continue;
            }
            let mut groups: Vec<(String, Vec<PartId>)> = by_value.into_values().collect();
            for (_, ps) in &mut groups {
                ps.sort_by(|a, c| natural_cmp(&b.part(*a).designator, &b.part(*c).designator));
            }
            // The most used value first: the one the MPN was most likely chosen for
            groups.sort_by_key(|g| std::cmp::Reverse(g.1.len()));
            let shown: Vec<String> = groups
                .iter()
                .map(|(text, ps)| {
                    let names: Vec<&str> =
                        ps.iter().map(|p| b.part(*p).designator.as_str()).collect();
                    let text = if text.is_empty() { "(no value)" } else { text };
                    format!("{text} on {}", names.join(", "))
                })
                .collect();
            let mut f = cx
                .finding(&META)
                .parts(groups.iter().flat_map(|(_, ps)| ps.iter().copied()))
                .message(format!(
                    "MPN {mpn} carries {} different values: {}",
                    groups.len(),
                    shown.join("; ")
                ))
                .fix("correct the MPN on the parts whose value was edited");
            for (_, ps) in &groups {
                for p in ps {
                    if let Some(fact) = view.field_fact(*p, "Value") {
                        f = f.fact(
                            format!("{} Value", b.part(*p).designator),
                            &fact.value,
                            fact.source.clone(),
                        );
                    }
                }
            }
            out.push(f.build());
        }
        out
    }
}

inventory::submit!(Registered(&Bom17));

#[cfg(test)]
mod tests {
    #[test]
    fn one_mpn_two_values() {
        // R1 and R2 share RC0402FR-074K7L as 4k7 and 4.7k; R3 carries it as 100k; C1, C2 differ in MPN
        assert_eq!(
            crate::testing::fingerprints(&super::Bom17, "bom-17_mpn_values"),
            ["BOM-17:part R1|part R2|part R3"]
        );
    }
}
