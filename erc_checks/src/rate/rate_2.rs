//! `RATE-2` Resistor power rating

use super::{Stress, field_stress, judge_minimum, judge_rating};
use crate::component;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::PartClass;
use erc_analysis::value::format_si;
use erc_model::{PartId, Provenance, Unit};

pub struct Rate2;

static META: CheckMeta = CheckMeta {
    id: "RATE-2",
    category: "RATE",
    severity: Severity::Warning,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::PartsLock],
    overlaps: &[],
    params: &[],
};

/// How to fix a miss
const FIX: &str =
    "a larger package, a higher value where the function allows, or resistors sharing the power";

impl Check for Rate2 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each MPN of a fitted resistor whose dissipation is known (its `Pworking` field, else V²/R from its
    /// working voltage and value): rated power at least `derating.resistor_p` times the dissipation; a `Preq_min`
    /// field states the rating needed in its place
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let mut out = Vec::new();
        for p in component::fitted_sorted(cx) {
            if cx.classes().get(p) != PartClass::Resistor {
                continue;
            }
            let stress = field_stress(cx, p, "Pworking", Unit::Watt, "dissipated")
                .or_else(|| from_voltage(cx, p));
            if let Some(f) = judge_minimum(
                cx,
                &META,
                p,
                "Preq_min",
                Unit::Watt,
                "power",
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
                "power",
                |_| Ok("resistor_p"),
                FIX,
            ));
        }
        out
    }
}

/// V²/R: the working voltage across the resistor and its value
fn from_voltage(cx: &Ctx, p: PartId) -> Option<Stress> {
    let w = component::working_voltage(cx, p)?;
    let pv = cx.values().get(p);
    let r = pv
        .value
        .filter(|r| *r > 0.0 && pv.unit == Some(Unit::Ohm))?;
    let reference = &cx.board().part(p).designator;
    let mut inputs = w.inputs;
    inputs.push((format!("{reference}.{}", pv.label()), r.to_string()));
    Some(Stress {
        value: w.volts * w.volts / r,
        unit: Unit::Watt,
        what: "dissipated",
        how: format!(
            "{}² / {} across {}",
            format_si(w.volts, Unit::Volt),
            format_si(r, Unit::Ohm),
            w.how
        ),
        source: if w.source.is_estimate() {
            w.source
        } else {
            Provenance::Derived {
                rule: "V²/R from the working voltage and the value".to_owned(),
            }
        },
        inputs,
    })
}

inventory::submit!(Registered(&Rate2));
