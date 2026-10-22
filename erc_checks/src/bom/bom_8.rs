//! `BOM-8` LED swapped (polarity)

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_model::{NetId, PartId, PinId};
use std::collections::BTreeSet;

pub struct Bom8;

static META: CheckMeta = CheckMeta {
    id: "BOM-8",
    category: "BOM",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[Param {
        name: "max_hops",
        default: 2.0,
        unit: "parts",
        doc: "how many series resistors or ferrites the walk from an LED pin to a rail or ground goes through",
    }],
};

const ANODE: &[&str] = &["A", "AN", "ANODE", "+"];
const CATHODE: &[&str] = &["K", "C", "CA", "CATHODE", "-"];

/// The power nets a walk from `start` reaches: through fitted two-pin resistors and ferrites, `hops` deep, not
/// past a rail or ground. Returns the rails (positive or of unknown voltage) and the grounds and negative rails.
fn reach(cx: &Ctx, led: PartId, start: NetId, hops: usize) -> (BTreeSet<NetId>, BTreeSet<NetId>) {
    let b = cx.board();
    let power = cx.power();
    let classes = cx.classes();
    let (mut high, mut low) = (BTreeSet::new(), BTreeSet::new());
    let mut seen = BTreeSet::from([start]);
    let mut frontier = vec![start];
    for depth in 0..=hops {
        let mut next = Vec::new();
        for n in frontier {
            if power.is_ground(n) || power.volts(n).is_some_and(|v| v < 0.0) {
                low.insert(n);
                continue;
            }
            if power.is_rail(n) {
                high.insert(n);
                continue;
            }
            if depth == hops {
                continue;
            }
            for pin in net_pins(b, n) {
                let part = b.pin(pin).part;
                let class = classes.get(part);
                if part == led
                    || !cx.fitted(part)
                    || !(class.is_resistor() || class == erc_model::PartClass::FerriteBead)
                    || b.part(part).pins.len() != 2
                {
                    continue;
                }
                for other in &b.part(part).pins {
                    if let Some(m) = b.pin(*other).net
                        && seen.insert(m)
                    {
                        next.push(m);
                    }
                }
            }
        }
        frontier = next;
    }
    (high, low)
}

fn named(cx: &Ctx, pins: &[PinId], names: &[&str]) -> Vec<PinId> {
    pins.iter()
        .copied()
        .filter(|p| {
            let n = cx.board().pin(*p).name.trim();
            names.iter().any(|x| n.eq_ignore_ascii_case(x))
        })
        .collect()
}

impl Check for Bom8 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// For each fitted LED with pins named anode and cathode (`A`, `K`): an anode that leads (directly or through
    /// resistors and ferrites) only to ground or a negative rail, or a cathode that leads only to a rail, is
    /// turned round
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let hops = cx.param(&META, "max_hops") as usize;
        let mut out = Vec::new();
        for led in cx.view().fitted_components() {
            if !classes.get(led).is_led() {
                continue;
            }
            let pins = &b.part(led).pins;
            let anodes = named(cx, pins, ANODE);
            let cathodes = named(cx, pins, CATHODE);
            let mut wrong = Vec::new();
            let mut nets = BTreeSet::new();
            for a in &anodes {
                let Some(n) = b.pin(*a).net else { continue };
                let (high, low) = reach(cx, led, n, hops);
                if !low.is_empty() && high.is_empty() {
                    let names: Vec<&str> = low.iter().map(|n| b.net(*n).name.as_str()).collect();
                    wrong.push(format!(
                        "anode {} leads to {}",
                        b.pin(*a).number,
                        names.join(", ")
                    ));
                    nets.extend(low);
                }
            }
            for k in &cathodes {
                let Some(n) = b.pin(*k).net else { continue };
                let (high, low) = reach(cx, led, n, hops);
                if !high.is_empty() && low.is_empty() {
                    let names: Vec<&str> = high.iter().map(|n| b.net(*n).name.as_str()).collect();
                    wrong.push(format!(
                        "cathode {} leads to {}",
                        b.pin(*k).number,
                        names.join(", ")
                    ));
                    nets.extend(high);
                }
            }
            if wrong.is_empty() {
                continue;
            }
            let class = classes.fact(led);
            out.push(
                cx.finding(&META)
                    .part(led)
                    .nets(nets)
                    .message(format!(
                        "LED {} is reversed: {}",
                        b.part(led).designator,
                        wrong.join("; ")
                    ))
                    .fact("class", class.value.describe(), class.source.clone())
                    .fix("turn the LED round in the schematic, and check the footprint's pad 1 against the datasheet's cathode mark")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Bom8));

#[cfg(test)]
mod tests {
    #[test]
    fn reversed_leds() {
        // D1: +3V3, R1, anode, cathode, GND (right); D2: GPIO, R2, cathode, anode on GND; D3: cathode through R3 to
        // +3V3, anode on a GPIO; D4: anode on +5V, cathode through R4 to GND (right)
        assert_eq!(
            crate::testing::fingerprints(&super::Bom8, "bom-8_led_polarity"),
            ["BOM-8:net /+3V3|part D3", "BOM-8:net /GND|part D2"]
        );
    }
}
