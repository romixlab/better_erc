//! `LIB-14` Unit of a multi-unit symbol not placed

use crate::meta::*;
use crate::{Ctx, Finding};
use std::collections::BTreeSet;

pub struct Lib14;

static META: CheckMeta = CheckMeta {
    id: "LIB-14",
    category: "LIB",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Variant],
    overlaps: &[],
    params: &[],
};

/// `A` for unit 1, `B` for 2: KiCad's letters when the symbol names no unit
fn unit_name(names: &[(u32, String)], unit: u32) -> String {
    names
        .iter()
        .find(|(u, _)| *u == unit)
        .map(|(_, n)| n.clone())
        .unwrap_or_else(|| {
            char::from_u32('A' as u32 + unit - 1)
                .filter(|c| c.is_ascii_uppercase())
                .map_or_else(|| unit.to_string(), String::from)
        })
}

impl Check for Lib14 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Fitted parts of a multi-unit symbol with a unit that is not placed: the symbol's pins of that unit whose
    /// numbers no placed unit has float on the board
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for p in cx.view().fitted_components() {
            let part = b.part(p);
            let Some(sym) = part.symbol.map(|s| &b.symbols[s.index()]) else {
                continue;
            };
            if sym.units < 2 {
                continue;
            }
            let placed: BTreeSet<u32> = part
                .units
                .iter()
                .map(|u| u.unit)
                .chain(part.pins.iter().map(|q| b.pin(*q).unit))
                .collect();
            let numbers: BTreeSet<&str> = part
                .pins
                .iter()
                .map(|q| b.pin(*q).number.as_str())
                .collect();
            let mut missing = Vec::new();
            for unit in 1..=sym.units {
                if placed.contains(&unit) {
                    continue;
                }
                let floating: Vec<String> = sym
                    .pins
                    .iter()
                    .filter(|d| d.unit == unit && !numbers.contains(d.number.as_str()))
                    .map(|d| {
                        if d.name.is_empty() || d.name == "~" {
                            d.number.clone()
                        } else {
                            format!("{} {}", d.number, d.name)
                        }
                    })
                    .collect();
                if !floating.is_empty() {
                    missing.push(format!(
                        "unit {} (pins {})",
                        unit_name(&sym.unit_names, unit),
                        floating.join(", ")
                    ));
                }
            }
            if missing.is_empty() {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .part(p)
                    .message(format!(
                        "{} ({}): {} not placed, its pins float on the board",
                        part.designator,
                        part.lib_id,
                        missing.join("; ")
                    ))
                    .fix("place every unit; tie unused gate inputs to a rail, wire unused op-amps as followers to a valid common-mode voltage")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Lib14));

#[cfg(test)]
mod tests {
    #[test]
    fn unplaced_units() {
        // U1 has units A and C (power), not B; U2 has all three; U3 lacks B too but is DNP
        assert_eq!(
            crate::testing::fingerprints(&super::Lib14, "lib-14_unplaced_units"),
            ["LIB-14:part U1"]
        );
    }
}
