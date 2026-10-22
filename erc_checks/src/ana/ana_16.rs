//! `ANA-16` Gain or filter resistors far outside the sane range for the op-amp

use super::{between, connected, units};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::value::format_si;
use erc_model::{PartClass, PartId, Unit};
use std::collections::BTreeSet;

pub struct Ana16;

static META: CheckMeta = CheckMeta {
    id: "ANA-16",
    category: "ANA",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[
        Param {
            name: "min_resistance",
            default: 1e3,
            unit: "ohm",
            doc: "a gain or feedback resistor below this loads the output",
        },
        Param {
            name: "max_resistance",
            default: 1e6,
            unit: "ohm",
            doc: "a gain or feedback resistor above this lets bias current and leakage set the result",
        },
        Param {
            name: "min_capacitance",
            default: 1e-12,
            unit: "F",
            doc: "a feedback capacitor below this is in the range of the board's parasitics",
        },
    ],
};

impl Check for Ana16 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Per op-amp channel, the gain network: fitted resistors on the inverting input's net when one of them feeds
    /// back from the output, and from the output to a rail or ground (a load); each outside `min_resistance`..`max_resistance` (0 Ω links left out); and
    /// capacitors from the output to the inverting input below `min_capacitance`. A series resistor from the
    /// output to anything else (isolation, an RC into an ADC) is no part of the gain network.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let (lo, hi, cmin) = (
            cx.param(&META, "min_resistance"),
            cx.param(&META, "max_resistance"),
            cx.param(&META, "min_capacitance"),
        );
        let mut out = Vec::new();
        let mut seen: BTreeSet<PartId> = BTreeSet::new();
        for u in units(cx, PartClass::Opamp) {
            let (Some(n), o) = (connected(cx, u.inn), connected(cx, u.out)) else {
                continue;
            };
            if power.is_power_net(n) {
                continue;
            }
            // The inverting input's resistors are a gain network when a resistor feeds back from the output; a
            // loop closed through a transistor (a V-to-I stage) has a sense resistor there instead
            let gain = o.is_some_and(|o| !between(cx, o, n, &|c| c.is_resistor()).is_empty());
            let mut network: Vec<PartId> = if gain {
                net_pins(b, n)
                    .map(|p| b.pin(p).part)
                    .filter(|p| cx.classes().get(*p).is_resistor())
                    .collect()
            } else {
                Vec::new()
            };
            if let Some(o) = o {
                network.extend(
                    net_pins(b, o)
                        .map(|p| b.pin(p).part)
                        .filter(|p| cx.classes().get(*p).is_resistor())
                        .filter(|r| {
                            b.part(*r)
                                .pins
                                .iter()
                                .filter_map(|q| b.pin(*q).net)
                                .any(|m| m != o && power.is_power_net(m))
                        }),
                );
            }
            for r in network {
                if !cx.fitted(r) || b.part(r).pins.len() != 2 || !seen.insert(r) {
                    continue;
                }
                let Some(v) = cx.values().get(r).value.filter(|v| *v > 0.0) else {
                    continue;
                };
                let why = if v < lo {
                    format!("below {}: it loads the output", format_si(lo, Unit::Ohm))
                } else if v > hi {
                    format!(
                        "above {}: bias current and leakage set the result",
                        format_si(hi, Unit::Ohm)
                    )
                } else {
                    continue;
                };
                out.push(
                    cx.finding(&META)
                        .part(u.part)
                        .part(r)
                        .message(format!(
                            "op-amp {}: {} = {} in its gain network, {why}",
                            u.label(b),
                            b.part(r).designator,
                            format_si(v, Unit::Ohm)
                        ))
                        .fact(
                            if v < lo { "min_resistance" } else { "max_resistance" },
                            format_si(if v < lo { lo } else { hi }, Unit::Ohm),
                            cx.param_source(&META, if v < lo { "min_resistance" } else { "max_resistance" }),
                        )
                        .fix("keep feedback networks between 1 kΩ and a few hundred kΩ; FET-input parts where high values are needed")
                        .build(),
                );
            }
            let Some(o) = o else { continue };
            for c in between(cx, o, n, &|c| c.is_capacitor()) {
                let Some(v) = cx.values().get(c).value.filter(|v| *v > 0.0) else {
                    continue;
                };
                if v >= cmin || !seen.insert(c) {
                    continue;
                }
                out.push(
                    cx.finding(&META)
                        .part(u.part)
                        .part(c)
                        .message(format!(
                            "op-amp {}: feedback capacitor {} = {}, below {}: the board's parasitics set the response",
                            u.label(b),
                            b.part(c).designator,
                            format_si(v, Unit::Farad),
                            format_si(cmin, Unit::Farad)
                        ))
                        .fact("min_capacitance", format_si(cmin, Unit::Farad), cx.param_source(&META, "min_capacitance"))
                        .fix("a larger capacitor with the resistors scaled down, or a check of the value's unit")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Ana16));

#[cfg(test)]
mod tests {
    #[test]
    fn resistors_out_of_range() {
        let found = crate::testing::fingerprints(&super::Ana16, "ana-16_gain_resistor_range");
        assert_eq!(
            found,
            [
                "ANA-16:part C1|part U2",
                "ANA-16:part R1|part U1",
                "ANA-16:part R2|part U1",
                "ANA-16:part R5|part U2"
            ],
            "{found:?}"
        );
    }
}
