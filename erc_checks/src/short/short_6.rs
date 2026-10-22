//! `SHORT-6` Inputs shorted to ground or power without a resistor

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::power::{NameRole, read_name};
use erc_model::{EcadPinType, PartClass};

pub struct Short6;

static META: CheckMeta = CheckMeta {
    id: "SHORT-6",
    category: "SHORT",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Short6 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// IO pins (bidirectional or tri-state, as the symbol types them) of fitted ICs straight on a rail or a ground:
    /// a pin that can drive becomes a short when it drives the other level. Input pins of configurable parts wait
    /// for pin roles (NET-7).
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let power = cx.power();
        let mut out = Vec::new();
        for p in cx.view().fitted_components() {
            let class = classes.get(p);
            if !(class.is_ic() || class == PartClass::Unknown) {
                continue;
            }
            for q in &b.part(p).pins {
                let pin = b.pin(*q);
                let Some(net) = pin.net else { continue };
                let io = matches!(
                    pin.etype.get(),
                    Some(EcadPinType::Bidirectional | EcadPinType::TriState)
                );
                if !io
                    || !power.is_power_net(net)
                    || matches!(
                        read_name(&pin.name, false).role,
                        NameRole::Rail(_) | NameRole::Ground(_)
                    )
                {
                    continue;
                }
                let to = if power.is_ground(net) {
                    "ground"
                } else {
                    "rail"
                };
                out.push(
                    cx.finding(&META)
                        .pin(*q)
                        .part(p)
                        .net(net)
                        .message(format!(
                            "{} ({}, {}) is tied straight to the {to} {}: a short when it drives the other level",
                            b.pin_name(*q),
                            pin.name,
                            pin.etype.get().map_or("?", |t| t.name()),
                            b.net(net).name
                        ))
                        .fix("tie through 1-10 kΩ, or leave it open and set it to analog or output-low in firmware (LEAK-3)")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Short6));

#[cfg(test)]
mod tests {
    #[test]
    fn ios_tied_hard() {
        assert_eq!(
            crate::testing::fingerprints(&super::Short6, "short-6_io_tied_to_rail"),
            ["SHORT-6:net GND|part U1|pin U1.3"]
        );
    }
}
