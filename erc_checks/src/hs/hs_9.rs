//! `HS-9` Common-mode choke on USB 3 or PCIe lanes

use super::is_choke;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::connector::{TypeCPin, type_c_pin};
use erc_analysis::graph::net_pins;
use erc_analysis::interface::is_high_speed_lane;
use erc_model::{NetId, PartId};
use std::collections::BTreeSet;

pub struct Hs9;

static META: CheckMeta = CheckMeta {
    id: "HS-9",
    category: "HS",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

/// A net of a multi-gigabit lane: by its name, or a pin on it of another part named for one (a Type-C SuperSpeed
/// contact included)
pub(crate) fn lane_net(cx: &Ctx, net: NetId, except: PartId) -> bool {
    let b = cx.board();
    is_high_speed_lane(&b.net(net).name)
        || net_pins(b, net).any(|p| {
            let pin = b.pin(p);
            pin.part != except
                && (is_high_speed_lane(&pin.name)
                    || (cx.classes().get(pin.part).is_connector()
                        && type_c_pin(pin) == Some(TypeCPin::SuperSpeed)))
        })
}

impl Check for Hs9 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A common-mode choke (four pins) with two or more of its nets on a USB 3, PCIe, SATA, HDMI, DisplayPort or
    /// FPGA transceiver lane
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for part in cx.view().fitted_components() {
            if !is_choke(cx, part) {
                continue;
            }
            let lanes: BTreeSet<NetId> = b
                .part(part)
                .pins
                .iter()
                .filter_map(|p| b.pin(*p).net)
                .filter(|n| lane_net(cx, *n, part))
                .collect();
            if lanes.len() < 2 {
                continue;
            }
            let names: Vec<&str> = lanes.iter().map(|n| b.net(*n).name.as_str()).collect();
            out.push(
                cx.finding(&META)
                    .part(part)
                    .nets(lanes.iter().copied())
                    .message(format!(
                        "common-mode choke {} sits in a multi-gigabit lane ({}): at these rates its pads and windings cost more loss and reflection than the noise it removes",
                        b.part(part).designator,
                        names.join(", ")
                    ))
                    .fix("default to a 0402 0 Ω footprint pair that can take a choke if EMC testing calls for one; AC caps on the protected side, all close to the connector")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Hs9));

#[cfg(test)]
mod tests {
    #[test]
    fn choke_on_superspeed() {
        let found = crate::testing::fingerprints(&super::Hs9, "hs-9_choke_on_lanes");
        assert_eq!(
            found,
            ["HS-9:net /SSTX_HUB_N|net /SSTX_HUB_P|net /SSTX_N|net /SSTX_P|part L1",],
            "{found:?}"
        );
    }
}
