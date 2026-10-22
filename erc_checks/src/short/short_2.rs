//! `SHORT-2` Accidental net name collision

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::{net_degree, series_paths};
use erc_model::{NetId, PartClass, PartId, PinId};
use std::collections::BTreeSet;

pub struct Short2;

static META: CheckMeta = CheckMeta {
    id: "SHORT-2",
    category: "SHORT",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[Param {
        name: "max_chain",
        default: 4.0,
        unit: "",
        doc: "the longest chain of parts followed from a net back to itself",
    }],
};

/// Pins that sit on the two sides of a part and must be on two nets: A1/B1 of a translator, SCL1/SCL2 and SDA1/SDA2
/// of a pass-FET translator, IN/OUT of a load switch
fn sides(cx: &Ctx, part: PartId) -> Vec<(PinId, PinId)> {
    let b = cx.board();
    let class = cx.classes().get(part);
    let pins = &b.part(part).pins;
    let name = |q: &PinId| b.pin(*q).name.to_uppercase();
    let find = |n: &str| pins.iter().copied().find(|q| name(q) == n);
    let mut out = Vec::new();
    match class {
        PartClass::Translator { .. } => {
            for q in pins {
                let n = name(q);
                let other = if let Some(i) = n
                    .strip_prefix('A')
                    .filter(|i| i.chars().all(|c| c.is_ascii_digit()))
                {
                    find(&format!("B{i}"))
                } else if let Some(w) = n.strip_suffix('1').filter(|w| matches!(*w, "SCL" | "SDA"))
                {
                    find(&format!("{w}2"))
                } else {
                    None
                };
                if let Some(o) = other {
                    out.push((*q, o));
                }
            }
        }
        PartClass::LoadSwitch => {
            for (a, c) in [("IN", "OUT"), ("VIN", "VOUT")] {
                if let (Some(a), Some(c)) = (find(a), find(c)) {
                    out.push((a, c));
                }
            }
        }
        _ => {}
    }
    out
}

impl Check for Short2 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Loops of passives through nodes nothing else uses (`VCCA`-R-R-`VCCA`), and the two sides of a translator
    /// or a load switch on one net
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let max = cx.param(&META, "max_chain") as usize;
        let mut out = Vec::new();
        let passive = |p: PartId| {
            matches!(
                classes.get(p),
                PartClass::Resistor
                    | PartClass::Capacitor { .. }
                    | PartClass::Inductor
                    | PartClass::FerriteBead
                    | PartClass::Jumper
                    | PartClass::NetTie
            )
        };
        // A node of a chain: only the two parts of the chain on it, and no rail or ground
        let power = cx.power();
        let node = |n: NetId| net_degree(b, n) == 2 && !power.is_power_net(n);
        let mut seen: BTreeSet<BTreeSet<PartId>> = BTreeSet::new();
        for i in 0..b.nets.len() {
            let start = NetId::from_index(i);
            for path in series_paths(b, start, &passive, &node, max) {
                if path.end != start || path.parts.len() < 2 {
                    continue;
                }
                let set: BTreeSet<PartId> = path.parts.iter().copied().collect();
                if !seen.insert(set) {
                    continue;
                }
                let names: Vec<&str> = path
                    .parts
                    .iter()
                    .map(|p| b.part(*p).designator.as_str())
                    .collect();
                out.push(
                    cx.finding(&META)
                        .net(start)
                        .parts(path.parts.iter().copied())
                        .message(format!(
                            "{} in series lead from {} back to {}: the chain bridges nothing",
                            names.join(", "),
                            b.net(start).name,
                            b.net(start).name
                        ))
                        .fix("one end was meant for another net: rename the label on that side")
                        .build(),
                );
            }
        }
        for p in cx.components() {
            for (a, c) in sides(cx, p) {
                let (na, nc) = (b.pin(a).net, b.pin(c).net);
                if na.is_none() || na != nc {
                    continue;
                }
                let net = na.expect("connected");
                out.push(
                    cx.finding(&META)
                        .part(p)
                        .net(net)
                        .message(format!(
                            "{} ({}): {} and {} are both on {}, the part is bypassed",
                            b.part(p).designator,
                            classes.get(p).name(),
                            b.pin(a).name,
                            b.pin(c).name,
                            b.net(net).name
                        ))
                        .fix("the two sides never share a name: rename one (`SDA_1V8`)")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Short2));

#[cfg(test)]
mod tests {
    #[test]
    fn collisions() {
        assert_eq!(
            crate::testing::fingerprints(&super::Short2, "short-2_name_collision"),
            [
                "SHORT-2:net /SDA|part U1",
                "SHORT-2:net VCCA|part R1|part R2",
            ]
        );
    }
}
