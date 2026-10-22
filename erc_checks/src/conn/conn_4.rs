//! `CONN-4` Connector without a ground pin

use super::connectors;
use crate::meta::*;
use crate::usb::connected;
use crate::{Ctx, Finding};

pub struct Conn4;

static META: CheckMeta = CheckMeta {
    id: "CONN-4",
    category: "CONN",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Conn4 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A fitted connector with connected pins, none of them (shell pins included) on a ground. Connectors isolated
    /// by design (relay contacts, an isolated input) are for a waiver.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for part in connectors(cx) {
            let nets: Vec<_> = b
                .part(part)
                .pins
                .iter()
                .filter_map(|p| connected(cx, *p))
                .collect();
            if nets.is_empty() || nets.iter().any(|n| power.is_ground(*n)) {
                continue;
            }
            let mut names: Vec<&str> = nets.iter().map(|n| b.net(*n).name.as_str()).collect();
            names.sort();
            names.dedup();
            out.push(
                cx.finding(&META)
                    .part(part)
                    .message(format!(
                        "connector {} carries {} and no ground: the return current takes whatever path is left (a shield, another cable, mains earth)",
                        b.part(part).designator,
                        names.join(", ")
                    ))
                    .fix("a ground pin next to the signals; for a differential bus the transceiver's reference ground, or a documented reason why the other end provides it")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Conn4));

#[cfg(test)]
mod tests {
    #[test]
    fn no_ground_pin() {
        let found = crate::testing::fingerprints(&super::Conn4, "conn-4_no_ground");
        assert_eq!(found, ["CONN-4:part J1",], "{found:?}");
    }
}
