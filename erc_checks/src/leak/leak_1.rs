//! `LEAK-1` Resistor divider straight on a power rail

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::{net_pins, series_paths};
use erc_analysis::value::format_si;
use erc_model::{NetId, PartClass, PartId, Provenance, Unit};

pub struct Leak1;

static META: CheckMeta = CheckMeta {
    id: "LEAK-1",
    category: "LEAK",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[
        Need::Graph,
        Need::PartClasses,
        Need::Values,
        Need::Rails,
        Need::RailVoltages,
    ],
    overlaps: &[],
    params: &[
        Param {
            name: "max_current",
            default: 100e-6,
            unit: "A",
            doc: "the current an always-on divider may draw from its rail (the project's budget)",
        },
        Param {
            name: "max_chain",
            default: 3.0,
            unit: "",
            doc: "the most resistors in series followed from a rail to a ground",
        },
    ],
};

/// A pin that makes a divider's tap a regulator's feedback node: any pin of a regulator, or an IC pin named
/// `FB`, `VFB`, `ADJ`, `FEEDBACK`
fn feedback_tap(cx: &Ctx, net: NetId) -> bool {
    let b = cx.board();
    net_pins(b, net).any(|p| {
        let pin = b.pin(p);
        let c = cx.classes().get(pin.part);
        c.is_regulator()
            || (c.is_ic()
                && matches!(
                    pin.name.to_uppercase().as_str(),
                    "FB" | "VFB" | "ADJ" | "FEEDBACK" | "FB1" | "FB2"
                ))
    })
}

impl Check for Leak1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Chains of two or more fitted resistors from a rail with a voltage to a ground through nodes that are no
    /// rails (nothing switches them), regulator feedback dividers left out, drawing more than `max_current`
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let values = cx.values();
        let max_current = cx.param(&META, "max_current");
        let max_chain = cx.param(&META, "max_chain") as usize;
        let resistor = |p: PartId| {
            cx.fitted(p)
                && cx.classes().get(p) == PartClass::Resistor
                && values.get(p).value.is_some()
        };
        let node = |n: NetId| !power.is_power_net(n) && !power.switching_nodes.contains(&n);
        let mut out = Vec::new();
        for (rail, r) in &power.rails {
            let Some(volts) = r.volts() else { continue };
            for path in series_paths(b, *rail, &resistor, &node, max_chain) {
                if path.parts.len() < 2 || !power.is_ground(path.end) {
                    continue;
                }
                let taps: Vec<NetId> = path
                    .parts
                    .windows(2)
                    .filter_map(|w| {
                        let a = erc_analysis::graph::part_nets(b, w[0]);
                        erc_analysis::graph::part_nets(b, w[1])
                            .into_iter()
                            .find(|n| a.contains(n))
                    })
                    .collect();
                if taps.iter().any(|t| feedback_tap(cx, *t)) {
                    continue;
                }
                let ohms: f64 = path.parts.iter().filter_map(|p| values.get(*p).value).sum();
                if ohms <= 0.0 {
                    continue;
                }
                let amps = volts.abs() / ohms;
                if amps <= max_current {
                    continue;
                }
                let names: Vec<&str> = path
                    .parts
                    .iter()
                    .map(|p| b.part(*p).designator.as_str())
                    .collect();
                let mut f = cx
                    .finding(&META)
                    .net(*rail)
                    .parts(path.parts.iter().copied())
                    .fact(
                        "divider resistance",
                        format_si(ohms, Unit::Ohm),
                        Provenance::Derived {
                            rule: "the Value fields along the chain, in series".into(),
                        },
                    )
                    .fact(
                        "max_current",
                        format_si(max_current, Unit::Ampere),
                        cx.param_source(&META, "max_current"),
                    );
                if let Some(v) = r.voltage.resolve() {
                    f = f.fact(
                        "rail voltage",
                        format_si(v.value, Unit::Volt),
                        v.source.clone(),
                    );
                }
                out.push(
                    f.message(format!(
                        "divider {} ({}) from {} to {} draws {} in every power state",
                        names.join(", "),
                        format_si(ohms, Unit::Ohm),
                        b.net(*rail).name,
                        b.net(path.end).name,
                        format_si(amps, Unit::Ampere),
                    ))
                    .fix("raise the values to what the load allows (a capacitor at the tap for a dynamic load), use a reference or buffer, or switch the divider (LEAK-7)")
                    .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Leak1));

#[cfg(test)]
mod tests {
    #[test]
    fn always_on_dividers() {
        let found = crate::testing::fingerprints(&super::Leak1, "leak-1_rail_dividers");
        assert_eq!(found, ["LEAK-1:net +3V3|part R1|part R2"], "{found:?}");
    }
}
