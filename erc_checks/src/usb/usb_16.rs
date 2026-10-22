//! `USB-16` Type-C receptacle with VBUS or GND pins unconnected

use super::{connected, type_c};
use crate::meta::*;
use crate::text::{and_list, pin_list};
use crate::{Ctx, Finding};
use erc_analysis::connector::{TypeCPin, type_c_pins};
use erc_model::{NetId, PinId};
use std::collections::BTreeSet;

pub struct Usb16;

static META: CheckMeta = CheckMeta {
    id: "USB-16",
    category: "USB",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Usb16 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// The VBUS contacts (A4, A9, B4, B9) of a Type-C receptacle all on one net, its ground contacts (A1, A12, B1,
    /// B12) all on one ground; a receptacle with none of them connected is left alone
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for part in type_c(cx, false) {
            let roles = [(TypeCPin::Vbus, "VBUS"), (TypeCPin::Gnd, "GND")];
            let pins: Vec<Vec<PinId>> = roles
                .iter()
                .map(|(r, _)| type_c_pins(b, part, *r))
                .collect();
            if pins.iter().flatten().all(|p| connected(cx, *p).is_none()) {
                continue;
            }
            for ((role, word), pins) in roles.iter().zip(&pins) {
                let open: Vec<PinId> = pins
                    .iter()
                    .copied()
                    .filter(|p| connected(cx, *p).is_none())
                    .collect();
                let nets: BTreeSet<NetId> = pins.iter().filter_map(|p| connected(cx, *p)).collect();
                let mut what = Vec::new();
                if !open.is_empty() {
                    what.push(format!("{} connected to nothing", pin_list(cx, &open)));
                }
                if nets.len() > 1 {
                    let names: Vec<&str> = nets.iter().map(|n| b.net(*n).name.as_str()).collect();
                    what.push(format!("on {} nets, {}", nets.len(), and_list(&names)));
                }
                if *role == TypeCPin::Gnd
                    && let Some(n) = nets.iter().find(|n| !cx.power().is_ground(**n))
                {
                    what.push(format!("{} is no ground", b.net(*n).name));
                }
                if what.is_empty() {
                    continue;
                }
                out.push(
                    cx.finding(&META)
                        .part(part)
                        .pins(open.iter().copied())
                        .nets(if nets.len() > 1 { nets.clone() } else { BTreeSet::new() })
                        .message(format!(
                            "Type-C receptacle {}: {} pins not bussed together: {}; the contacts left carry the cable's whole current",
                            b.part(part).designator,
                            word,
                            what.join("; ")
                        ))
                        .fix("connect every VBUS and GND contact, with copper sized for the current")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Usb16));

#[cfg(test)]
mod tests {
    #[test]
    fn contacts_left_open() {
        let found = crate::testing::fingerprints(&super::Usb16, "usb-16_vbus_gnd_pins");
        assert_eq!(
            found,
            [
                "USB-16:net /VBUS_B|net VBUS|part J3",
                "USB-16:part J1|pin J1.B12",
                "USB-16:part J1|pin J1.B4|pin J1.B9",
                "USB-16:part J3|pin J3.A12",
            ],
            "{found:?}"
        );
    }
}
