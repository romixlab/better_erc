//! `MEM-8` DDR ZQ pin without its 240 Ω to VSS

use crate::meta::*;
use crate::text::{designators, pin_list};
use crate::usb::connected;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::interface::{DdrPin, ddr_pin};
use erc_analysis::value::format_si;
use erc_model::{PartId, PinId, Unit};

pub struct Mem8;

static META: CheckMeta = CheckMeta {
    id: "MEM-8",
    category: "MEM",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[
        Param {
            name: "zq_resistance",
            default: 240.0,
            unit: "ohm",
            doc: "the ZQ calibration resistor",
        },
        Param {
            name: "tolerance",
            default: 0.01,
            unit: "",
            doc: "how far the ZQ resistor may be from its value, and the tolerance it must have where its value states one, as a fraction",
        },
    ],
};

/// A ZQ pin of a fitted IC (a DRAM or a memory controller)
fn zq_pin(cx: &Ctx, pin: PinId) -> bool {
    let b = cx.board();
    let p = b.pin(pin);
    let class = cx.classes().get(p.part);
    cx.fitted(p.part)
        && (class.is_ic() || class.is_programmable())
        && ddr_pin(&p.name) == Some(DdrPin::Zq)
}

impl Check for Mem8 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each ZQ pin: exactly one fitted resistor on its net, to a ground, of `zq_resistance` within `tolerance` (and
    /// no looser tolerance where its value states one), and no other ZQ pin on that net
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let rz = cx.param(&META, "zq_resistance");
        let tol = cx.param(&META, "tolerance");
        let mut out = Vec::new();
        let zq: Vec<PinId> = cx
            .view()
            .fitted_components()
            .flat_map(|p| b.part(p).pins.iter().copied())
            .filter(|p| zq_pin(cx, *p))
            .collect();
        for pin in zq {
            let mut what = Vec::new();
            let mut parts: Vec<PartId> = Vec::new();
            let net = connected(cx, pin);
            match net {
                None => what.push("is connected to nothing".to_string()),
                Some(net) => {
                    let mut rs: Vec<PartId> = net_pins(b, net)
                        .map(|p| b.pin(p).part)
                        .filter(|p| {
                            cx.fitted(*p)
                                && cx.classes().get(*p).is_resistor()
                                && b.part(*p).pins.len() == 2
                        })
                        .collect();
                    rs.sort();
                    rs.dedup();
                    let shared: Vec<PinId> = net_pins(b, net)
                        .filter(|p| *p != pin && zq_pin(cx, *p))
                        .collect();
                    if rs.is_empty() {
                        what.push(format!("({}) has no resistor", b.net(net).name));
                    } else if rs.len() > 1 {
                        what.push(format!(
                            "({}) has {} resistors, {}",
                            b.net(net).name,
                            rs.len(),
                            designators(cx, rs.iter().copied())
                        ));
                    }
                    if !shared.is_empty() {
                        what.push(format!(
                            "shares its resistor with {}",
                            pin_list(cx, &shared)
                        ));
                    }
                    for r in &rs {
                        let far = b
                            .part(*r)
                            .pins
                            .iter()
                            .find_map(|q| b.pin(*q).net.filter(|n| *n != net));
                        if let Some(far) = far
                            && !power.is_ground(far)
                        {
                            what.push(format!(
                                "{} goes to {}, not VSS",
                                b.part(*r).designator,
                                b.net(far).name
                            ));
                        }
                        let v = cx.values().get(*r);
                        match v.value {
                            Some(x) if (x - rz).abs() > rz * tol + 1e-9 => what.push(format!(
                                "{} is {}, not {}",
                                b.part(*r).designator,
                                format_si(x, Unit::Ohm),
                                format_si(rz, Unit::Ohm)
                            )),
                            _ => {}
                        }
                        if let Some(t) = v.tolerance
                            && t > tol * 100.0 + 1e-9
                        {
                            what.push(format!(
                                "{} is ±{t} %, not ±{} %",
                                b.part(*r).designator,
                                tol * 100.0
                            ));
                        }
                    }
                    parts = rs;
                    if what.is_empty() {
                        continue;
                    }
                }
            }
            let mut f = cx.finding(&META).pin(pin).parts(parts.iter().copied());
            if let Some(net) = net {
                f = f.net(net);
            }
            out.push(
                f.message(format!(
                    "ZQ {} {}: drive strength and on-die termination calibrate against the wrong reference",
                    pin_list(cx, &[pin]),
                    what.join("; ")
                ))
                .fact("zq_resistance", format_si(rz, Unit::Ohm), cx.param_source(&META, "zq_resistance"))
                .fact("tolerance", tol, cx.param_source(&META, "tolerance"))
                .fix("one 240 Ω 1 % resistor from each ZQ pin to VSS, close to the pin")
                .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Mem8));

#[cfg(test)]
mod tests {
    #[test]
    fn zq_resistors() {
        let found = crate::testing::fingerprints(&super::Mem8, "mem-8_ddr_zq");
        assert_eq!(
            found,
            [
                "MEM-8:net /ZQ2|part R2|pin U2.3",
                "MEM-8:net /ZQ34|part R3|pin U3.3",
                "MEM-8:net /ZQ34|part R3|pin U4.3",
                "MEM-8:net /ZQ5|part R5|pin U5.3",
                "MEM-8:net /ZQ7|part R7|pin U7.3",
                "MEM-8:pin U8.3",
            ],
            "{found:?}"
        );
    }
}
