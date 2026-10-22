//! `PIN-4` No overcurrent protection on a power input

use super::{Way, designators, loads_on, pin_list, ports, power_path};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::feed::LinkKind;
use erc_model::{NetId, PartClass, PartId};
use std::collections::BTreeSet;

pub struct Pin4;

static META: CheckMeta = CheckMeta {
    id: "PIN-4",
    category: "PIN",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Pin4 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A connector or battery pin that brings power in and reaches the power pins of a part (a regulator's input,
    /// an IC) with no fuse on the way; a load switch or eFuse is the protection, not a load. Self-limiting
    /// sources (a USB host port) are reported too until the project file can exempt them (CFG-2).
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let feeds = cx.feeds();
        let mut out = Vec::new();
        for (port, way) in ports(cx) {
            if way != Way::In {
                continue;
            }
            let reached = feeds.reach([port.net], &|l| power_path(l) && l.kind != LinkKind::Fuse);
            let loads: Vec<_> = loads_on(cx, reached.keys().copied())
                .into_iter()
                .filter(|p| cx.classes().get(b.pin(*p).part) != PartClass::LoadSwitch)
                .collect();
            if loads.is_empty() {
                continue;
            }
            let load_nets: BTreeSet<NetId> = loads.iter().filter_map(|p| b.pin(*p).net).collect();
            let between: BTreeSet<PartId> = load_nets
                .iter()
                .flat_map(|n| reached[n].iter().copied())
                .collect();
            let path = if between.is_empty() {
                String::new()
            } else {
                format!(" through {}", designators(cx, between.iter().copied()))
            };
            out.push(
                cx.finding(&META)
                    .part(port.part)
                    .net(port.net)
                    .message(format!(
                        "power comes in on {} ({}) and reaches {}{path} with no fuse, polyfuse or eFuse on the way",
                        pin_list(cx, &port.pins),
                        b.net(port.net).name,
                        designators(cx, loads.iter().map(|p| b.pin(*p).part)),
                    ))
                    .fix("a fuse or polyfuse sized per PIN-7, PIN-10, PIN-15 right after the connector, or an eFuse that also covers inrush and overvoltage")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Pin4));

#[cfg(test)]
mod tests {
    #[test]
    fn unfused_inputs() {
        let found = crate::testing::fingerprints(&super::Pin4, "pin-4_unfused_inputs");
        assert_eq!(
            found,
            ["PIN-4:net +12V_IN|part J1", "PIN-4:net /VBAT|part BT1"],
            "{found:?}"
        );
    }
}
