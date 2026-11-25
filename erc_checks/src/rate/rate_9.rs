//! `RATE-9` Resistor voltage rating

use super::{Stress, judge_minimum, judge_rating};
use crate::component;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::PartClass;
use erc_model::Unit;

pub struct Rate9;

static META: CheckMeta = CheckMeta {
    id: "RATE-9",
    category: "RATE",
    severity: Severity::Warning,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::PartsLock],
    overlaps: &[],
    params: &[],
};

/// How to fix a miss
const FIX: &str = "several resistors in series, a larger package, or a part rated for the voltage";

impl Check for Rate9 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each MPN of a fitted resistor whose working voltage is known (its `Vworking` field, else its two nets):
    /// rated (limiting element) voltage at least `derating.resistor_v` times the working voltage; a `Vreq_min`
    /// field states the rating needed in its place
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let mut out = Vec::new();
        for p in component::fitted_sorted(cx) {
            if cx.classes().get(p) != PartClass::Resistor {
                continue;
            }
            let stress = component::working_voltage(cx, p).map(Stress::from);
            if let Some(f) = judge_minimum(
                cx,
                &META,
                p,
                "Vreq_min",
                Unit::Volt,
                "rated_voltage",
                stress.as_ref(),
                FIX,
            ) {
                out.extend(f);
                continue;
            }
            let Some(s) = stress else {
                continue;
            };
            out.extend(judge_rating(
                cx,
                &META,
                p,
                &s,
                "rated_voltage",
                |_| Ok("resistor_v"),
                FIX,
            ));
        }
        out
    }
}

inventory::submit!(Registered(&Rate9));
