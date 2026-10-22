//! `ANA-10` Op-amp with no feedback path, or feedback into the non-inverting input

use super::{connected, units, walk};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::PartClass;

pub struct Ana10;

static META: CheckMeta = CheckMeta {
    id: "ANA-10",
    category: "ANA",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// The parts a feedback path runs through: passives, diodes and transistors (a precision rectifier, a V-to-I
/// stage), never another IC
fn feedback_part(class: PartClass) -> bool {
    class.is_resistor()
        || class.is_capacitor()
        || class.is_dc_tie()
        || class.is_diode()
        || class.is_transistor()
        || class == PartClass::Inductor
}

impl Check for Ana10 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each op-amp channel (pins by name: `+`, `-`, `IN+`, `OUT`, KiCad's `~` output) whose output, inverting and
    /// non-inverting inputs are all connected: from the output through resistors, capacitors, inductors, diodes
    /// and transistors (not through rails and grounds), the inverting input must be reached; reaching only the
    /// non-inverting one is positive feedback. A channel with an input or its output unconnected is ANA-11's.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for u in units(cx, PartClass::Opamp) {
            let (Some(o), Some(n), Some(p)) = (
                connected(cx, u.out),
                connected(cx, u.inn),
                connected(cx, u.inp),
            ) else {
                continue;
            };
            let reached = walk(cx, o, &|part| feedback_part(cx.classes().get(part)));
            let negative = o == n || (!power.is_power_net(n) && reached.contains(&n));
            if negative {
                continue;
            }
            let positive = !power.is_power_net(p) && reached.contains(&p);
            let what = if positive {
                "its only feedback goes to the non-inverting input: positive feedback, a latch"
            } else {
                "no path from its output back to the inverting input: open loop, the output sits at a rail"
            };
            out.push(
                cx.finding(&META)
                    .part(u.part)
                    .pins(u.out.into_iter().chain(u.inn))
                    .message(format!("op-amp {}: {what}", u.label(b)))
                    .fix("connect the feedback network to the inverting input; if a comparator was meant, use a comparator")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Ana10));

#[cfg(test)]
mod tests {
    #[test]
    fn missing_negative_feedback() {
        let found = crate::testing::fingerprints(&super::Ana10, "ana-10_opamp_feedback");
        assert_eq!(
            found,
            [
                "ANA-10:part U1|pin U1.1|pin U1.2",
                "ANA-10:part U1|pin U1.6|pin U1.7"
            ],
            "{found:?}"
        );
    }
}
