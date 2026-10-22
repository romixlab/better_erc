//! `BOM-1` Resistors (or capacitors) with values a step apart that could be one value

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::value::format_si;
use erc_model::{ESeries, PartId, Unit, natural_cmp};
use std::collections::BTreeMap;

pub struct Bom1;

static META: CheckMeta = CheckMeta {
    id: "BOM-1",
    category: "BOM",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values],
    overlaps: &[],
    params: &[],
};

/// One value of a class on the board, with its parts
struct Used {
    value: f64,
    series: ESeries,
    parts: Vec<PartId>,
}

impl Check for Bom1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// The distinct values of the fitted resistors and of the fitted capacitors; two values one step apart in
    /// the smallest series that has both (33 Ω and 39 Ω in E12, 4.7 kΩ and 5.1 kΩ in E24, 1 µF and 2.2 µF in E3)
    /// are a finding, with the parts of both
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let values = cx.values();
        let mut by_class: BTreeMap<(&str, Unit), BTreeMap<String, Used>> = BTreeMap::new();
        for p in cx.view().fitted_components() {
            let class = classes.get(p);
            if !(class.is_resistor() || class.is_capacitor()) {
                continue;
            }
            let v = values.get(p);
            let (Some(q), Some(series)) = (v.quantity(), v.series) else {
                continue;
            };
            by_class
                .entry((class.name(), q.unit))
                .or_default()
                .entry(format!("{:.6e}", q.value))
                .or_insert_with(|| Used {
                    value: q.value,
                    series,
                    parts: Vec::new(),
                })
                .parts
                .push(p);
        }
        let mut out = Vec::new();
        for ((class, unit), used) in by_class {
            let mut used: Vec<Used> = used.into_values().collect();
            used.sort_by(|a, c| a.value.total_cmp(&c.value));
            for (i, a) in used.iter().enumerate() {
                for c in &used[i + 1..] {
                    let series = a.series.max(c.series);
                    if series.steps(a.value, c.value) != Some(1) {
                        continue;
                    }
                    let list = |u: &Used| {
                        let mut names: Vec<&str> = u
                            .parts
                            .iter()
                            .map(|p| b.part(*p).designator.as_str())
                            .collect();
                        names.sort_by(|x, y| natural_cmp(x, y));
                        format!("{} ({})", format_si(u.value, unit), names.join(", "))
                    };
                    out.push(
                        cx.finding(&META)
                            .parts(a.parts.iter().chain(&c.parts).copied())
                            .message(format!(
                                "{class}s {} and {} are one {} step apart: could they be one value?",
                                list(a),
                                list(c),
                                series.name()
                            ))
                            .fix("pick one value where the circuit allows (pull-ups, LED resistors, series terminations, bulk decoupling); leave dividers and filters alone")
                            .build(),
                    );
                }
            }
        }
        out
    }
}

inventory::submit!(Registered(&Bom1));

#[cfg(test)]
mod tests {
    #[test]
    fn values_one_step_apart() {
        // R1 33R, R2 39R (E12), R3 4k7, R4 5k1 (E24), R5 10k, R6 15k is DNP; C1 1u, C2 2u2 (E3), C3 100n
        assert_eq!(
            crate::testing::fingerprints(&super::Bom1, "bom-1_values_a_step_apart"),
            [
                "BOM-1:part C1|part C2",
                "BOM-1:part R1|part R2",
                "BOM-1:part R3|part R4"
            ]
        );
    }
}
