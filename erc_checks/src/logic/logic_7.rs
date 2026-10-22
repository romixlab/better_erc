//! `LOGIC-7` Open-drain or open-collector output as the only driver of a net with no pull-up: never goes high

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_model::{EcadPinType, NetId, PartClass, PinId};
use std::collections::BTreeSet;

pub struct Logic7;

static META: CheckMeta = CheckMeta {
    id: "LOGIC-7",
    category: "LOGIC",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[Param {
        name: "max_hops",
        default: 2.0,
        unit: "parts",
        doc: "how many resistors, diodes or ferrites in series a pull-up may go through to its rail (an LED and its resistor)",
    }],
};

/// Whether `start` reaches a net `target` accepts through fitted two-pin resistors, diodes (LEDs), ferrites and
/// inductors, `hops` deep
fn pulled(cx: &Ctx, start: NetId, hops: usize, target: &dyn Fn(NetId) -> bool) -> bool {
    let b = cx.board();
    let classes = cx.classes();
    let mut seen = BTreeSet::from([start]);
    let mut frontier = vec![start];
    for _ in 0..hops {
        let mut next = Vec::new();
        for n in frontier {
            for pin in net_pins(b, n) {
                let part = b.pin(pin).part;
                let c = classes.get(part);
                if !cx.fitted(part)
                    || b.part(part).pins.len() != 2
                    || !(c.is_resistor()
                        || c.is_diode()
                        || matches!(c, PartClass::FerriteBead | PartClass::Inductor))
                {
                    continue;
                }
                for other in &b.part(part).pins {
                    let Some(m) = b.pin(*other).net else { continue };
                    if m == n || !seen.insert(m) {
                        continue;
                    }
                    if target(m) {
                        return true;
                    }
                    next.push(m);
                }
            }
        }
        frontier = next;
    }
    false
}

impl Check for Logic7 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Signal nets whose output-capable pins on fitted parts are all open collector (open drain) or all open emitter
    /// (KiCad pin types), with a pin of another part to receive, and no resistor (or LED and resistor) up to a rail for
    /// open collectors, down to ground for open emitters. A bidirectional pin is not counted as a driver: its
    /// direction (and internal pull) needs pin roles (`NET-7`)
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let hops = cx.param(&META, "max_hops") as usize;
        let mut out = Vec::new();
        for n in (0..b.nets.len()).map(NetId::from_index) {
            if power.is_power_net(n) {
                continue;
            }
            let pins: Vec<PinId> = net_pins(b, n)
                .filter(|p| cx.fitted(b.pin(*p).part))
                .collect();
            let ty = |p: &PinId| b.pin(*p).etype.get().copied();
            let of = |t: EcadPinType| -> Vec<PinId> {
                pins.iter().copied().filter(|p| ty(p) == Some(t)).collect()
            };
            let (oc, oe) = (of(EcadPinType::OpenCollector), of(EcadPinType::OpenEmitter));
            let push_pull = pins.iter().any(|p| {
                matches!(
                    ty(p),
                    Some(EcadPinType::Output | EcadPinType::TriState | EcadPinType::PowerOut)
                )
            });
            if push_pull || (oc.is_empty() == oe.is_empty()) {
                continue;
            }
            let (drivers, up) = if oc.is_empty() {
                (oe, false)
            } else {
                (oc, true)
            };
            let driver_parts: BTreeSet<_> = drivers.iter().map(|p| b.pin(*p).part).collect();
            let receivers = pins
                .iter()
                .any(|p| !driver_parts.contains(&b.pin(*p).part) && !drivers.contains(p));
            if !receivers {
                continue;
            }
            let target = |m: NetId| {
                if up {
                    power.is_rail(m) && power.volts(m).is_none_or(|v| v > 0.0)
                } else {
                    power.is_ground(m)
                }
            };
            if pulled(cx, n, hops, &target) {
                continue;
            }
            let shown: Vec<String> = drivers.iter().map(|p| b.pin_name(*p)).collect();
            let (kind, missing, fix) = if up {
                (
                    "open-collector (open-drain)",
                    "pull-up",
                    "a pull-up to the receiver's rail, sized per IO-17",
                )
            } else {
                ("open-emitter", "pull-down", "a pull-down to ground")
            };
            out.push(
                cx.finding(&META)
                    .net(n)
                    .parts(driver_parts.iter().copied())
                    .message(format!(
                        "{} is driven only by {kind} output{} {} and has no {missing}: it floats when released",
                        b.net(n).name,
                        if drivers.len() > 1 { "s" } else { "" },
                        shown.join(", ")
                    ))
                    .fix(fix)
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Logic7));

#[cfg(test)]
mod tests {
    #[test]
    fn open_drain_without_pull_up() {
        // U1's open-collector OUT into U3 with no pull-up; U2's with R1 to +3V3; U4's through LED D1 and R2 to +3V3;
        // U5's output alone on its net
        assert_eq!(
            crate::testing::fingerprints(&super::Logic7, "logic-7_open_drain"),
            ["LOGIC-7:net /CMP1|part U1"]
        );
    }
}
