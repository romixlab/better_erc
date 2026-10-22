//! `BOM-11` Same value in two packages

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::package::Package;
use erc_analysis::value::format_si;
use erc_model::{PartId, natural_cmp};
use std::collections::BTreeMap;

pub struct Bom11;

static META: CheckMeta = CheckMeta {
    id: "BOM-11",
    category: "BOM",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values],
    overlaps: &[],
    params: &[],
};

impl Check for Bom11 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Fitted parts whose value reads as a quantity (resistors, capacitors, inductors, ferrites, crystals, fuses),
    /// grouped by class and value; a value in more than one package (the case size, else the footprint name) is a
    /// finding
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let values = cx.values();
        // (class, value) -> package -> parts
        type Packages = BTreeMap<String, Vec<PartId>>;
        let mut groups: BTreeMap<(&str, String), (String, Packages)> = BTreeMap::new();
        for p in cx.view().fitted_components() {
            let class = classes.get(p);
            let v = values.get(p);
            if class.value_unit().is_none() || v.expected != v.unit {
                continue;
            }
            let (Some(q), Some(pkg)) = (v.quantity(), Package::of(b, p)) else {
                continue;
            };
            groups
                .entry((class.name(), format!("{:.6e}", q.value)))
                .or_insert_with(|| (format_si(q.value, q.unit), BTreeMap::new()))
                .1
                .entry(pkg.short().to_string())
                .or_default()
                .push(p);
        }
        let mut out = Vec::new();
        for ((class, _), (value, packages)) in groups {
            if packages.len() < 2 {
                continue;
            }
            let mut by_count: Vec<(String, Vec<PartId>)> = packages.into_iter().collect();
            for (_, ps) in &mut by_count {
                ps.sort_by(|a, c| natural_cmp(&b.part(*a).designator, &b.part(*c).designator));
            }
            by_count.sort_by(|a, c| {
                c.1.len()
                    .cmp(&a.1.len())
                    .then_with(|| natural_cmp(&a.0, &c.0))
            });
            let shown: Vec<String> = by_count
                .iter()
                .map(|(pkg, ps)| {
                    let names: Vec<&str> =
                        ps.iter().map(|p| b.part(*p).designator.as_str()).collect();
                    format!("{} × {pkg} ({})", ps.len(), names.join(", "))
                })
                .collect();
            out.push(
                cx.finding(&META)
                    .parts(by_count.iter().flat_map(|(_, ps)| ps.iter().copied()))
                    .message(format!(
                        "{class} {value} in {} packages: {}",
                        by_count.len(),
                        shown.join(", ")
                    ))
                    .fix(format!(
                        "move the minority to {} unless a power, voltage or DC-bias rating needs the bigger case",
                        by_count[0].0
                    ))
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Bom11));

#[cfg(test)]
mod tests {
    #[test]
    fn value_in_two_packages() {
        // C1..C3 100n 0402, C4 100n 0603; R1, R2 10k both 0402; C5 1u 0603 alone; R3 10k 0805 is DNP
        assert_eq!(
            crate::testing::fingerprints(&super::Bom11, "bom-11_packages"),
            ["BOM-11:part C1|part C2|part C3|part C4"]
        );
    }
}
