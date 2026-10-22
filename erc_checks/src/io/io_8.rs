//! `IO-8` Mention a needed pull-up or down where it applies, e.g. buttons

use super::{button_sides, net_pulls, switches};
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::pin_name::{PinFunction, functions};
use erc_model::{EcadPinType, NetId, PinId};
use std::collections::BTreeMap;

pub struct Io8;

static META: CheckMeta = CheckMeta {
    id: "IO-8",
    category: "IO",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails, Need::Values],
    overlaps: &[],
    params: &[],
};

impl Check for Io8 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A switch to a rail or ground with no fitted pull to the other level on its signal side, and an
    /// open-drain output (pin type) that is no interrupt, fault or power-good flag (IO-15) and no I2C line
    /// (I2C-1) on a net with an MCU pin and no pull-up: the firmware must enable an internal pull. Errata that
    /// rule internal pulls out (RP2350-E9) need part data (CFG-5) and are not judged here.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for sw in switches(cx) {
            let Some((far, signals)) = button_sides(cx, sw) else {
                continue;
            };
            let to_ground = power.is_ground(far);
            for net in signals {
                let p = net_pulls(cx, net);
                let opposite = if to_ground { &p.up } else { &p.down };
                if !opposite.is_empty() {
                    continue;
                }
                let (pull, level) = if to_ground {
                    ("pull-up", "high")
                } else {
                    ("pull-down", "low")
                };
                out.push(
                    cx.finding(&META)
                        .net(net)
                        .part(sw)
                        .message(format!(
                            "{} pulls {} to {} with no external {pull}: the reader needs an internal {pull} to see it {level} while released",
                            b.part(sw).designator,
                            b.net(net).name,
                            b.net(far).name
                        ))
                        .fix(format!("an external {pull}, or the internal one enabled in the firmware's pin setup and noted in the programmer report"))
                        .build(),
                );
            }
        }
        // Open-drain outputs read by a controller
        let mut nets: BTreeMap<NetId, Vec<PinId>> = BTreeMap::new();
        for part in cx.view().fitted_components() {
            for pin in &b.part(part).pins {
                let p = b.pin(*pin);
                let open = matches!(
                    p.etype.get(),
                    Some(EcadPinType::OpenCollector | EcadPinType::OpenEmitter)
                );
                let named = functions(&p.name).iter().any(|f| {
                    f.function.is_flag()
                        || matches!(f.function, PinFunction::I2cScl | PinFunction::I2cSda)
                });
                if let Some(net) = p.net
                    && open
                    && !named
                    && !power.is_power_net(net)
                {
                    nets.entry(net).or_default().push(*pin);
                }
            }
        }
        for (net, pins) in nets {
            let reader = net_pins(b, net).any(|p| {
                let part = b.pin(p).part;
                cx.fitted(part) && cx.classes().get(part).is_programmable()
            });
            if !reader || !net_pulls(cx, net).up.is_empty() {
                continue;
            }
            out.push(
                pins.iter()
                    .fold(cx.finding(&META).net(net), |f, p| f.pin(*p))
                    .message(format!(
                        "{} on {} is open-drain with no external pull-up: the controller needs its internal pull-up enabled",
                        pin_list(cx, &pins),
                        b.net(net).name
                    ))
                    .fix("an external pull-up to the reader's IO rail, or the internal one in the firmware's pin setup and the programmer report")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Io8));

#[cfg(test)]
mod tests {
    #[test]
    fn internal_pulls_needed() {
        let found = crate::testing::fingerprints(&super::Io8, "io-8_internal_pulls");
        assert_eq!(
            found,
            [
                "IO-8:net /BTN_DN|part SW2",
                "IO-8:net /BTN_UP|part SW1",
                "IO-8:net /LIGHT_DRDY|pin U2.3"
            ],
            "{found:?}"
        );
    }
}
