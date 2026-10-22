//! `SHORT-7` Shorted IOs

use super::{drives, programmable};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_model::{EcadPinType, NetId, PinId};
use std::collections::BTreeSet;

pub struct Short7;

static META: CheckMeta = CheckMeta {
    id: "SHORT-7",
    category: "SHORT",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Short7 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Signal nets with two or more IO pins (bidirectional, tri-state) of fitted MCUs, SoCs or FPGAs and no other
    /// driver; I2C lines (open drain by design) are left out
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let power = cx.power();
        let i2c: BTreeSet<NetId> = cx
            .i2c()
            .buses
            .iter()
            .flat_map(|bus| [bus.scl, bus.sda])
            .collect();
        let mut out = Vec::new();
        for i in 0..b.nets.len() {
            let net = NetId::from_index(i);
            if power.is_power_net(net) || i2c.contains(&net) {
                continue;
            }
            let pins: Vec<PinId> = net_pins(b, net)
                .filter(|q| cx.fitted(b.pin(*q).part))
                .collect();
            let io: Vec<PinId> = pins
                .iter()
                .copied()
                .filter(|q| {
                    let pin = b.pin(*q);
                    programmable(classes.get(pin.part))
                        && matches!(
                            pin.etype.get(),
                            Some(EcadPinType::Bidirectional | EcadPinType::TriState)
                        )
                })
                .collect();
            let other_driver = pins
                .iter()
                .any(|q| !io.contains(q) && drives(b.pin(*q).etype.get()));
            if io.len() < 2 || other_driver {
                continue;
            }
            let names: Vec<String> = io
                .iter()
                .map(|q| format!("{} ({})", b.pin_name(*q), b.pin(*q).name))
                .collect();
            out.push(
                cx.finding(&META)
                    .net(net)
                    .parts(io.iter().map(|q| b.pin(*q).part))
                    .message(format!(
                        "{}: IOs {} are tied together; firmware must never drive them against each other",
                        b.net(net).name,
                        names.join(", ")
                    ))
                    .fix("note which pin drives and which stays an input, on the schematic and in the pin map; or put ≥ 330 Ω in series")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Short7));

#[cfg(test)]
mod tests {
    #[test]
    fn ios_tied_together() {
        assert_eq!(
            crate::testing::fingerprints(&super::Short7, "short-7_shorted_ios"),
            ["SHORT-7:net /SPI_SCK|part U1|part U2"]
        );
    }
}
