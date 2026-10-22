//! `ETH-3` MDIO without a pull-up (1.5k-10k)

use crate::mem::line_pulls;
use crate::meta::*;
use crate::text::{and_list, pin_list};
use crate::usb::connected;
use crate::{Ctx, Finding};
use erc_analysis::interface::is_mdio;
use erc_analysis::value::format_si;
use erc_model::{NetId, PinId, Unit};
use std::collections::BTreeMap;

pub struct Eth3;

static META: CheckMeta = CheckMeta {
    id: "ETH-3",
    category: "ETH",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[
        Param {
            name: "min_pull_up",
            default: 1000.0,
            unit: "ohm",
            doc: "lower end of the MDIO pull-up window, inside it",
        },
        Param {
            name: "max_pull_up",
            default: 10_000.0,
            unit: "ohm",
            doc: "upper end of the MDIO pull-up window, inside it",
        },
    ],
};

impl Check for Eth3 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A net on a pin named MDIO (of a PHY or a MAC) whose line (the net and one series resistor away) has no
    /// pull-up to a rail, or only pull-ups outside the window
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let (min, max) = (
            cx.param(&META, "min_pull_up"),
            cx.param(&META, "max_pull_up"),
        );
        let mut nets: BTreeMap<NetId, Vec<PinId>> = BTreeMap::new();
        for part in cx.view().fitted_components() {
            for pin in &b.part(part).pins {
                if is_mdio(&b.pin(*pin).name)
                    && let Some(net) = connected(cx, *pin)
                    && !cx.power().is_power_net(net)
                {
                    nets.entry(net).or_default().push(*pin);
                }
            }
        }
        let mut out = Vec::new();
        for (net, pins) in nets {
            let ups = line_pulls(cx, net, true);
            let outside: Vec<String> = ups
                .iter()
                .filter_map(|u| {
                    let v = cx.values().get(u.part).value?;
                    (v < min || v > max).then(|| {
                        format!("{} {}", b.part(u.part).designator, format_si(v, Unit::Ohm))
                    })
                })
                .collect();
            let what = if ups.is_empty() {
                "has no pull-up".to_string()
            } else if outside.len() == ups.len() {
                format!(
                    "is pulled up by {}, outside {}..{}",
                    and_list(&outside),
                    format_si(min, Unit::Ohm),
                    format_si(max, Unit::Ohm)
                )
            } else {
                continue;
            };
            out.push(
                cx.finding(&META)
                    .net(net)
                    .pins(pins.iter().copied())
                    .parts(ups.iter().map(|u| u.part))
                    .message(format!(
                        "MDIO {} ({}) {}: the idle level and the preamble of ones come from the pull-up alone; the line floats in turnaround",
                        b.net(net).name,
                        pin_list(cx, &pins),
                        what
                    ))
                    .fact(
                        "window",
                        format!("{}..{}", format_si(min, Unit::Ohm), format_si(max, Unit::Ohm)),
                        cx.param_source(&META, "min_pull_up"),
                    )
                    .fix("one 1.5 kΩ (1-4.7 kΩ) pull-up to the PHY's IO rail per MDIO bus")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Eth3));

#[cfg(test)]
mod tests {
    #[test]
    fn mdio_pull_up() {
        let found = crate::testing::fingerprints(&super::Eth3, "eth-3_mdio_pull_up");
        assert_eq!(
            found,
            [
                "ETH-3:net /MDIO1|pin U1.3|pin U2.3",
                "ETH-3:net /MDIO3|part R3|pin U5.3|pin U6.3",
            ],
            "{found:?}"
        );
    }
}
