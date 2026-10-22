//! `IO-15` Open-drain interrupt or fault output without a pull-up

use super::net_pulls;
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::pin_name::functions;
use erc_model::{EcadPinType, NetId, PinId};
use std::collections::BTreeMap;

pub struct Io15;

static META: CheckMeta = CheckMeta {
    id: "IO-15",
    category: "IO",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails, Need::Values],
    overlaps: &[],
    params: &[],
};

impl Check for Io15 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// An interrupt, fault or power-good output (`INT`, `ALERT`, `FAULT`, `PGOOD` by name) of a fitted part that
    /// is open-drain: typed open collector or open emitter, or a pin of an IC other than an MCU whose type does
    /// not say push-pull; on a net with no fitted pull-up and no push-pull driver
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut nets: BTreeMap<NetId, Vec<PinId>> = BTreeMap::new();
        for part in cx.view().fitted_components() {
            let class = cx.classes().get(part);
            for pin in &b.part(part).pins {
                let p = b.pin(*pin);
                if !functions(&p.name).iter().any(|f| f.function.is_flag()) {
                    continue;
                }
                let open = match p.etype.get() {
                    Some(EcadPinType::OpenCollector | EcadPinType::OpenEmitter) => true,
                    Some(
                        EcadPinType::Output
                        | EcadPinType::PowerOut
                        | EcadPinType::PowerIn
                        | EcadPinType::Input
                        | EcadPinType::NoConnect,
                    ) => false,
                    _ => class.is_ic() && !class.is_programmable(),
                };
                if let Some(net) = p.net
                    && open
                    && !power.is_power_net(net)
                {
                    nets.entry(net).or_default().push(*pin);
                }
            }
        }
        let mut out = Vec::new();
        for (net, pins) in nets {
            let push_pull = net_pins(b, net).any(|p| {
                let pin = b.pin(p);
                cx.fitted(pin.part)
                    && matches!(
                        pin.etype.get(),
                        Some(EcadPinType::Output | EcadPinType::PowerOut)
                    )
            });
            if push_pull || !net_pulls(cx, net).up.is_empty() {
                continue;
            }
            out.push(
                pins.iter()
                    .fold(cx.finding(&META).net(net), |f, p| f.pin(*p))
                    .message(format!(
                        "{} on {} is an open-drain flag with no pull-up: the line never goes inactive, or floats",
                        pin_list(cx, &pins),
                        b.net(net).name
                    ))
                    .fix("a pull-up (10-100 kΩ) to the reader's IO rail, or the internal pull in the programmer report if that is the intent")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Io15));

#[cfg(test)]
mod tests {
    #[test]
    fn open_drain_flags_without_pull_up() {
        let found = crate::testing::fingerprints(&super::Io15, "io-15_open_drain_flags");
        assert_eq!(
            found,
            [
                "IO-15:net /PMIC_INT|pin U2.3",
                "IO-15:net /TEMP_ALERT|pin U4.3"
            ],
            "{found:?}"
        );
    }
}
