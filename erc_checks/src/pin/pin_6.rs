//! `PIN-6` No input filter on a power input

use super::{Way, designators, pin_list, ports, power_path};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::feed::{LinkKind, power_in_on};
use erc_analysis::supply::caps_to_ground;
use erc_model::class::RegulatorKind;
use erc_model::{NetId, PartClass, PartId};
use std::collections::BTreeSet;

pub struct Pin6;

static META: CheckMeta = CheckMeta {
    id: "PIN-6",
    category: "PIN",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// A switching regulator: a buck, boost or buck-boost by class, or a regulator with a pin on a switching node
fn switching(cx: &Ctx, part: PartId) -> bool {
    let b = cx.board();
    match cx.classes().get(part) {
        PartClass::Regulator {
            kind: Some(RegulatorKind::Buck | RegulatorKind::Boost | RegulatorKind::BuckBoost),
        } => true,
        PartClass::Regulator { .. } => b
            .part(part)
            .pins
            .iter()
            .filter_map(|p| b.pin(*p).net)
            .any(|n| cx.power().switching_nodes.contains(&n)),
        _ => false,
    }
}

impl Check for Pin6 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A power input whose way to a switching regulator's input has no ferrite or inductor in series, no
    /// polarized (bulk) capacitor, and no capacitor except on the regulator's own input net
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let feeds = cx.feeds();
        let mut out = Vec::new();
        for (port, way) in ports(cx) {
            if way != Way::In {
                continue;
            }
            let reached = feeds.reach([port.net], &power_path);
            for (vin, path) in &reached {
                let regulators: BTreeSet<PartId> = power_in_on(cx.view(), *vin)
                    .into_iter()
                    .map(|p| b.pin(p).part)
                    .filter(|p| switching(cx, *p))
                    .collect();
                if regulators.is_empty() {
                    continue;
                }
                let links: Vec<_> = feeds
                    .links
                    .iter()
                    .filter(|l| path.contains(&l.part))
                    .collect();
                let series = links
                    .iter()
                    .any(|l| matches!(l.kind, LinkKind::Ferrite | LinkKind::Inductor));
                let mut nets: BTreeSet<NetId> = BTreeSet::from([port.net, *vin]);
                for l in &links {
                    nets.insert(l.a);
                    nets.insert(l.b);
                }
                let caps_on = |n: NetId| caps_to_ground(cx.view(), cx.classes(), cx.power(), n);
                let bulk = nets.iter().flat_map(|n| caps_on(*n)).any(|c| {
                    matches!(
                        cx.classes().get(c),
                        PartClass::Capacitor { polarized: true }
                    )
                });
                let own_only = nets
                    .iter()
                    .filter(|n| **n != *vin)
                    .all(|n| caps_on(*n).is_empty());
                if series || bulk || !own_only {
                    continue;
                }
                let own = caps_on(*vin);
                let own = if own.is_empty() {
                    "no capacitor at all".to_string()
                } else {
                    format!("only its own {}", designators(cx, own))
                };
                out.push(
                    cx.finding(&META)
                        .part(port.part)
                        .parts(regulators.iter().copied())
                        .net(port.net)
                        .message(format!(
                            "power from {} ({}) reaches the switching regulator {} with {own}: no ferrite, inductor or bulk capacitor filters the input",
                            pin_list(cx, &port.pins),
                            b.net(port.net).name,
                            designators(cx, regulators.iter().copied()),
                        ))
                        .fix("a ferrite or small inductor with a bulk capacitor, damped (an electrolytic or a series R) so the LC does not ring")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Pin6));

#[cfg(test)]
mod tests {
    #[test]
    fn unfiltered_inputs() {
        let found = crate::testing::fingerprints(&super::Pin6, "pin-6_unfiltered_inputs");
        assert_eq!(found, ["PIN-6:net +5V_USB|part J1|part U1"], "{found:?}");
    }
}
