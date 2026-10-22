//! `WIRE-14` Ground-named pin on a net that is not a ground

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::power::{GroundKind, NameRole, PowerSignal, read_name};
use erc_model::NetKind;

pub struct Wire14;

static META: CheckMeta = CheckMeta {
    id: "WIRE-14",
    category: "WIRE",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Wire14 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Pins named as a ground (`GND`, `VSS`, `AGND`, `PGND`; chassis and earth pins left out, a shield goes to
    /// ground through an RC on purpose) on a net that is no ground of the power structure, or a ground only
    /// because of such pins (no name, power symbol or project file says so)
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for p in cx.components() {
            for q in &b.part(p).pins {
                let pin = b.pin(*q);
                let Some(net) = pin.net else { continue };
                if b.net(net).kind == NetKind::Unconnected {
                    continue;
                }
                let NameRole::Ground(kind) = read_name(&pin.name, false).role else {
                    continue;
                };
                if matches!(kind, GroundKind::Chassis | GroundKind::Earth) {
                    continue;
                }
                let ground = power
                    .grounds
                    .get(&net)
                    .is_some_and(|g| g.signals.iter().any(|s| s.value != PowerSignal::GroundPin));
                if ground {
                    continue;
                }
                let what = if power.is_rail(net) {
                    "a rail"
                } else if power.is_ground(net) {
                    "a ground only by its ground pins, joined to no named ground"
                } else {
                    "not a ground"
                };
                out.push(
                    cx.finding(&META)
                        .pin(*q)
                        .part(p)
                        .net(net)
                        .message(format!(
                            "{} ({}) is on {}, which is {what}",
                            b.pin_name(*q),
                            pin.name,
                            b.net(net).name
                        ))
                        .fix("correct the symbol or the wiring; declare an isolated ground as a ground; waive a real negative supply")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Wire14));

#[cfg(test)]
mod tests {
    #[test]
    fn ground_pins_off_ground() {
        assert_eq!(
            crate::testing::fingerprints(&super::Wire14, "wire-14_ground_pin_not_ground"),
            ["WIRE-14:net /SW|part U1|pin U1.2"]
        );
    }
}
