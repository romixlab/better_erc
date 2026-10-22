//! `RAIL-19` Analog supply fed straight from the digital rail without a filter

use super::{designators, ground_named, pin_list};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::feed::power_in_on;
use erc_analysis::graph::net_pins;
use erc_model::{EcadPinType, NetId, PartId, PinId};
use std::collections::BTreeSet;

pub struct Rail19;

static META: CheckMeta = CheckMeta {
    id: "RAIL-19",
    category: "RAIL",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// An analog supply or reference input by its name: `VDDA`, `AVDD`, `AVCC`, `ADC_AVDD`, `VDDA2`, `VREF+`,
/// `VREFP`, `VDD_ANA` (not `VCCA`, a translator's A side)
pub(crate) fn analog_supply_name(name: &str) -> bool {
    let n = name.to_uppercase();
    n.split(['/', '_', '-', ' '])
        .chain(std::iter::once(n.as_str()))
        .any(|w| {
            let base = w.trim_end_matches(|c: char| c.is_ascii_digit());
            matches!(
                base,
                "VDDA" | "AVDD" | "AVCC" | "VREF+" | "VREFP" | "VDDANA" | "AVDDA"
            )
        })
        || (n.starts_with("VDD") && n.split('_').any(|w| w == "ANA" || w == "ANALOG"))
}

impl Check for Rail19 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// The nets of analog supply pins (by name) with, on the same net or behind 0 Ω resistors, jumpers or net
    /// ties only, the power pins of digital supplies (power inputs not named analog or ground)
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let feeds = cx.feeds();
        let mut out = Vec::new();
        let analog = |p: PinId| {
            let pin = b.pin(p);
            cx.fitted(pin.part)
                && analog_supply_name(&pin.name)
                && pin.etype.get() != Some(&EcadPinType::PowerOut)
        };
        for i in 0..b.nets.len() {
            let net = NetId::from_index(i);
            if !power.is_rail(net) {
                continue;
            }
            let analog_pins: Vec<PinId> = net_pins(b, net).filter(|p| analog(*p)).collect();
            if analog_pins.is_empty() {
                continue;
            }
            let region = feeds.reach([net], &|l| l.transparent());
            let mut digital: Vec<PinId> = Vec::new();
            let mut digital_nets: BTreeSet<NetId> = BTreeSet::new();
            let mut ties: BTreeSet<PartId> = BTreeSet::new();
            for (n, path) in &region {
                let d: Vec<PinId> = power_in_on(cx.view(), *n)
                    .into_iter()
                    .filter(|p| !analog(*p) && !ground_named(cx, *p))
                    .collect();
                if !d.is_empty() {
                    digital.extend(d);
                    digital_nets.insert(*n);
                    ties.extend(path.iter().copied());
                }
            }
            if digital.is_empty() {
                continue;
            }
            let users: BTreeSet<PartId> = analog_pins.iter().map(|p| b.pin(*p).part).collect();
            let how = if ties.is_empty() {
                "on the same net".to_string()
            } else {
                format!("through {} only", designators(cx, ties.iter().copied()))
            };
            let digital_net_names: Vec<&str> = digital_nets
                .iter()
                .map(|n| b.net(*n).name.as_str())
                .collect();
            out.push(
                cx.finding(&META)
                    .net(net)
                    .nets(digital_nets.iter().copied())
                    .parts(users.iter().copied())
                    .parts(ties.iter().copied())
                    .message(format!(
                        "analog supply {} on {} shares {} with the digital supply pins of {} {how}: no ferrite, RC or regulator filters it",
                        pin_list(cx, &analog_pins),
                        b.net(net).name,
                        digital_net_names.join(", "),
                        designators(cx, digital.iter().map(|p| b.pin(*p).part)),
                    ))
                    .fix("a ferrite or a small RC with the decoupling on the pin side, or a separate LDO; keep a 0 Ω footprint as the option")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Rail19));

#[cfg(test)]
mod tests {
    use super::analog_supply_name;

    #[test]
    fn analog_names() {
        for n in [
            "VDDA", "AVDD", "ADC_AVDD", "VREF+", "VDDA2", "VDD_ANA", "AVCC",
        ] {
            assert!(analog_supply_name(n), "{n}");
        }
        for n in ["VDD", "VCCA", "VREF", "IOVDD", "DVDD", "VDDIO"] {
            assert!(!analog_supply_name(n), "{n}");
        }
    }

    #[test]
    fn unfiltered_analog_supply() {
        let found =
            crate::testing::fingerprints(&super::Rail19, "rail-19_unfiltered_analog_supply");
        assert_eq!(
            found,
            [
                "RAIL-19:net +3V3|net +3V3_VDDA|part R1|part U2",
                "RAIL-19:net +3V3|part U1"
            ],
            "{found:?}"
        );
    }
}
