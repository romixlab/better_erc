//! `USB-9` CC1 and CC2 tied together on a receptacle

use super::{connected, pin, type_c};
use crate::meta::*;
use crate::text::{designators, pin_list};
use crate::{Ctx, Finding};
use erc_analysis::access::{is_zero_ohm, zero_ohm_groups};
use erc_analysis::connector::TypeCPin;
use erc_analysis::graph::net_pins;

pub struct Usb9;

static META: CheckMeta = CheckMeta {
    id: "USB-9",
    category: "USB",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values],
    overlaps: &[],
    params: &[],
};

impl Check for Usb9 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// CC1 and CC2 of a Type-C receptacle on one net, or on two nets joined by 0 Ω links
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let groups = zero_ohm_groups(cx.view(), cx.classes(), cx.values());
        let group = |n| *groups.get(&n).unwrap_or(&n);
        let mut out = Vec::new();
        for part in type_c(cx, false) {
            let (Some(cc1), Some(cc2)) =
                (pin(cx, part, TypeCPin::Cc1), pin(cx, part, TypeCPin::Cc2))
            else {
                continue;
            };
            let (Some(n1), Some(n2)) = (connected(cx, cc1), connected(cx, cc2)) else {
                continue;
            };
            if group(n1) != group(n2) {
                continue;
            }
            // The 0 Ω links between the two nets, when they are two
            let ties: Vec<_> = if n1 == n2 {
                Vec::new()
            } else {
                net_pins(b, n1)
                    .map(|p| b.pin(p).part)
                    .filter(|p| is_zero_ohm(cx.view(), cx.classes(), cx.values(), *p))
                    .collect()
            };
            let how = if ties.is_empty() {
                format!("on one net, {}", b.net(n1).name)
            } else {
                format!("joined by {}", designators(cx, ties.iter().copied()))
            };
            out.push(
                cx.finding(&META)
                    .part(part)
                    .pins([cc1, cc2])
                    .nets([n1, n2])
                    .parts(ties.iter().copied())
                    .message(format!(
                        "Type-C receptacle {}: {} {}: an e-marked cable's Ra lands on the sink's Rd and the source does not turn VBUS on; orientation detection and PD are gone",
                        b.part(part).designator,
                        pin_list(cx, &[cc1, cc2]),
                        how
                    ))
                    .fix("a separate 5.1 kΩ to GND on each CC pin, or a controller with two CC pins")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Usb9));

#[cfg(test)]
mod tests {
    #[test]
    fn cc_pins_tied() {
        let found = crate::testing::fingerprints(&super::Usb9, "usb-9_cc_tied");
        assert_eq!(
            found,
            [
                "USB-9:net /CC1_J2|net /CC2_J2|part J2|part R2|pin J2.A5|pin J2.B5",
                "USB-9:net /CC_J1|part J1|pin J1.A5|pin J1.B5",
            ],
            "{found:?}"
        );
    }
}
