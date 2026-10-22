//! `DRV-12` BJT base driven from a logic pin without a base resistor

use super::{BASE, EMITTER, logic_drivers, pin_named};
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_model::PartId;

pub struct Drv12;

static META: CheckMeta = CheckMeta {
    id: "DRV-12",
    category: "DRV",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// A pre-biased (digital) transistor with its base resistor built in: by part number (`DTC114`, `PDTC143`,
/// `MUN5211`, `RN1104`, `BCR108`) or by its description
pub(crate) fn pre_biased(cx: &Ctx, part: PartId) -> bool {
    let b = cx.board();
    let p = b.part(part);
    let desc = p
        .symbol
        .map(|s| b.symbols[s.index()].description.to_lowercase())
        .unwrap_or_default();
    let name = p.lib_id.split(':').nth(1).unwrap_or("");
    let texts = [
        name.to_uppercase(),
        cx.view().value_text(part).unwrap_or("").to_uppercase(),
        p.field("MPN").unwrap_or("").to_uppercase(),
        p.field("Mpn1").unwrap_or("").to_uppercase(),
    ];
    texts.iter().any(|t| {
        [
            "DTC", "DTA", "PDTC", "PDTA", "MUN5", "RN1", "RN2", "BCR1", "DDTC", "DDTA",
        ]
        .iter()
        .any(|pre| t.starts_with(pre))
    }) || [
        "pre-biased",
        "prebiased",
        "digital transistor",
        "resistor-equipped",
        "bias resistor",
        "built-in resistor",
    ]
    .iter()
    .any(|w| desc.contains(w))
}

impl Check for Drv12 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A fitted bipolar transistor (not pre-biased) with its emitter on a rail or ground (a switch, not a
    /// follower) whose base net holds an IC's output, bidirectional or tri-state pin: no base resistor
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for q in cx.view().fitted_components() {
            let c = cx.classes().get(q);
            if !(c.is_bjt() || c == erc_model::PartClass::Transistor) || pre_biased(cx, q) {
                continue;
            }
            let (Some((_, base)), Some((_, e))) =
                (pin_named(cx, q, BASE), pin_named(cx, q, EMITTER))
            else {
                continue;
            };
            if !power.is_power_net(e) || power.is_power_net(base) {
                continue;
            }
            let drivers = logic_drivers(cx, base);
            if drivers.is_empty() {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .part(q)
                    .net(base)
                    .message(format!(
                        "the base of {} is on {} with {} and no resistor: the base-emitter junction clamps the pin and draws its short-circuit current",
                        b.part(q).designator,
                        b.net(base).name,
                        pin_list(cx, &drivers)
                    ))
                    .fix("a base resistor sized for the collector current over the minimum gain (1-10 kΩ for small loads), or a pre-biased transistor (DTC, PDTC)")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Drv12));

#[cfg(test)]
mod tests {
    #[test]
    fn base_without_resistor() {
        let found = crate::testing::fingerprints(&super::Drv12, "drv-12_base_resistor");
        assert_eq!(found, ["DRV-12:net /BUZZ|part Q1"], "{found:?}");
    }
}
