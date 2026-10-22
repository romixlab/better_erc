//! `SHORT-5` Supply pins shorted to ground, or ground pins to a rail

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::power::{NameRole, RailKind, read_name};
use erc_model::NetKind;

pub struct Short5;

static META: CheckMeta = CheckMeta {
    id: "SHORT-5",
    category: "SHORT",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Short5 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A pin named as a positive supply (`VDD`, `VCC`, `VIN`, `AVDD`; not a reference) on a ground, and a pin
    /// named as a ground (`GND`, `VSS`) or a negative supply (`VEE`, `V-`) on a positive rail
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
                let name = read_name(&pin.name, false);
                // A rail without a voltage counts as positive for a ground pin unless its name is negative; a negative
                // supply pin needs a rail known to be positive
                let rail = power.rails.get(&net);
                let positive_rail = rail.is_some_and(|r| {
                    r.volts()
                        .map_or_else(|| !read_name(&b.net(net).name, false).negative, |v| v > 0.0)
                });
                let known_positive = rail.and_then(|r| r.volts()).is_some_and(|v| v > 0.0);
                let wrong = match name.role {
                    NameRole::Rail(kind) if kind != RailKind::Reference && !name.negative => {
                        power.is_ground(net).then_some("supply pin on a ground")
                    }
                    NameRole::Rail(_) if name.negative => {
                        known_positive.then_some("negative supply pin on a positive rail")
                    }
                    NameRole::Ground(_) => positive_rail.then_some("ground pin on a positive rail"),
                    _ => None,
                };
                let Some(what) = wrong else { continue };
                out.push(
                    cx.finding(&META)
                        .pin(*q)
                        .part(p)
                        .net(net)
                        .message(format!(
                            "{} ({}): {what}, {}",
                            b.pin_name(*q),
                            pin.name,
                            b.net(net).name
                        ))
                        .fix("rewire the pin; check the symbol's pin numbers against the datasheet pinout")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Short5));

#[cfg(test)]
mod tests {
    #[test]
    fn supply_pins_on_the_wrong_net() {
        assert_eq!(
            crate::testing::fingerprints(&super::Short5, "short-5_supply_pins_swapped"),
            [
                "SHORT-5:net +3V3|part U2|pin U2.2",
                "SHORT-5:net GND|part U1|pin U1.1",
            ]
        );
    }
}
