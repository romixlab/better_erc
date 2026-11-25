//! `RATE` - part ratings against the stress the circuit puts on them, with the project's derating factors

mod rate_1;
mod rate_15;
mod rate_2;
mod rate_9;

use crate::component::{self, Candidate, Judged};
use crate::meta::CheckMeta;
use crate::{Ctx, Finding};
use erc_analysis::value::format_si;
use erc_model::{PartId, Provenance, Subject, Unit};

/// What the circuit puts on a part: a working voltage, a dissipated power, a peak current
pub(crate) struct Stress {
    pub value: f64,
    pub unit: Unit,
    /// `working`, `dissipated`, `peak`
    pub what: &'static str,
    /// How it is known (`+5V (5 V) to GND (0 V)`, `Ipeak field`)
    pub how: String,
    pub source: Provenance,
    /// The inputs it rests on
    pub inputs: Vec<(String, String)>,
}

impl From<component::Working> for Stress {
    fn from(w: component::Working) -> Stress {
        Stress {
            value: w.volts,
            unit: Unit::Volt,
            what: "working",
            how: w.how,
            source: w.source,
            inputs: w.inputs,
        }
    }
}

/// A stress from a field of the part (`Ipeak`, `Pworking`) read in `unit`
pub(crate) fn field_stress(
    cx: &Ctx,
    p: PartId,
    name: &str,
    unit: Unit,
    what: &'static str,
) -> Option<Stress> {
    let f = component::field(cx, p, name)?;
    let value = component::quantity(&f.text, unit)?.abs();
    let reference = &cx.board().part(p).designator;
    Some(Stress {
        value,
        unit,
        what,
        how: format!("{} field", f.from),
        source: f.source,
        inputs: vec![(format!("{reference}.{}", f.from), f.text)],
    })
}

/// Each MPN of `p` against `rated ≥ k × stress`: `fact` is the rating's key, `factor` picks the derating key for
/// an MPN or names the fact it needs to (a capacitor's technology)
pub(crate) fn judge_rating(
    cx: &Ctx,
    meta: &'static CheckMeta,
    p: PartId,
    stress: &Stress,
    fact: &'static str,
    factor: impl Fn(&Candidate) -> Result<&'static str, &'static str>,
    fix: &str,
) -> Vec<Finding> {
    let derating = cx.derating();
    let judged = component::candidates(cx, p)
        .iter()
        .map(|c| {
            let mut j = Judged::new(c);
            let key = match factor(c) {
                Ok(k) => k,
                Err(missing) => {
                    j.lacks(missing);
                    return j;
                }
            };
            let Some(rated) = c.number(fact) else {
                j.lacks(fact);
                return j;
            };
            let k = derating
                .get(key)
                .expect("the derating key a check names is a derating key");
            j.input(
                format!("{} {}", stress.what, stress.unit.name()),
                format!("{} ({})", format_si(stress.value, stress.unit), stress.how),
                stress.source.clone(),
            );
            j.inputs(&stress.inputs);
            j.input(
                format!("{}.{fact}", c.field),
                rated.to_string(),
                c.source(fact),
            );
            j.input(
                format!("derating.{key}"),
                k.to_string(),
                derating.source(key),
            );
            let what = format!("{} ({})", stress.what, stress.how);
            if let Some(line) = component::margin(rated, stress.value, k, stress.unit, &what) {
                j.miss(line);
            }
            j
        })
        .collect();
    component::report(cx, meta, p, judged, fix)
}

/// Each MPN of `p` against the part's minimum rating field (`Vreq_min`, `Preq_min`): the rating this board needs,
/// stated in place of `k` times the stress. Each rating must also exceed the stress itself where it is known, and a
/// minimum below the stress is a finding of its own. `None` when the part has no such field, or its text is no
/// `unit` quantity.
#[expect(
    clippy::too_many_arguments,
    reason = "one call per rating check, each argument a fact of it"
)]
pub(crate) fn judge_minimum(
    cx: &Ctx,
    meta: &'static CheckMeta,
    p: PartId,
    name: &str,
    unit: Unit,
    fact: &'static str,
    stress: Option<&Stress>,
    fix: &str,
) -> Option<Vec<Finding>> {
    let f = component::field(cx, p, name)?;
    let want = component::quantity(&f.text, unit)?.abs();
    let reference = &cx.board().part(p).designator;
    let mut out = Vec::new();
    if let Some(s) = stress
        && want < s.value
    {
        let mut b = cx
            .finding(meta)
            .part(p)
            .item(
                Subject::Part(cx.board_id, p),
                format!("{reference} {}", f.from),
            )
            .message(format!(
                "{reference}: {} {} is below the {} {} ({})",
                f.from,
                format_si(want, unit),
                s.what,
                format_si(s.value, unit),
                s.how
            ))
            .fact(&f.from, &f.text, f.source.clone())
            .fact(s.what, format_si(s.value, unit), s.source.clone())
            .input(format!("{reference}.{}", f.from), &f.text)
            .fix(format!(
                "raise {} to the rating this board needs, above the {} value",
                f.from, s.what
            ));
        for (n, v) in &s.inputs {
            b = b.input(n, v);
        }
        out.push(b.build());
    }
    let judged = component::candidates(cx, p)
        .iter()
        .map(|c| {
            let mut j = Judged::new(c);
            let Some(rated) = c.number(fact) else {
                j.lacks(fact);
                return j;
            };
            j.input(
                format!("{reference}.{}", f.from),
                f.text.clone(),
                f.source.clone(),
            );
            j.input(
                format!("{}.{fact}", c.field),
                rated.to_string(),
                c.source(fact),
            );
            if rated < want {
                j.miss(format!(
                    "{} rated, {} asks {}",
                    format_si(rated, unit),
                    f.from,
                    format_si(want, unit)
                ));
            }
            if let Some(s) = stress {
                j.inputs(&s.inputs);
                if rated <= s.value {
                    j.miss(format!(
                        "{} rated, not above the {} {} ({})",
                        format_si(rated, unit),
                        s.what,
                        format_si(s.value, unit),
                        s.how
                    ));
                }
            }
            j
        })
        .collect();
    out.extend(component::report(cx, meta, p, judged, fix));
    Some(out)
}
