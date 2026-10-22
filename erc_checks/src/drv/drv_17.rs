//! `DRV-17` LED without a series resistor or current limit

use super::{designators, main_terminals};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_model::{EcadPinType, NetId, PartId};
use std::collections::BTreeSet;

pub struct Drv17;

static META: CheckMeta = CheckMeta {
    id: "DRV-17",
    category: "DRV",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// An IC that sets an LED's current: an LED driver, a constant-current sink or regulator, by its description
fn current_limit(cx: &Ctx, part: PartId) -> bool {
    let b = cx.board();
    let desc = b
        .part(part)
        .symbol
        .map(|s| b.symbols[s.index()].description.to_lowercase())
        .unwrap_or_default();
    [
        "led driver",
        "constant current",
        "constant-current",
        "current sink",
        "current source",
        "current regulator",
        "current limit",
    ]
    .iter()
    .any(|w| desc.contains(w))
}

/// A fitted two-pin LED
fn led(cx: &Ctx, part: PartId) -> bool {
    cx.fitted(part) && cx.classes().get(part).is_led() && cx.board().part(part).pins.len() == 2
}

impl Check for Drv17 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A string of fitted two-pin LEDs (one, or several through nets that hold only their pins) whose both ends
    /// sit straight on a stiff node: a rail, a ground, an IC pin that is not a supply, or a transistor's main
    /// terminal; no resistor and no LED driver or current sink anywhere in the chain
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut done: BTreeSet<PartId> = BTreeSet::new();
        let mut out = Vec::new();
        for start in cx.view().fitted_components() {
            if !led(cx, start) || done.contains(&start) {
                continue;
            }
            // Walk the string both ways through nets of exactly two LED pins
            let mut string: BTreeSet<PartId> = BTreeSet::from([start]);
            let mut ends: Vec<NetId> = Vec::new();
            for first in b.part(start).pins.iter().filter_map(|p| b.pin(*p).net) {
                let (mut at, mut from) = (first, start);
                loop {
                    let pins: Vec<_> = net_pins(b, at).collect();
                    let next = (pins.len() == 2 && !power.is_power_net(at))
                        .then(|| pins.iter().map(|p| b.pin(*p).part).find(|p| *p != from))
                        .flatten()
                        .filter(|p| led(cx, *p) && !string.contains(p));
                    let Some(next) = next else {
                        ends.push(at);
                        break;
                    };
                    string.insert(next);
                    let Some(far) = b
                        .part(next)
                        .pins
                        .iter()
                        .filter_map(|p| b.pin(*p).net)
                        .find(|n| *n != at)
                    else {
                        ends.push(at);
                        break;
                    };
                    (at, from) = (far, next);
                }
            }
            done.extend(string.iter().copied());
            let stiff = |net: NetId| {
                power.is_power_net(net)
                    || net_pins(b, net).any(|p| {
                        let pin = b.pin(p);
                        let part = pin.part;
                        if string.contains(&part) || !cx.fitted(part) {
                            return false;
                        }
                        let c = cx.classes().get(part);
                        (c.is_ic()
                            && !current_limit(cx, part)
                            && pin.etype.get() != Some(&EcadPinType::PowerIn))
                            || (c.is_transistor()
                                && main_terminals(cx, part)
                                    .is_some_and(|(x, y)| x == net || y == net))
                    })
            };
            let limited = ends.iter().any(|n| {
                net_pins(b, *n).any(|p| {
                    let part = b.pin(p).part;
                    cx.fitted(part) && cx.classes().get(part).is_ic() && current_limit(cx, part)
                })
            });
            if ends.len() != 2 || limited || !ends.iter().all(|n| stiff(*n)) {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .parts(string.iter().copied())
                    .message(format!(
                        "{} {} between {} and {} with no series resistor or current limit: only the source's own resistance sets the current",
                        designators(cx, string.iter().copied()),
                        if string.len() == 1 { "sits" } else { "sit in series" },
                        b.net(ends[0]).name,
                        b.net(ends[1]).name
                    ))
                    .fix("a series resistor (V - Vf) / I, or an LED driver")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Drv17));

#[cfg(test)]
mod tests {
    #[test]
    fn leds_without_resistor() {
        let found = crate::testing::fingerprints(&super::Drv17, "drv-17_led_resistor");
        assert_eq!(
            found,
            ["DRV-17:part D1", "DRV-17:part D2|part D3"],
            "{found:?}"
        );
    }
}
