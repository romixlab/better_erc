//! `CONN-16` Connector shell or mounting pins left floating without a decision

use super::connectors;
use crate::meta::*;
use crate::text::pin_list;
use crate::usb::connected;
use crate::{Ctx, Finding};
use erc_analysis::connector::is_shell_pin;
use erc_analysis::graph::net_pins;
use erc_model::PinId;

pub struct Conn16;

static META: CheckMeta = CheckMeta {
    id: "CONN-16",
    category: "CONN",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Conn16 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Shell, shield and mounting pins of a fitted connector that are connected to nothing (a no-connect flag
    /// included: "isolated on purpose" is a waiver) or only to the connector's own shell pins
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for part in connectors(cx) {
            let floating: Vec<PinId> = b
                .part(part)
                .pins
                .iter()
                .copied()
                .filter(|p| is_shell_pin(b.pin(*p)))
                .filter(|p| {
                    connected(cx, *p).is_none_or(|net| {
                        net_pins(b, net).all(|q| b.pin(q).part == part && is_shell_pin(b.pin(q)))
                    })
                })
                .collect();
            if floating.is_empty() {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .part(part)
                    .pins(floating.iter().copied())
                    .message(format!(
                        "connector {}: {} float: an ESD discharge on the shell finds no defined path and a cable shield ending there shields nothing",
                        b.part(part).designator,
                        pin_list(cx, &floating)
                    ))
                    .fix("tie the shell to ground or chassis, directly or through the R/C network the design calls for (GND-7, USB-4), and note the decision on the schematic")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Conn16));

#[cfg(test)]
mod tests {
    #[test]
    fn floating_shells() {
        let found = crate::testing::fingerprints(&super::Conn16, "conn-16_floating_shell");
        assert_eq!(
            found,
            [
                "CONN-16:part J1|pin J1.MP",
                "CONN-16:part J3|pin J3.9",
                "CONN-16:part J4|pin J4.MP",
            ],
            "{found:?}"
        );
    }
}
