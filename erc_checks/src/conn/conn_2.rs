//! `CONN-2` GPIOs straight to connectors without series protection

use super::{connectors, contacts};
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::interface::{is_high_speed_lane, is_mipi_lane, usb_data};
use erc_analysis::names::diff_half;
use erc_model::{EcadPinType, NetId, PartClass, PinId};
use std::collections::BTreeMap;

pub struct Conn2;

static META: CheckMeta = CheckMeta {
    id: "CONN-2",
    category: "CONN",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// A logic pin of an MCU, SoC, FPGA or logic IC: not a supply pin
fn logic_pin(cx: &Ctx, pin: PinId) -> bool {
    let b = cx.board();
    let p = b.pin(pin);
    let class = cx.classes().get(p.part);
    cx.fitted(p.part)
        && (class.is_programmable() || matches!(class, PartClass::Logic { .. }))
        && !matches!(
            p.etype.get(),
            Some(EcadPinType::PowerIn | EcadPinType::PowerOut | EcadPinType::NoConnect)
        )
}

/// A net of an interface that cannot take a series resistor: a differential pair by its name, USB data, a
/// multi-gigabit or MIPI lane by a pin or net name, or a net class that names one
fn exempt(cx: &Ctx, net: NetId) -> bool {
    let b = cx.board();
    let name = &b.net(net).name;
    let class = b
        .net(net)
        .class
        .get()
        .map(|c| c.to_uppercase())
        .unwrap_or_default();
    diff_half(name).is_some()
        || is_high_speed_lane(name)
        || is_mipi_lane(name)
        || ["USB", "ETH", "HDMI", "PCIE", "LVDS", "MIPI", "DIFF"]
            .iter()
            .any(|w| class.contains(w))
        || net_pins(b, net).any(|p| {
            let n = &b.pin(p).name;
            usb_data(n).is_some() || is_high_speed_lane(n) || is_mipi_lane(n)
        })
}

impl Check for Conn2 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A connector pin on the same net as a logic pin of an MCU, SoC, FPGA or logic IC: nothing in series between
    /// them. Rails, grounds and interfaces that cannot take a series part are left out.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut by_net: BTreeMap<NetId, Vec<PinId>> = BTreeMap::new();
        for part in connectors(cx) {
            for (pin, net) in contacts(cx, part) {
                if !power.is_power_net(net) {
                    by_net.entry(net).or_default().push(pin);
                }
            }
        }
        let mut out = Vec::new();
        for (net, conn_pins) in by_net {
            let ics: Vec<PinId> = net_pins(b, net).filter(|p| logic_pin(cx, *p)).collect();
            if ics.is_empty() || exempt(cx, net) {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .net(net)
                    .pins(conn_pins.iter().chain(&ics).copied())
                    .message(format!(
                        "{} goes straight from {} to {} with nothing in series: a fault on the cable drives the pin's clamp diodes",
                        b.net(net).name,
                        pin_list(cx, &conn_pins),
                        pin_list(cx, &ics)
                    ))
                    .fix("100 Ω-1 kΩ in series near the connector, behind the TVS, sized from the worst fault voltage and the pin's injection limit; a buffer with Ioff for lines that need drive")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Conn2));

#[cfg(test)]
mod tests {
    #[test]
    fn gpio_on_header() {
        let found = crate::testing::fingerprints(&super::Conn2, "conn-2_gpio_protection");
        assert_eq!(
            found,
            [
                "CONN-2:net /IO_A|pin J1.2|pin U1.3",
                "CONN-2:net /IO_C|pin J1.4|pin U1.5",
            ],
            "{found:?}"
        );
    }
}
