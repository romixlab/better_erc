//! `GND-7` Chassis or shield ground tied to signal ground, or floating, without a decision

use super::designators;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::power::GroundKind;
use erc_model::{NetId, PartClass, PartId, PinId};
use std::collections::BTreeMap;

pub struct Gnd7;

static META: CheckMeta = CheckMeta {
    id: "GND-7",
    category: "GND",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// A connector's shell, shield or mounting pin by name (`SHIELD`, `SH1`, `SHELL`, `MP`, `MH2`, `MOUNT`)
fn shell_pin_name(name: &str) -> bool {
    let n = name.to_uppercase();
    let base = n.trim_end_matches(|c: char| c.is_ascii_digit());
    matches!(
        base,
        "SHIELD" | "SH" | "SHELL" | "SHLD" | "MP" | "MH" | "MOUNT" | "MOUNTING" | "CASE"
    )
}

/// A signal-side ground: what a chassis is joined to
fn signal_ground(k: GroundKind) -> bool {
    matches!(
        k,
        GroundKind::Signal | GroundKind::Digital | GroundKind::Analog | GroundKind::Power
    )
}

impl Check for Gnd7 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A chassis or earth net that no fitted part joins to a signal ground (floating), and connector shell pins or
    /// mounting-hole pads straight on a signal ground with no part between: no drawn decision either way. A
    /// resistor, capacitor, both, a net tie, 0 Ω or ferrite between them is the decision, drawn.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for (net, g) in &power.grounds {
            if !matches!(g.kind, GroundKind::Chassis | GroundKind::Earth) {
                continue;
            }
            let joined = power.ground_links.iter().any(|l| {
                l.grounds.contains(net)
                    && l.grounds.iter().any(|o| {
                        power
                            .grounds
                            .get(o)
                            .is_some_and(|og| signal_ground(og.kind))
                    })
            });
            if joined {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .net(*net)
                    .message(format!(
                        "{} is joined to no signal ground: a floating shield does not shield and charges up until it arcs",
                        b.net(*net).name
                    ))
                    .fix("decide per product (enclosure, cable, earth) and draw it at the connector: a net tie, a resistor, a capacitor or R ‖ C to ground")
                    .build(),
            );
        }
        let mut direct: BTreeMap<(PartId, NetId), Vec<PinId>> = BTreeMap::new();
        for part in cx.view().fitted_components() {
            let class = cx.classes().get(part);
            let hole = class == PartClass::MountingHole;
            if !(hole || class.is_connector()) {
                continue;
            }
            for p in &b.part(part).pins {
                let pin = b.pin(*p);
                let Some(net) = pin.net else { continue };
                let on_signal_ground = power
                    .grounds
                    .get(&net)
                    .is_some_and(|g| signal_ground(g.kind));
                if on_signal_ground && (hole || shell_pin_name(&pin.name)) {
                    direct.entry((part, net)).or_default().push(*p);
                }
            }
        }
        for ((part, net), pins) in direct {
            let what = if cx.classes().get(part) == PartClass::MountingHole {
                format!("mounting hole {}", designators(cx, [part]))
            } else {
                format!(
                    "shell of {} ({})",
                    designators(cx, [part]),
                    crate::text::pin_list(cx, &pins)
                )
            };
            out.push(
                cx.finding(&META)
                    .part(part)
                    .net(net)
                    .message(format!(
                        "the {what} is on {} directly: cable-shield and ESD currents flow through signal ground, and nothing says it was chosen",
                        b.net(net).name
                    ))
                    .fix("a chassis net joined to ground by a part you choose (net tie, 0 Ω, R ‖ C such as 1 MΩ ‖ 4.7 nF), at the connector")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Gnd7));

#[cfg(test)]
mod tests {
    use super::shell_pin_name;

    #[test]
    fn shell_names() {
        for n in ["SHIELD", "SH1", "Shell", "MP", "MH2"] {
            assert!(shell_pin_name(n), "{n}");
        }
        for n in ["GND", "VBUS", "SHDN", "MISO"] {
            assert!(!shell_pin_name(n), "{n}");
        }
    }

    #[test]
    fn chassis_decisions() {
        let found = crate::testing::fingerprints(&super::Gnd7, "gnd-7_chassis_ground");
        assert_eq!(
            found,
            [
                "GND-7:net /SHIELD_ETH",
                "GND-7:net GND|part H1",
                "GND-7:net GND|part J1"
            ],
            "{found:?}"
        );
    }
}
