//! `BOM-12` Value field vs the MPN's parameters

use crate::component::{self, Candidate, Dielectrics, Judged, Ratings, Stated, dielectric};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::PartClass;
use erc_analysis::value::{PartValue, format_si};
use erc_model::{PartId, Provenance, Subject, Unit};

pub struct Bom12;

static META: CheckMeta = CheckMeta {
    id: "BOM-12",
    category: "BOM",
    severity: Severity::Error,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::PartsLock],
    overlaps: &[],
    params: &[],
};

/// Two values closer than this, relative, are the same (a value written `4k7` and a fact of 4700.0000001)
const SAME_REL: f64 = 1e-6;

impl Check for Bom12 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Every MPN of each fitted resistor, capacitor and inductor against what the part requires: its value (within
    /// the MPN's tolerance) and tolerance. And the part's display fields, the worst over its MPNs exactly: `Voltage`
    /// and `Power` the lowest rating, `Dielectric` the worst code; a part without the field is held to the board's
    /// `DEFAULT_*` text variable.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let mut out = Vec::new();
        for p in component::fitted_sorted(cx) {
            let (key, unit) = match cx.classes().get(p) {
                PartClass::Resistor => ("resistance", Unit::Ohm),
                PartClass::Capacitor { .. } => ("capacitance", Unit::Farad),
                PartClass::Inductor => ("inductance", Unit::Henry),
                _ => continue,
            };
            let cands = component::candidates(cx, p);
            if cands.is_empty() {
                continue;
            }
            let reference = &cx.board().part(p).designator;
            let pv = cx.values().get(p);
            let wants = Wants::of(cx, p, reference, pv, unit);
            let mut judged: Vec<Judged> = cands
                .iter()
                .map(|c| judge(c, &wants, reference, key, unit))
                .collect();
            let shown = Shown {
                cx,
                p,
                reference,
                cands: &cands,
            };
            for (stated, fact, u) in [
                (&wants.voltage, "rated_voltage", Unit::Volt),
                (&wants.power, "power", Unit::Watt),
            ] {
                if let Some((want, s)) = stated {
                    out.extend(shown.lowest(&mut judged, *want, s, fact, u));
                }
            }
            if let Some(s) = &wants.dielectric {
                out.extend(shown.worst_dielectric(&mut judged, s));
            }
            out.extend(component::report(
                cx,
                &META,
                p,
                judged,
                "correct the MPN to one of the part's value and tolerance",
            ));
        }
        out
    }
}

/// A part's display fields against its MPNs
struct Shown<'s, 'a, 'c> {
    cx: &'s Ctx<'a>,
    p: PartId,
    reference: &'s str,
    cands: &'s [Candidate<'c>],
}

impl Shown<'_, '_, '_> {
    /// A display finding on the field `name` of the part: it names the value to write
    fn finding(
        &self,
        name: &str,
        s: &Stated,
        message: String,
        facts: Vec<(String, String, Provenance)>,
    ) -> Finding {
        let mut f = self
            .cx
            .finding(&META)
            .part(self.p)
            .item(Subject::Part(self.cx.board_id, self.p), format!("{} {name}", self.reference))
            .message(message)
            .fix(format!(
                "write the {name} field as the finding says, or change the MPNs; the field states the worst of them"
            ))
            .fact(&s.from, &s.text, s.source.clone())
            .input(format!("{}.{}", self.reference, s.from), &s.text);
        for (n, v, src) in facts {
            f = f.input(&n, &v).fact(n, v, src);
        }
        f.build()
    }

    /// `Voltage` or `Power` equals the lowest rating among the MPNs; an MPN without the rating leaves it not checked
    fn lowest(
        &self,
        judged: &mut [Judged],
        want: f64,
        s: &Stated,
        fact: &'static str,
        unit: Unit,
    ) -> Option<Finding> {
        let ratings = Ratings::of(self.cands, fact);
        for i in &ratings.lacking {
            judged[*i].lacks(fact);
        }
        let (low, by) = ratings.lowest()?;
        if (want - low).abs() <= SAME_REL * low.abs() {
            return None;
        }
        let name = if fact == "power" { "Power" } else { "Voltage" };
        let write = format_si(low, unit).replace(' ', "");
        let facts = ratings
            .known
            .iter()
            .map(|(v, c)| (format!("{}.{fact}", c.field), v.to_string(), c.source(fact)))
            .collect();
        Some(self.finding(
            name,
            s,
            format!(
                "{}: {} states {}, the lowest rating of its MPNs is {} ({} {}): write {name} {write}",
                self.reference,
                s.from,
                format_si(want, unit),
                format_si(low, unit),
                by.field,
                by.mpn,
            ),
            facts,
        ))
    }

    /// `Dielectric` equals the worst code among the ceramic MPNs: the one every other covers
    fn worst_dielectric(&self, judged: &mut [Judged], s: &Stated) -> Option<Finding> {
        let found = Dielectrics::of(self.cands);
        for i in &found.lacking {
            judged[*i].lacks("dielectric");
        }
        // A code no reading knows is not judged
        if !found.judgeable() || dielectric::covers(&s.text, &s.text).is_none() {
            return None;
        }
        let worst = found.worst();
        if worst.is_some_and(|w| component::same_dielectric(&s.text, w)) {
            return None;
        }
        let listed: Vec<String> = found
            .codes
            .iter()
            .map(|(d, c)| format!("{} {d}", c.field))
            .collect();
        let write = match worst {
            Some(w) => format!(": write Dielectric {w}"),
            None => ": no one of them is worse than the others".to_owned(),
        };
        let facts = found
            .codes
            .iter()
            .map(|(d, c)| {
                (
                    format!("{}.dielectric", c.field),
                    (*d).to_owned(),
                    c.source("dielectric"),
                )
            })
            .collect();
        Some(self.finding(
            "Dielectric",
            s,
            format!(
                "{}: {} states {}, its MPNs are {}{write}",
                self.reference,
                s.from,
                s.text,
                listed.join(", ")
            ),
            facts,
        ))
    }
}

