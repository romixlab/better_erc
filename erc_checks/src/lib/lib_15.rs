//! `LIB-15` Symbol with no power pins at all (hidden, or forgotten)

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::power::{NameRole, read_name};
use erc_model::EcadPinType;

pub struct Lib15;

static META: CheckMeta = CheckMeta {
    id: "LIB-15",
    category: "LIB",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Variant],
    overlaps: &[],
    params: &[],
};

/// A pin typed power, or named like a supply or ground (`VDD`, `VCC`, `V+`, `GND`, `VSS`)
fn power_pin(name: &str, etype: EcadPinType) -> bool {
    matches!(etype, EcadPinType::PowerIn | EcadPinType::PowerOut)
        || matches!(
            read_name(name, false).role,
            NameRole::Rail(_) | NameRole::Ground(_)
        )
}

impl Check for Lib15 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Fitted active parts (ICs, oscillators) whose symbol, over all its units, has no pin typed power and none
    /// named like a supply or ground; a part without a library symbol (a netlist) is judged by its pins
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let mut out = Vec::new();
        for p in cx.view().fitted_components() {
            if !classes.get(p).is_ic() {
                continue;
            }
            let part = b.part(p);
            let has_power = match part.symbol.map(|s| &b.symbols[s.index()]) {
                Some(sym) if !sym.pins.is_empty() => sym.pins.iter().any(|d| {
                    power_pin(&d.name, d.etype)
                        || d.alternates.iter().any(|a| power_pin(&a.name, a.etype))
                }),
                _ => part.pins.iter().any(|q| {
                    let pin = b.pin(*q);
                    power_pin(
                        &pin.name,
                        pin.etype.get().copied().unwrap_or(EcadPinType::Unspecified),
                    )
                }),
            };
            if has_power || part.pins.is_empty() {
                continue;
            }
            let class = classes.fact(p);
            out.push(
                cx.finding(&META)
                    .part(p)
                    .message(format!(
                        "{} ({}, {}): the symbol has no power pins, so no check sees its supply",
                        part.designator,
                        part.lib_id,
                        class.value.describe()
                    ))
                    .fact("class", class.value.describe(), class.source.clone())
                    .fix("draw the power pins, visible, in the symbol (or in a power unit, KLC S3.8)")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Lib15));

#[cfg(test)]
mod tests {
    #[test]
    fn no_power_pins() {
        // U2 has VCC and GND typed power, U3 VDD and GND by name, U4 hidden power pins; R1 is passive
        assert_eq!(
            crate::testing::fingerprints(&super::Lib15, "lib-15_no_power_pins"),
            ["LIB-15:part U1"]
        );
    }
}
