//! `DBG-7` Suggest DNM pull-up and down on EN pins

use super::named_pins;
use crate::meta::*;
use crate::text::{designators, pin_list};
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::pin_name::PinFunction;
use erc_model::{NetId, PartId};

pub struct Dbg7;

static META: CheckMeta = CheckMeta {
    id: "DBG-7",
    category: "DBG",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Dbg7 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// An enable pin of a fitted IC (`EN`, `ON`, `SHDN` by name) without resistor footprints, fitted or not,
    /// to both a rail and a ground: tied to a rail, driven from one place, or pulled one way only
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for part in cx.view().fitted_components() {
            if !cx.classes().get(part).is_ic() && !cx.classes().get(part).is_programmable() {
                continue;
            }
            let mut seen = Vec::new();
            for (pin, f) in named_pins(cx, part) {
                if f.function != PinFunction::Enable || seen.contains(&pin) {
                    continue;
                }
                seen.push(pin);
                let Some(net) = b.pin(pin).net else { continue };
                if power.is_ground(net) {
                    continue;
                }
                let how = if power.is_rail(net) {
                    format!("tied to {}", b.net(net).name)
                } else {
                    let (up, down) = pulls(cx, net);
                    if !up.is_empty() && !down.is_empty() {
                        continue;
                    }
                    let drivers: Vec<_> = net_pins(b, net)
                        .filter(|p| *p != pin && !cx.classes().get(b.pin(*p).part).is_resistor())
                        .collect();
                    match (up.is_empty(), down.is_empty()) {
                        (false, _) => format!(
                            "pulled up by {} with no footprint to pull it down",
                            designators(cx, up)
                        ),
                        (_, false) => format!(
                            "pulled down by {} with no footprint to pull it up",
                            designators(cx, down)
                        ),
                        _ if drivers.is_empty() => "left with no pull either way".into(),
                        _ => format!("driven by {} only", pin_list(cx, &drivers)),
                    }
                };
                out.push(
                    cx.finding(&META)
                        .pin(pin)
                        .net(net)
                        .message(format!("{} {how}", pin_list(cx, &[pin])))
                        .fix("drive EN through a resistor (or from a GPIO with a pull) and add a DNM resistor to the other level")
                        .build(),
                );
            }
        }
        out
    }
}

/// The resistors (fitted or not) from `net` to a rail and to a ground
fn pulls(cx: &Ctx, net: NetId) -> (Vec<PartId>, Vec<PartId>) {
    let b = cx.board();
    let power = cx.power();
    let (mut up, mut down) = (Vec::new(), Vec::new());
    for pin in net_pins(b, net) {
        let r = b.pin(pin).part;
        if !cx.classes().get(r).is_resistor() || b.part(r).pins.len() != 2 {
            continue;
        }
        let far = b
            .part(r)
            .pins
            .iter()
            .filter(|p| **p != pin)
            .find_map(|p| b.pin(*p).net);
        match far {
            Some(n) if power.is_rail(n) => up.push(r),
            Some(n) if power.is_ground(n) => down.push(r),
            _ => {}
        }
    }
    (up, down)
}

inventory::submit!(Registered(&Dbg7));

#[cfg(test)]
mod tests {
    #[test]
    fn enable_without_override() {
        let found = crate::testing::fingerprints(&super::Dbg7, "dbg-7_enable_pulls");
        assert_eq!(
            found,
            [
                "DBG-7:net +5V|pin U1.4",
                "DBG-7:net /EN_U2|pin U2.4",
                "DBG-7:net /EN_U3|pin U3.4"
            ],
            "{found:?}"
        );
    }
}
