//! `HS-7` USB D+ and D- swapped (by pin name on the part vs the connector)

use crate::meta::*;
use crate::text::pin_list;
use crate::usb::connected;
use crate::{Ctx, Finding};
use erc_analysis::connector::{TypeCPin, type_c_pin};
use erc_analysis::graph::net_pins;
use erc_analysis::interface::usb_data;
use erc_analysis::names::Half;
use erc_model::class::ConnectorKind;
use erc_model::{NetId, PartClass, PartId, PinId};
use std::collections::{BTreeMap, BTreeSet};

pub struct Hs7;

static META: CheckMeta = CheckMeta {
    id: "HS-7",
    category: "HS",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[Param {
        name: "max_hops",
        default: 3.0,
        unit: "parts",
        doc: "how many series parts (resistors, ferrites, inductors, ties) the path from the connector may pass",
    }],
};

/// The USB data half of a connector pin: a Type-C contact by its number (A6/B6 D+, A7/B7 D-), else its name
fn connector_half(cx: &Ctx, pin: PinId) -> Option<Half> {
    let p = cx.board().pin(pin);
    match type_c_pin(p) {
        Some(TypeCPin::Dp { .. }) => Some(Half::Pos),
        Some(TypeCPin::Dn { .. }) => Some(Half::Neg),
        _ => usb_data(&p.name),
    }
}

fn is_usb_connector(class: PartClass) -> bool {
    matches!(
        class,
        PartClass::Connector {
            kind: Some(
                ConnectorKind::UsbA
                    | ConnectorKind::UsbB
                    | ConnectorKind::UsbMicro
                    | ConnectorKind::UsbC
            )
        }
    )
}

impl Check for Hs7 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// From each D+ or D- pin of a USB connector, through up to `max_hops` series resistors, ferrites, inductors and
    /// ties: an IC pin named for the other line. Common-mode chokes and analog switches in the path are not
    /// followed (signal following, `NET-10`).
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let max_hops = cx.param(&META, "max_hops") as usize;
        let series = |part: PartId| {
            let c = cx.classes().get(part);
            cx.fitted(part)
                && b.part(part).pins.len() == 2
                && (c.is_resistor() || c.is_dc_tie() || c == PartClass::Inductor)
        };
        let mut out = Vec::new();
        for conn in cx.view().fitted_components() {
            if !is_usb_connector(cx.classes().get(conn)) {
                continue;
            }
            // IC -> (connector pins, IC pins) crossed
            let mut crossed: BTreeMap<PartId, (BTreeSet<PinId>, BTreeSet<PinId>)> = BTreeMap::new();
            for &cp in &b.part(conn).pins {
                let (Some(half), Some(start)) = (connector_half(cx, cp), connected(cx, cp)) else {
                    continue;
                };
                let mut seen = BTreeSet::from([start]);
                let mut layer = vec![start];
                for hop in 0..=max_hops {
                    let mut next = Vec::new();
                    for net in layer {
                        for pin in net_pins(b, net) {
                            let part = b.pin(pin).part;
                            let class = cx.classes().get(part);
                            if part != conn
                                && cx.fitted(part)
                                && (class.is_ic() || class.is_programmable())
                                && usb_data(&b.pin(pin).name).is_some_and(|h| h != half)
                            {
                                let e = crossed.entry(part).or_default();
                                e.0.insert(cp);
                                e.1.insert(pin);
                            }
                            if hop < max_hops && series(part) {
                                for q in &b.part(part).pins {
                                    if let Some(n) = b.pin(*q).net
                                        && !power.is_power_net(n)
                                        && seen.insert(n)
                                    {
                                        next.push(n);
                                    }
                                }
                            }
                        }
                    }
                    layer = next;
                }
            }
            for (ic, (cpins, ipins)) in crossed {
                let cpins: Vec<PinId> = cpins.into_iter().collect();
                let ipins: Vec<PinId> = ipins.into_iter().collect();
                let nets: BTreeSet<NetId> = cpins.iter().filter_map(|p| b.pin(*p).net).collect();
                out.push(
                    cx.finding(&META)
                        .parts([conn, ic])
                        .pins(cpins.iter().chain(&ipins).copied())
                        .nets(nets)
                        .message(format!(
                            "USB data crossed between connector {} and {}: {} reach {}",
                            b.part(conn).designator,
                            b.part(ic).designator,
                            pin_list(cx, &cpins),
                            pin_list(cx, &ipins)
                        ))
                        .fix("cross the lines back (a pin swap on the connector or the ESD array), or set the part's swap register and document it")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Hs7));

#[cfg(test)]
mod tests {
    #[test]
    fn crossed_data_lines() {
        let found = crate::testing::fingerprints(&super::Hs7, "hs-7_usb_dp_dm_swapped");
        assert_eq!(
            found,
            [
                "HS-7:net /J1_DM|net /J1_DP|part J1|part U1|pin J1.A6|pin J1.A7|pin J1.B6|pin J1.B7|pin U1.3|pin U1.4",
            ],
            "{found:?}"
        );
    }
}
