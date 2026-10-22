//! `SER-14` CAN split termination (2 × 60 Ω) without the centre capacitor to ground

use crate::mem::line_nets;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::interface::can_line;
use erc_analysis::names::Half;
use erc_analysis::supply::caps_to_ground;
use erc_analysis::value::format_si;
use erc_model::{NetId, PartId, Unit};
use std::collections::BTreeSet;

pub struct Ser14;

static META: CheckMeta = CheckMeta {
    id: "SER-14",
    category: "SER",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[
        Param {
            name: "min_half",
            default: 50.0,
            unit: "ohm",
            doc: "lower end of one half of a split termination",
        },
        Param {
            name: "max_half",
            default: 70.0,
            unit: "ohm",
            doc: "upper end of one half of a split termination",
        },
    ],
};

impl Check for Ser14 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Two fitted resistors of `min_half`..`max_half` in series through a middle net, their outer ends on lines that
    /// reach (directly or one series part away) the CANH and CANL pins of one transceiver, and no fitted capacitor
    /// from the middle net to a ground
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let (min, max) = (cx.param(&META, "min_half"), cx.param(&META, "max_half"));
        let halves: Vec<(PartId, [NetId; 2])> = cx
            .view()
            .fitted_components()
            .filter(|p| cx.classes().get(*p).is_resistor() && b.part(*p).pins.len() == 2)
            .filter(|p| {
                cx.values()
                    .get(*p)
                    .value
                    .is_some_and(|v| v >= min && v <= max)
            })
            .filter_map(|p| {
                let n: Vec<NetId> = b
                    .part(p)
                    .pins
                    .iter()
                    .filter_map(|q| b.pin(*q).net)
                    .collect();
                match n.as_slice() {
                    [x, y] if x != y => Some((p, [*x, *y])),
                    _ => None,
                }
            })
            .collect();
        // The transceivers with the CAN half each line reaches
        let reach = |net: NetId| -> BTreeSet<(PartId, Half)> {
            line_nets(cx, net)
                .into_iter()
                .flat_map(|n| net_pins(b, n))
                .filter_map(|p| {
                    let pin = b.pin(p);
                    let c = cx.classes().get(pin.part);
                    (cx.fitted(pin.part) && (c.is_ic() || c.is_programmable()))
                        .then(|| Some((pin.part, can_line(&pin.name)?)))
                        .flatten()
                })
                .collect()
        };
        let mut out = Vec::new();
        for (i, (r1, n1)) in halves.iter().enumerate() {
            for (r2, n2) in &halves[i + 1..] {
                let Some(mid) = n1.iter().find(|n| n2.contains(n)).copied() else {
                    continue;
                };
                let a = n1.iter().copied().find(|n| *n != mid).expect("two nets");
                let c = n2.iter().copied().find(|n| *n != mid).expect("two nets");
                if a == c || power.is_power_net(mid) {
                    continue;
                }
                let (ra, rc) = (reach(a), reach(c));
                let can = ra.iter().any(|(t, h)| {
                    rc.contains(&(
                        *t,
                        if *h == Half::Pos {
                            Half::Neg
                        } else {
                            Half::Pos
                        },
                    ))
                });
                if !can || !caps_to_ground(cx.view(), cx.classes(), power, mid).is_empty() {
                    continue;
                }
                let v =
                    |r: PartId| format_si(cx.values().get(r).value.unwrap_or_default(), Unit::Ohm);
                out.push(
                    cx.finding(&META)
                        .parts([*r1, *r2])
                        .net(mid)
                        .message(format!(
                            "CAN split termination {} ({}) and {} ({}) has no capacitor from its centre {} to ground: common-mode noise has no path and it is a plain 120 Ω",
                            b.part(*r1).designator,
                            v(*r1),
                            b.part(*r2).designator,
                            v(*r2),
                            b.net(mid).name
                        ))
                        .fix("4.7 nF (1-10 nF, rated for the bus fault voltage) from the centre to ground or to the transceiver's SPLIT pin; 1 % resistors")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Ser14));

#[cfg(test)]
mod tests {
    #[test]
    fn split_termination() {
        let found = crate::testing::fingerprints(&super::Ser14, "ser-14_can_split_termination");
        assert_eq!(
            found,
            [
                "SER-14:net /CAN1_SPLIT|part R1|part R2",
                "SER-14:net /CAN3_SPLIT|part R5|part R6",
            ],
            "{found:?}"
        );
    }
}
