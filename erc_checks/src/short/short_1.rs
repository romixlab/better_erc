//! `SHORT-1` Net shunting a known part

use super::{pins_named, two_terminal};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::part_nets;
use erc_model::{NetId, PartId, PinId};
use std::collections::BTreeSet;

pub struct Short1;

static META: CheckMeta = CheckMeta {
    id: "SHORT-1",
    category: "SHORT",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

/// Terminal pairs of transistors that must not share a net: gate-source, drain-source, base-emitter
const PAIRS: [(&[&str], &[&str]); 3] = [
    (&["G", "GATE"], &["S", "SOURCE"]),
    (&["D", "DRAIN"], &["S", "SOURCE"]),
    (&["B", "BASE"], &["E", "EMITTER"]),
];

impl Check for Short1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Two-terminal parts with both terminals on one net, transistors with G-S, D-S or B-E on one net; fitted or
    /// not, since the collision is in the drawing
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let mut out = Vec::new();
        for p in cx.components() {
            let class = classes.get(p);
            let part = b.part(p);
            let nets_of = |pins: &[PinId]| -> BTreeSet<NetId> {
                pins.iter().filter_map(|q| b.pin(*q).net).collect()
            };
            let shunted: Option<(NetId, String)> = if two_terminal(class) {
                let numbers: BTreeSet<&str> = part
                    .pins
                    .iter()
                    .map(|q| b.pin(*q).number.as_str())
                    .collect();
                let nets = part_nets(b, p);
                let all_connected = part.pins.iter().all(|q| b.pin(*q).net.is_some());
                (numbers.len() == 2 && nets.len() == 1 && all_connected).then(|| {
                    (
                        *nets.iter().next().expect("one net"),
                        "both terminals".to_string(),
                    )
                })
            } else if class.is_transistor() {
                PAIRS.iter().find_map(|(a, c)| {
                    let (na, nc) = (nets_of(&pins_named(b, p, a)), nets_of(&pins_named(b, p, c)));
                    na.intersection(&nc)
                        .next()
                        .map(|n| (*n, format!("{} and {}", a[0], c[0])))
                })
            } else {
                None
            };
            let Some((net, what)) = shunted else {
                continue;
            };
            out.push(finding(cx, p, net, &what));
        }
        out
    }
}

fn finding(cx: &Ctx, p: PartId, net: NetId, what: &str) -> Finding {
    let b = cx.board();
    let class = cx.classes().fact(p);
    cx.finding(&META)
        .part(p)
        .net(net)
        .message(format!(
            "{} ({}): {what} on {}, the part is shorted out",
            b.part(p).designator,
            class.value.name(),
            b.net(net).name
        ))
        .fact("class", class.value.name(), class.source.clone())
        .fix("find the label that merged the two nets (WIRE-2, WIRE-12) and rename it")
        .build()
}

inventory::submit!(Registered(&Short1));

#[cfg(test)]
mod tests {
    #[test]
    fn shunted_parts() {
        assert_eq!(
            crate::testing::fingerprints(&super::Short1, "short-1_shunted_parts"),
            [
                "SHORT-1:net /VBAT|part R12",
                "SHORT-1:net GND|part Q2",
                "SHORT-1:net Net-(Q1-G)|part Q1",
            ]
        );
    }
}
