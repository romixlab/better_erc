//! `USB-10` Plug or captive cable with Rd on both CC pins

use super::{connected, pin, type_c};
use crate::meta::*;
use crate::text::{designators, pin_list};
use crate::{Ctx, Finding};
use erc_analysis::connector::TypeCPin;
use erc_analysis::pull::pulls;

pub struct Usb10;

static META: CheckMeta = CheckMeta {
    id: "USB-10",
    category: "USB",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Usb10 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A Type-C plug with a resistor to ground on its CC contact (A5) and another on VCONN (B5): the host sees
    /// Rd/Rd, a Debug Accessory
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for part in type_c(cx, true) {
            let (Some(cc), Some(vconn)) =
                (pin(cx, part, TypeCPin::Cc1), pin(cx, part, TypeCPin::Cc2))
            else {
                continue;
            };
            let down = |p| {
                connected(cx, p).map_or(Vec::new(), |net| {
                    pulls(cx.view(), cx.classes(), cx.values(), cx.power(), net)
                        .down
                        .iter()
                        .map(|d| d.part)
                        .collect()
                })
            };
            let (rd_cc, rd_vconn) = (down(cc), down(vconn));
            if rd_cc.is_empty() || rd_vconn.is_empty() {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .part(part)
                    .pins([cc, vconn])
                    .parts(rd_cc.iter().chain(&rd_vconn).copied())
                    .message(format!(
                        "Type-C plug {}: {} both go to ground through {}: the host sees Rd/Rd, a Debug Accessory, and may not attach as a sink",
                        b.part(part).designator,
                        pin_list(cx, &[cc, vconn]),
                        designators(cx, rd_cc.iter().chain(&rd_vconn).copied())
                    ))
                    .fix("Rd on A5 (CC) only; B5 open, or used as VCONN where the design is VCONN-powered")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Usb10));

#[cfg(test)]
mod tests {
    #[test]
    fn plug_with_rd_rd() {
        let found = crate::testing::fingerprints(&super::Usb10, "usb-10_plug_rd");
        assert_eq!(
            found,
            ["USB-10:part P1|part R1|part R2|pin P1.A5|pin P1.B5",],
            "{found:?}"
        );
    }
}
