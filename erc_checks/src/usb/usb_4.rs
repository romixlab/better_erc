//! `USB-4` USB shield connected through L/R/C, or not connected

use super::connected;
use crate::meta::*;
use crate::text::{and_list, designators, pin_list};
use crate::{Ctx, Finding};
use erc_analysis::access::is_zero_ohm;
use erc_analysis::connector::is_shell_pin;
use erc_analysis::graph::net_pins;
use erc_model::class::ConnectorKind;
use erc_model::{NetId, PartClass, PartId, PinId};
use std::collections::BTreeSet;

pub struct Usb4;

static META: CheckMeta = CheckMeta {
    id: "USB-4",
    category: "USB",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// The grounds the circuit returns to: those with a pin of an IC on them, or every ground when no IC is on any
pub(crate) fn signal_grounds(cx: &Ctx) -> BTreeSet<NetId> {
    let b = cx.board();
    let grounds: Vec<NetId> = cx.power().grounds.keys().copied().collect();
    let with_ic: BTreeSet<NetId> = grounds
        .iter()
        .copied()
        .filter(|g| {
            net_pins(b, *g).any(|p| {
                let c = cx.classes().get(b.pin(p).part);
                cx.fitted(b.pin(p).part) && (c.is_ic() || c.is_programmable())
            })
        })
        .collect();
    if with_ic.is_empty() {
        grounds.into_iter().collect()
    } else {
        with_ic
    }
}

/// The fitted two-pin parts from `net` to another net that `to` accepts, with that net
pub(crate) fn links(cx: &Ctx, net: NetId, to: &dyn Fn(NetId) -> bool) -> Vec<(PartId, NetId)> {
    let b = cx.board();
    let mut out: Vec<(PartId, NetId)> = net_pins(b, net)
        .filter_map(|pin| {
            let part = b.pin(pin).part;
            if !cx.fitted(part) || b.part(part).pins.len() != 2 {
                return None;
            }
            let far = b
                .part(part)
                .pins
                .iter()
                .filter(|q| **q != pin)
                .find_map(|q| b.pin(*q).net)?;
            (far != net && to(far)).then_some((part, far))
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

fn is_usb(class: PartClass) -> bool {
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

impl Check for Usb4 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// The shell pins of each USB connector: connected to nothing, on a net that reaches a signal ground only
    /// through an inductor, ferrite, resistor or capacitor, or on a net that reaches no ground. A shell on a signal
    /// ground, joined to one by a 0 Ω link, or on a ground of its own (a chassis bond) is a decision taken.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let signal = signal_grounds(cx);
        let mut out = Vec::new();
        for part in cx.view().fitted_components() {
            if !is_usb(cx.classes().get(part)) {
                continue;
            }
            let shells: Vec<PinId> = b
                .part(part)
                .pins
                .iter()
                .copied()
                .filter(|p| is_shell_pin(b.pin(*p)))
                .collect();
            if shells.is_empty() {
                continue;
            }
            let nets: BTreeSet<NetId> = shells.iter().filter_map(|p| connected(cx, *p)).collect();
            if nets.is_empty() {
                out.push(
                    cx.finding(&META)
                        .part(part)
                        .pins(shells.iter().copied())
                        .message(format!(
                            "USB connector {}: shell {} connected to nothing: the cable shield ends in an antenna and ESD on the shell has no path",
                            b.part(part).designator,
                            pin_list(cx, &shells)
                        ))
                        .fix("shell straight to the ground plane with several vias, or a chosen chassis bond (RC or direct) noted on the schematic (GND-7)")
                        .build(),
                );
                continue;
            }
            for net in nets {
                if signal.contains(&net) {
                    continue;
                }
                let tied = links(cx, net, &|n| signal.contains(&n))
                    .iter()
                    .any(|(p, _)| is_zero_ohm(cx.view(), cx.classes(), cx.values(), *p));
                if tied {
                    continue;
                }
                let through: Vec<(PartId, NetId)> = links(cx, net, &|n| power.is_ground(n))
                    .into_iter()
                    .filter(|(p, _)| {
                        let c = cx.classes().get(*p);
                        c.is_resistor()
                            || c.is_capacitor()
                            || matches!(c, PartClass::Inductor | PartClass::FerriteBead)
                    })
                    .collect();
                let what = if !through.is_empty() {
                    let parts: Vec<String> = through
                        .iter()
                        .map(|(p, _)| {
                            format!(
                                "{} ({})",
                                b.part(*p).designator,
                                cx.classes().get(*p).name()
                            )
                        })
                        .collect();
                    format!("reaches ground only through {}", and_list(&parts))
                } else if power.is_ground(net) {
                    // A ground of its own (CHASSIS): a bond chosen on purpose
                    continue;
                } else {
                    "reaches no ground".to_string()
                };
                let pins: Vec<PinId> = shells
                    .iter()
                    .copied()
                    .filter(|p| b.pin(*p).net == Some(net))
                    .collect();
                out.push(
                    cx.finding(&META)
                        .part(part)
                        .net(net)
                        .parts(through.iter().map(|(p, _)| *p))
                        .message(format!(
                            "USB connector {}: shell {} on {} {}: confirm the shield termination is a decision",
                            b.part(part).designator,
                            pin_list(cx, &pins),
                            b.net(net).name,
                            what
                        ))
                        .fix(if through.is_empty() {
                            "shell straight to the ground plane with several vias, and to chassis where there is one".to_string()
                        } else {
                            format!(
                                "a series ferrite or inductor is high impedance where the shield must be low: shell straight to ground, or keep the RC ({}) as a noted decision (GND-7)",
                                designators(cx, through.iter().map(|(p, _)| *p))
                            )
                        })
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Usb4));

#[cfg(test)]
mod tests {
    #[test]
    fn shield_terminations() {
        let found = crate::testing::fingerprints(&super::Usb4, "usb-4_shield");
        assert_eq!(
            found,
            [
                "USB-4:net /SHIELD_J1|part FB1|part J1",
                "USB-4:net /SHIELD_J2|part C1|part J2|part R5",
                "USB-4:part J3|pin J3.6",
            ],
            "{found:?}"
        );
    }
}
