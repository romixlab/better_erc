//! `WIRE-11` Unnamed net with many pins or crossing sheets

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_model::{NetId, NetKind, PinId, SheetId};
use std::collections::BTreeSet;

pub struct Wire11;

static META: CheckMeta = CheckMeta {
    id: "WIRE-11",
    category: "WIRE",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[Param {
        name: "max_pins",
        default: 4.0,
        unit: "",
        doc: "the most pins an auto-named net may have",
    }],
};

impl Check for Wire11 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Auto-named nets (`Net-(R12-Pad2)`) with more than `max_pins` component pins, or with pins on more than one
    /// sheet
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let max = cx.param(&META, "max_pins");
        let sheet_of = |q: PinId| -> SheetId {
            let pin = b.pin(q);
            let part = b.part(pin.part);
            part.units
                .iter()
                .find(|u| u.unit == pin.unit)
                .map_or(part.sheet, |u| u.sheet)
        };
        let mut out = Vec::new();
        for i in 0..b.nets.len() {
            let net = NetId::from_index(i);
            if b.net(net).kind != NetKind::Auto {
                continue;
            }
            let pins: Vec<PinId> = net_pins(b, net).collect();
            let sheets: BTreeSet<SheetId> = pins.iter().map(|q| sheet_of(*q)).collect();
            let many = pins.len() as f64 > max;
            if !many && sheets.len() < 2 {
                continue;
            }
            let mut why = Vec::new();
            if many {
                why.push(format!("{} pins", pins.len()));
            }
            if sheets.len() > 1 {
                let paths: Vec<String> = sheets.iter().map(|s| b.sheet_path(*s)).collect();
                why.push(format!("pins on sheets {}", paths.join(", ")));
            }
            let mut f = cx.finding(&META).net(net);
            if many {
                f = f.fact("max_pins", max, cx.param_source(&META, "max_pins"));
            }
            out.push(
                f.message(format!(
                    "{} has no name ({}): it cannot be found again in a review, a layout note or a test plan",
                    b.net(net).name,
                    why.join("; ")
                ))
                .fix("label the net with a name that says what it carries")
                .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Wire11));

#[cfg(test)]
mod tests {
    #[test]
    fn unnamed_nets() {
        assert_eq!(
            crate::testing::fingerprints(&super::Wire11, "wire-11_unnamed_nets"),
            ["WIRE-11:net Net-(U1-FB)"]
        );
    }
}
