//! `PIN-3` Power outputs on connectors from nets used on board

use super::{Way, designators, loads_on, pin_list, ports, power_path};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::feed::LinkKind;
use erc_model::{NetId, PartId};
use std::collections::BTreeSet;

pub struct Pin3;

static META: CheckMeta = CheckMeta {
    id: "PIN-3",
    category: "PIN",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Pin3 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A connector pin that hands a rail out (a regulator feeds it) and reaches the power pins of on-board parts
    /// through nothing but passive links (ferrite, fuse, 0 Ω, diode, a discrete FET): no load switch or eFuse
    /// between. Through a fuse on every way to the loads it is a suggestion, one level lower.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let feeds = cx.feeds();
        let mut out = Vec::new();
        for (port, way) in ports(cx) {
            if way != Way::Out {
                continue;
            }
            let region = feeds.region([port.net], &power_path);
            let loads = loads_on(cx, region.keys().copied());
            if loads.is_empty() {
                continue;
            }
            let load_nets: BTreeSet<NetId> = loads.iter().filter_map(|p| b.pin(*p).net).collect();
            let fused = load_nets.iter().all(|n| {
                region[n].iter().any(|p| {
                    feeds
                        .links
                        .iter()
                        .any(|l| l.part == *p && l.kind == LinkKind::Fuse)
                })
            });
            let between: BTreeSet<PartId> = load_nets
                .iter()
                .flat_map(|n| region[n].iter().copied())
                .collect();
            let path = if between.is_empty() {
                "directly".to_string()
            } else {
                format!("through {} only", designators(cx, between.iter().copied()))
            };
            let mut f = cx
                .finding(&META)
                .part(port.part)
                .net(port.net)
                .message(format!(
                    "{} hands {} out to the connector {path}: a short there pulls down the rail {} run from",
                    pin_list(cx, &port.pins),
                    b.net(port.net).name,
                    designators(cx, loads.iter().map(|p| b.pin(*p).part)),
                ))
                .fix("a current-limited load switch or eFuse per output (TPS2553, TPS25947 class) with its fault output read by the MCU, or a separate regulator for the outputs");
            if fused {
                f = f.severity(Severity::Suggestion);
            }
            out.push(f.build());
        }
        out
    }
}

inventory::submit!(Registered(&Pin3));

#[cfg(test)]
mod tests {
    #[test]
    fn rail_handed_out() {
        let found = crate::testing::fingerprints(&super::Pin3, "pin-3_unprotected_outputs");
        assert_eq!(
            found,
            [
                "PIN-3:net +3V3_OUT|part J2",
                "PIN-3:net +3V3|part J3",
                "PIN-3:net +5V0_OUT|part J2"
            ],
            "{found:?}"
        );
    }
}
