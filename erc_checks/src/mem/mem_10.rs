//! `MEM-10` DDR RESET_n without a pull-down and CKE without a defined level at power-up

use super::line_pulls;
use crate::meta::*;
use crate::text::pin_list;
use crate::usb::connected;
use crate::{Ctx, Finding};
use erc_analysis::interface::{DdrPin, ddr_pin};
use erc_model::class::MemoryKind;
use erc_model::{PartClass, PinId};

pub struct Mem10;

static META: CheckMeta = CheckMeta {
    id: "MEM-10",
    category: "MEM",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Mem10 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A fitted DRAM with a reset pin (DDR3, DDR4: SDR and DDR2 parts have none) whose RESET or CKE line (the net and
    /// one series resistor away) has no pull-down to a ground
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for part in cx.view().fitted_components() {
            if cx.classes().get(part)
                != (PartClass::Memory {
                    kind: Some(MemoryKind::Dram),
                })
            {
                continue;
            }
            let pins: Vec<(PinId, DdrPin)> = b
                .part(part)
                .pins
                .iter()
                .filter_map(|p| Some((*p, ddr_pin(&b.pin(*p).name)?)))
                .filter(|(_, f)| matches!(f, DdrPin::Reset | DdrPin::Cke))
                .collect();
            if !pins.iter().any(|(_, f)| *f == DdrPin::Reset) {
                continue;
            }
            for (pin, function) in pins {
                let net = connected(cx, pin);
                if net.is_some_and(|n| {
                    cx.power().is_ground(n) || !line_pulls(cx, n, false).is_empty()
                }) {
                    continue;
                }
                let what = match function {
                    DdrPin::Reset => "RESET_n must be low while the supplies ramp",
                    _ => "CKE must be low until the controller starts initialisation",
                };
                let mut f = cx.finding(&META).part(part).pin(pin);
                if let Some(n) = net {
                    f = f.net(n);
                }
                out.push(
                    f.message(format!(
                        "DRAM {}: {} {} has no pull-down: {}, and the controller drives nothing until its own supply and reset are up",
                        b.part(part).designator,
                        pin_list(cx, &[pin]),
                        net.map_or("(connected to nothing)".to_string(), |n| format!("({})", b.net(n).name)),
                        what
                    ))
                    .fix("10 kΩ pull-downs on RESET_n and CKE, near the DRAM")
                    .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Mem10));

#[cfg(test)]
mod tests {
    #[test]
    fn reset_and_cke_pull_downs() {
        let found = crate::testing::fingerprints(&super::Mem10, "mem-10_ddr_reset_cke");
        assert_eq!(
            found,
            [
                "MEM-10:net /DDR_CKE|part U2|pin U2.5",
                "MEM-10:net /DDR_RESETN2|part U3|pin U3.4",
            ],
            "{found:?}"
        );
    }
}
