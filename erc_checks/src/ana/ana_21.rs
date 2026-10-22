//! `ANA-21` AC-coupled (or transformer-coupled) amplifier input with no DC return path for the bias current

use super::{connected, units, walk};
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_model::{EcadPinType, PartClass, Provenance};

pub struct Ana21;

static META: CheckMeta = CheckMeta {
    id: "ANA-21",
    category: "ANA",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Ana21 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each connected op-amp input: the nets it reaches at DC (through fitted resistors, inductors, ferrites,
    /// jumpers and net ties) must hold a rail, a ground, a connector pin or a pin that can drive (an amplifier's
    /// output, any IC pin that is no input); when they do not and a capacitor sits on them, the input is
    /// AC-coupled with no return. Transformer windings are not known (part data), so an input whose path ends at a
    /// transformer is reported as an estimate.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for u in units(cx, PartClass::Opamp) {
            for pin in [u.inp, u.inn].into_iter().flatten() {
                let Some(net) = connected(cx, Some(pin)) else {
                    continue;
                };
                let reached = walk(cx, net, &|part| {
                    let c = cx.classes().get(part);
                    (c.is_resistor() || c.is_dc_tie() || c == PartClass::Inductor)
                        && b.part(part).pins.len() == 2
                });
                let mut caps = Vec::new();
                let mut transformer = false;
                let mut returned = false;
                for n in &reached {
                    if power.is_power_net(*n) {
                        returned = true;
                        break;
                    }
                    for q in net_pins(b, *n) {
                        if q == pin {
                            continue;
                        }
                        let p = b.pin(q);
                        let c = cx.classes().get(p.part);
                        if !cx.fitted(p.part) && !c.is_connector() {
                            continue;
                        }
                        let drives = c.is_connector()
                            || ((c.is_ic() || c.is_programmable())
                                && !matches!(
                                    p.etype.get(),
                                    Some(
                                        EcadPinType::Input
                                            | EcadPinType::PowerIn
                                            | EcadPinType::NoConnect
                                    )
                                ));
                        if drives {
                            returned = true;
                        } else if c.is_capacitor() {
                            caps.push(p.part);
                        } else if c == PartClass::Transformer {
                            transformer = true;
                        }
                    }
                }
                if returned || (caps.is_empty() && !transformer) {
                    continue;
                }
                let through = if transformer {
                    "a transformer winding"
                } else {
                    "a series capacitor"
                };
                let mut f = cx
                    .finding(&META)
                    .part(u.part)
                    .pin(pin)
                    .message(format!(
                        "op-amp {} input {} reaches the circuit only through {through}: no DC path for its bias current, it drifts to a rail",
                        u.label(b),
                        pin_list(cx, &[pin])
                    ));
                if transformer {
                    f = f.fact(
                        "transformer windings",
                        "not known",
                        Provenance::Assumed {
                            rule: "ANA-21: transformer pins taken as no DC path (no part data)"
                                .into(),
                        },
                    );
                }
                out.push(
                    f.fix("a resistor from the input to ground (dual supply) or to the bias voltage (single supply); a centre tap or resistors to ground on a transformer secondary")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Ana21));

#[cfg(test)]
mod tests {
    #[test]
    fn no_dc_return() {
        let found = crate::testing::fingerprints(&super::Ana21, "ana-21_dc_return");
        assert_eq!(found, ["ANA-21:part U1|pin U1.3"], "{found:?}");
    }
}
