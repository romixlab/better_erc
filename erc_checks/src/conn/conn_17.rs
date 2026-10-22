//! `CONN-17` Two identical connectors on one board with incompatible pinouts

use super::{connectors, contacts};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::{NetId, PartId, natural_cmp};
use std::collections::BTreeMap;

pub struct Conn17;

static META: CheckMeta = CheckMeta {
    id: "CONN-17",
    category: "CONN",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// The fields a part number is written in
const MPN_FIELDS: &[&str] = &[
    "MPN",
    "Mpn1",
    "MPN1",
    "Manufacturer_Part_Number",
    "Manufacturer Part Number",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Power,
    Ground,
    Signal,
}

impl Check for Conn17 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Fitted connectors grouped by footprint (else part number); within a group, two connectors where a pin number
    /// is on a rail on one and on a ground or a signal on the other
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let view = cx.view();
        let mut groups: BTreeMap<String, Vec<PartId>> = BTreeMap::new();
        for part in connectors(cx) {
            let key = view
                .field(part, "Footprint")
                .filter(|f| !f.is_empty())
                .map(|f| format!("footprint {f}"))
                .or_else(|| {
                    MPN_FIELDS
                        .iter()
                        .find_map(|f| view.field(part, f).filter(|v| !v.is_empty()))
                        .map(|m| format!("part number {m}"))
                });
            if let Some(key) = key {
                groups.entry(key).or_default().push(part);
            }
        }
        let roles = |part: PartId| -> BTreeMap<String, (Role, NetId)> {
            contacts(cx, part)
                .into_iter()
                .map(|(pin, net)| {
                    let role = if power.is_ground(net) {
                        Role::Ground
                    } else if power.is_rail(net) {
                        Role::Power
                    } else {
                        Role::Signal
                    };
                    (b.pin(pin).number.clone(), (role, net))
                })
                .collect()
        };
        let mut out = Vec::new();
        for (key, mut parts) in groups {
            parts.sort_by(|x, y| natural_cmp(&b.part(*x).designator, &b.part(*y).designator));
            for (i, &x) in parts.iter().enumerate() {
                for &y in &parts[i + 1..] {
                    let (rx, ry) = (roles(x), roles(y));
                    let mut clashes: Vec<(&String, NetId, NetId)> = rx
                        .iter()
                        .filter_map(|(number, (r1, n1))| {
                            let (r2, n2) = ry.get(number)?;
                            let clash = (*r1 == Role::Power && *r2 != Role::Power)
                                || (*r2 == Role::Power && *r1 != Role::Power);
                            clash.then_some((number, *n1, *n2))
                        })
                        .collect();
                    if clashes.is_empty() {
                        continue;
                    }
                    clashes.sort_by(|a, c| natural_cmp(a.0, c.0));
                    let list: Vec<String> = clashes
                        .iter()
                        .map(|(number, n1, n2)| {
                            format!(
                                "pin {number}: {} on {}, {} on {}",
                                b.net(*n1).name,
                                b.part(x).designator,
                                b.net(*n2).name,
                                b.part(y).designator
                            )
                        })
                        .collect();
                    out.push(
                        cx.finding(&META)
                            .parts([x, y])
                            .message(format!(
                                "{} and {} are the same connector ({key}) with power in different places: {}; a cable or module meant for one fits the other",
                                b.part(x).designator,
                                b.part(y).designator,
                                list.join("; ")
                            ))
                            .fix("the same power and ground positions on every connector of a type, or different connector types, keyings or colours for different functions")
                            .build(),
                    );
                }
            }
        }
        out
    }
}

inventory::submit!(Registered(&Conn17));

#[cfg(test)]
mod tests {
    #[test]
    fn look_alike_connectors() {
        let found = crate::testing::fingerprints(&super::Conn17, "conn-17_look_alike_pinouts");
        assert_eq!(
            found,
            ["CONN-17:part J5|part J6", "CONN-17:part J6|part J9",],
            "{found:?}"
        );
    }
}
