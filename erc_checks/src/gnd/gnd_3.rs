//! `GND-3` Analog and digital grounds joined at more than one point, or not at all

use super::designators;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::power::{GroundKind, Joint};
use erc_model::{NetId, PartId};
use std::collections::BTreeSet;

pub struct Gnd3;

static META: CheckMeta = CheckMeta {
    id: "GND-3",
    category: "GND",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Gnd3 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each analog ground on a board that also has a digital, signal or power ground: the fitted parts joining
    /// it to them (a net tie, jumper, 0 Ω, ferrite or inductor, or an IC with pins on both) must be exactly one
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let other = |k: GroundKind| {
            matches!(
                k,
                GroundKind::Digital | GroundKind::Signal | GroundKind::Power
            )
        };
        let others: BTreeSet<NetId> = power
            .grounds
            .iter()
            .filter(|(_, g)| other(g.kind))
            .map(|(n, _)| *n)
            .collect();
        let mut out = Vec::new();
        for (agnd, g) in &power.grounds {
            if g.kind != GroundKind::Analog || others.is_empty() {
                continue;
            }
            let mut joints: BTreeSet<PartId> = BTreeSet::new();
            let mut joined: BTreeSet<NetId> = BTreeSet::new();
            for link in &power.ground_links {
                let ic = link.joint == Joint::Part && cx.classes().get(link.part).is_ic();
                if !link.grounds.contains(agnd) || !(link.joint.dc() || ic) {
                    continue;
                }
                let to: Vec<NetId> = link.grounds.intersection(&others).copied().collect();
                if !to.is_empty() {
                    joints.insert(link.part);
                    joined.extend(to);
                }
            }
            if joints.len() == 1 {
                continue;
            }
            let name = &b.net(*agnd).name;
            let f = cx.finding(&META).net(*agnd);
            let f = if joints.is_empty() {
                let names: Vec<&str> = others.iter().map(|n| b.net(*n).name.as_str()).collect();
                f.nets(others.iter().copied()).message(format!(
                    "{name} is joined to {} nowhere: the analog side floats and its signals return through parasitic paths",
                    names.join(", ")
                ))
            } else {
                let names: Vec<&str> = joined.iter().map(|n| b.net(*n).name.as_str()).collect();
                f.nets(joined.iter().copied())
                    .parts(joints.iter().copied())
                    .message(format!(
                        "{name} is joined to {} at {} points ({}): digital return current flows through the analog side",
                        names.join(", "),
                        joints.len(),
                        designators(cx, joints.iter().copied())
                    ))
            };
            out.push(
                f.fix("one joint, at the mixed-signal part or the supply entry; or one plane and no split (GND-1)")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Gnd3));

#[cfg(test)]
mod tests {
    #[test]
    fn split_grounds() {
        let found = crate::testing::fingerprints(&super::Gnd3, "gnd-3_split_ground_joints");
        assert_eq!(
            found,
            [
                "GND-3:net AGND|net GND|part FB1|part NT1|part U1",
                "GND-3:net GND|net GNDA"
            ],
            "{found:?}"
        );
    }
}
