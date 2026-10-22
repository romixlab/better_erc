//! `DBG-16` No firmware-driven LED (heartbeat), only power LEDs

use super::{led_ends, leds, programmable, with_resistor_ends};
use crate::meta::*;
use crate::text::designators;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_model::{EcadPinType, NetId, PartId};
use std::collections::BTreeSet;

pub struct Dbg16;

static META: CheckMeta = CheckMeta {
    id: "DBG-16",
    category: "DBG",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &["DBG-3"],
    params: &[],
};

impl Check for Dbg16 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A board with an MCU, SoC or FPGA and LEDs, none of which an MCU pin drives: directly or through a
    /// resistor, or through one transistor whose gate or base an MCU pin drives (through a resistor or not).
    /// A board with no LED at all is DBG-3's.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let mcus = programmable(cx);
        let leds = leds(cx);
        if mcus.is_empty() || leds.is_empty() {
            return Vec::new();
        }
        let driven = leds.iter().any(|led| {
            led_ends(cx, *led).iter().any(|end| {
                end.iter()
                    .any(|n| mcu_pin_on(cx, &mcus, *n) || via_transistor(cx, &mcus, *n))
            })
        });
        if driven {
            return Vec::new();
        }
        vec![
            cx.finding(&META)
                .parts(leds.iter().copied())
                .message(format!(
                    "no LED is driven by {}: {} only show power",
                    designators(cx, mcus.iter().copied()),
                    designators(cx, leds.iter().copied())
                ))
                .fix("one LED on a free GPIO, ideally one with a timer output")
                .build(),
        ]
    }
}

/// Whether a signal pin (not a power pin) of one of `mcus` is on `net`
fn mcu_pin_on(cx: &Ctx, mcus: &[PartId], net: NetId) -> bool {
    let b = cx.board();
    !cx.power().is_power_net(net)
        && net_pins(b, net).any(|p| {
            let pin = b.pin(p);
            mcus.contains(&pin.part)
                && !matches!(
                    pin.etype.get(),
                    Some(EcadPinType::PowerIn | EcadPinType::PowerOut)
                )
        })
}

/// Whether `net` is switched by a transistor whose gate or base an MCU pin drives
fn via_transistor(cx: &Ctx, mcus: &[PartId], net: NetId) -> bool {
    let b = cx.board();
    if cx.power().is_power_net(net) {
        return false;
    }
    net_pins(b, net).any(|p| {
        let q = b.pin(p).part;
        if !cx.fitted(q) || !cx.classes().get(q).is_transistor() {
            return false;
        }
        let control: BTreeSet<NetId> = b
            .part(q)
            .pins
            .iter()
            .map(|c| b.pin(*c))
            .filter(|c| matches!(c.name.to_uppercase().as_str(), "G" | "B" | "GATE" | "BASE"))
            .filter_map(|c| c.net)
            .collect();
        control.iter().any(|c| {
            *c != net
                && with_resistor_ends(cx, *c)
                    .iter()
                    .any(|n| mcu_pin_on(cx, mcus, *n))
        })
    })
}

inventory::submit!(Registered(&Dbg16));

#[cfg(test)]
mod tests {
    #[test]
    fn only_power_leds() {
        let found = crate::testing::fingerprints(&super::Dbg16, "dbg-16_heartbeat_led");
        assert_eq!(found, ["DBG-16:part D1|part D2"], "{found:?}");
        // An LED on a GPIO, and one switched by a FET whose gate a GPIO drives
        for driven in ["dbg-4_power_leds", "dbg-8_open_drain_pull_ups"] {
            let found = crate::testing::fingerprints(&super::Dbg16, driven);
            assert!(found.is_empty(), "{driven}: {found:?}");
        }
    }
}