/// What a part states: requirements each MPN must meet, and display fields stating the worst of its MPNs
struct Wants {
    /// The value in its class's unit, and where it is written
    value: Option<(f64, String, Provenance)>,
    tolerance: Option<(f64, Stated)>,
    dielectric: Option<Stated>,
    voltage: Option<(f64, Stated)>,
    power: Option<(f64, Stated)>,
}

impl Wants {
    fn of(cx: &Ctx, p: PartId, reference: &str, pv: &PartValue, unit: Unit) -> Wants {
        let value_src = pv.source.clone().unwrap_or(Provenance::Derived {
            rule: "Value field".to_owned(),
        });
        let value = match (pv.value, pv.unit) {
            (Some(v), Some(u)) if u == unit && v > 0.0 => Some((v, pv.label(), value_src.clone())),
            _ => None,
        };
        let from_value = |text: String, what: &str| Stated {
            text,
            from: format!("{} ({what})", pv.label()),
            source: value_src.clone(),
        };
        let stated = |name: &str| component::stated(cx, p, name, None);
        let with_default =
            |name: &str| component::stated(cx, p, name, component::default_var(reference, name));
        let tolerance = stated("Tolerance")
            .and_then(|s| percent(&s.text).map(|t| (t, s)))
            .or_else(|| {
                pv.tolerance
                    .map(|t| (t, from_value(format!("{t}%"), "tolerance")))
            });
        let dielectric = if unit == Unit::Farad {
            stated("Dielectric")
                .or_else(|| pv.dielectric.clone().map(|d| from_value(d, "dielectric")))
                .or_else(|| with_default("Dielectric"))
        } else {
            None
        };
        let rating = |name: &str, u: Unit| {
            stated(name)
                .and_then(|s| component::quantity(&s.text, u).map(|v| (v, s)))
                .or_else(|| {
                    pv.ratings
                        .iter()
                        .find(|q| q.unit == u)
                        .map(|q| (q.value, from_value(format_si(q.value, u), "rating")))
                })
                .or_else(|| {
                    with_default(name).and_then(|s| component::quantity(&s.text, u).map(|v| (v, s)))
                })
        };
        Wants {
            value,
            tolerance,
            dielectric,
            voltage: rating("Voltage", Unit::Volt),
            power: rating("Power", Unit::Watt),
        }
    }
}

/// `1%`, `±5 %`, `0.1` read as percent
fn percent(text: &str) -> Option<f64> {
    let t: String = text
        .chars()
        .filter(|c| !matches!(c, '±' | '%' | '+' | ' '))
        .collect();
    t.parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0)
}

fn judge<'c>(
    c: &Candidate<'c>,
    w: &Wants,
    reference: &str,
    key: &'static str,
    unit: Unit,
) -> Judged<'c> {
    let mut j = Judged::new(c);
    if let Some((want, label, src)) = &w.value {
        match c.number(key) {
            None => j.lacks(key),
            Some(got) => {
                j.input(
                    format!("{reference}.{label}"),
                    want.to_string(),
                    src.clone(),
                );
                j.input(format!("{}.{key}", c.field), got.to_string(), c.source(key));
                let apart_pct = (got - want).abs() / want * 100.0;
                if apart_pct > SAME_REL * 100.0 {
                    match c.tolerance_pct(Some(got)) {
                        None => j.lacks("tolerance"),
                        Some(tol) => {
                            j.input(
                                format!("{}.tolerance", c.field),
                                tol.to_string(),
                                c.source("tolerance"),
                            );
                            if apart_pct > tol {
                                j.miss(format!(
                                    "{} where {label} says {}, {apart_pct:.1} % apart, beyond its ±{tol} %",
                                    format_si(got, unit),
                                    format_si(*want, unit),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some((want, s)) = &w.tolerance {
        match c.tolerance_pct(c.number(key)) {
            None => j.lacks("tolerance"),
            Some(got) => {
                j.input(
                    format!("{reference}.{}", s.from),
                    s.text.clone(),
                    s.source.clone(),
                );
                j.input(
                    format!("{}.tolerance", c.field),
                    got.to_string(),
                    c.source("tolerance"),
                );
                if got > *want {
                    j.miss(format!("±{got} %, {} asks ±{want} %", s.from));
                }
            }
        }
    }
    j
}

inventory::submit!(Registered(&Bom12));

#[cfg(test)]
mod tests {
    use super::percent;

    #[test]
    fn percents() {
        assert_eq!(percent("1%"), Some(1.0));
        assert_eq!(percent("±5 %"), Some(5.0));
        assert_eq!(percent("0.1"), Some(0.1));
        assert_eq!(percent("tight"), None);
    }
}
