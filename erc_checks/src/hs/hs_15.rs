//! `HS-15` MIPI DSI / CSI lanes with series parts or protection in line

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::interface::is_mipi_lane;
use erc_model::{NetId, PartId};
use std::collections::{BTreeMap, BTreeSet};

pub struct Hs15;

static META: CheckMeta = CheckMeta {
    id: "HS-15",
    category: "HS",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Hs15 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// The nets of MIPI D-PHY lanes (by net name, or a pin on them named for one: `DSI_D0P`, `CSI0_CLK_N`) and every
    /// fitted part on them that is no connector and has no lane pin of its own (the SoC, the bridge, the sensor):
    /// resistors, capacitors, filters, chokes, ESD parts. D-PHY rated parts wait for part data to be waived.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let endpoint = |part: PartId| {
            cx.classes().get(part).is_connector()
                || b.part(part)
                    .pins
                    .iter()
                    .any(|p| is_mipi_lane(&b.pin(*p).name))
        };
        let lanes: BTreeSet<NetId> = (0..b.nets.len())
            .map(NetId::from_index)
            .filter(|n| {
                is_mipi_lane(&b.net(*n).name)
                    || net_pins(b, *n).any(|p| is_mipi_lane(&b.pin(p).name))
            })
            .collect();
        let mut on: BTreeMap<PartId, BTreeSet<NetId>> = BTreeMap::new();
        for net in &lanes {
            for pin in net_pins(b, *net) {
                let part = b.pin(pin).part;
                if cx.fitted(part) && !endpoint(part) {
                    on.entry(part).or_default().insert(*net);
                }
            }
        }
        let mut out = Vec::new();
        for (part, nets) in on {
            let names: Vec<&str> = nets.iter().map(|n| b.net(*n).name.as_str()).collect();
            out.push(
                cx.finding(&META)
                    .part(part)
                    .nets(nets.iter().copied())
                    .message(format!(
                        "{} ({}) is in line with MIPI D-PHY lane {}: it shifts the high-speed levels or slows the low-power edges",
                        b.part(part).designator,
                        cx.classes().get(part).name(),
                        names.join(", ")
                    ))
                    .fix("route the lanes straight from the SoC to the connector; where ESD is needed, a part specified for D-PHY rates, and say so")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Hs15));

#[cfg(test)]
mod tests {
    #[test]
    fn parts_on_lanes() {
        let found = crate::testing::fingerprints(&super::Hs15, "hs-15_mipi_lane_parts");
        assert_eq!(
            found,
            [
                "HS-15:net /CSI_D0_N|part R2",
                "HS-15:net /CSI_D0_P|part C1",
                "HS-15:net /CSI_D0_P|part R1",
            ],
            "{found:?}"
        );
    }
}
