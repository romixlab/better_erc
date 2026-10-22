//! `USB-11` USB 2.0 wired to one orientation only

use super::{connected, pin, type_c};
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::access::zero_ohm_groups;
use erc_analysis::connector::TypeCPin;
use erc_analysis::graph::net_pins;
use erc_model::{NetId, PartId};
use std::collections::BTreeSet;

pub struct Usb11;

static META: CheckMeta = CheckMeta {
    id: "USB-11",
    category: "USB",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values],
    overlaps: &[],
    params: &[],
};

impl Check for Usb11 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A Type-C receptacle whose A6 and B6 (D+), or A7 and B7 (D-), are not on one net: directly, through 0 Ω
    /// links, or through an IC both reach (a mux that joins them by design). A receptacle with neither row wired
    /// carries no USB 2.0 and is left alone.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let groups = zero_ohm_groups(cx.view(), cx.classes(), cx.values());
        let group = |n: NetId| *groups.get(&n).unwrap_or(&n);
        let ics = |net: NetId, except: PartId| -> BTreeSet<PartId> {
            net_pins(b, net)
                .map(|p| b.pin(p).part)
                .filter(|p| {
                    let c = cx.classes().get(*p);
                    *p != except && cx.fitted(*p) && (c.is_ic() || c.is_programmable())
                })
                .collect()
        };
        let mut out = Vec::new();
        for part in type_c(cx, false) {
            for (line, a, bb) in [
                (
                    "D+",
                    TypeCPin::Dp { row_b: false },
                    TypeCPin::Dp { row_b: true },
                ),
                (
                    "D-",
                    TypeCPin::Dn { row_b: false },
                    TypeCPin::Dn { row_b: true },
                ),
            ] {
                let (Some(pa), Some(pb)) = (pin(cx, part, a), pin(cx, part, bb)) else {
                    continue;
                };
                let (na, nb) = (connected(cx, pa), connected(cx, pb));
                let what = match (na, nb) {
                    (None, None) => continue,
                    (Some(x), Some(y)) => {
                        if group(x) == group(y) || !ics(x, part).is_disjoint(&ics(y, part)) {
                            continue;
                        }
                        format!("are on two nets, {} and {}", b.net(x).name, b.net(y).name)
                    }
                    (Some(x), None) | (None, Some(x)) => {
                        format!("are not both wired: only one is on {}", b.net(x).name)
                    }
                };
                out.push(
                    cx.finding(&META)
                        .part(part)
                        .pins([pa, pb])
                        .nets(na.into_iter().chain(nb))
                        .message(format!(
                            "Type-C receptacle {}: {} {} {}: USB works in one plug orientation only",
                            b.part(part).designator,
                            line,
                            pin_list(cx, &[pa, pb]),
                            what
                        ))
                        .fix("join the rows at the receptacle (A6 to B6, A7 to B7), stub under 3.5 mm")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Usb11));

#[cfg(test)]
mod tests {
    #[test]
    fn one_row_wired() {
        let found = crate::testing::fingerprints(&super::Usb11, "usb-11_one_orientation");
        assert_eq!(
            found,
            [
                "USB-11:net /USB4_DM|net /USB4_DM_B|part J4|pin J4.A7|pin J4.B7",
                "USB-11:net /USB_DP|part J1|pin J1.A6|pin J1.B6",
            ],
            "{found:?}"
        );
    }
}
