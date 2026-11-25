//! What the component checks (`BOM-12`, `LIB-6`, `RATE-1`, `RATE-2`, `RATE-9`, `RATE-15`) share: every MPN of a
//! fitted part with its facts from the parts snapshot, what the part's fields state (or the board's defaults
//! for them), the working voltage across a part, and the report of each verdict.
//!
//! Each MPN is judged on its own. A miss is one finding per part and MPN field (`C4 Mpn2`), carrying the hash of
//! every input it rests on, so a waiver goes stale when a field, a fact or a derating factor changes. A fact
//! the snapshot lacks is never a pass: the part gets one finding at `info` that names each MPN and fact not
//! checked.

pub(crate) mod dielectric;

use crate::Ctx;
use crate::finding::Finding;
use crate::meta::{CheckMeta, Param, Severity};
use erc_analysis::value::format_si;
use erc_model::{PartId, natural_cmp};
use erc_model::{Provenance, Subject, Unit, Value, Written};
use erc_parts::lock::{FactValue, LockedPart, PartMatch};

/// Two values closer than this, relative, are the same (a value written `4k7` and a fact of 4700.0000001); in
/// the rounding of a rating, the same noise in steps of the precision written
pub(crate) const SAME_REL: f64 = 1e-6;

/// How far a `Voltage` or `Power` text may round the rating it states (`BOM-12`, `berc fields sync`)
pub(crate) const RATING_ROUNDING_MAX: Param = Param {
    name: "drawing.fields.rating_rounding_max",
    default: 1.0,
    unit: "%",
    doc: "most a rounded Voltage or Power text may differ from the lowest rating of a part's MPNs, in percent of it \
          (BOM-12, berc fields sync)",
    choices: &[],
};

/// One MPN of a part, as the snapshot knows it
pub(crate) struct Candidate<'a> {
    /// The field it is written in (`Mpn2`)
    pub field: &'a str,
    pub mpn: &'a str,
    pub part: &'a LockedPart,
}

/// The MPNs of a fitted part that the snapshot lists, in field order. An MPN it does not list is `BOM-20`'s.
pub(crate) fn candidates<'a>(cx: &Ctx<'a>, p: PartId) -> Vec<Candidate<'a>> {
    let Some(lock) = cx.parts() else {
        return Vec::new();
    };
    erc_parts::mpn::mpns(cx.view(), p)
        .into_iter()
        .filter_map(|(field, mpn)| lock.part(mpn).map(|part| Candidate { field, mpn, part }))
        .collect()
}

impl Candidate<'_> {
    /// A number fact in its base unit; a range gives its maximum (a rated voltage `0..50 V` is 50 V)
    pub fn number(&self, key: &str) -> Option<f64> {
        match &self.part.fact(key)?.value {
            FactValue::Number { value, .. } => Some(*value),
            FactValue::Range { max, .. } => Some(*max),
            FactValue::Tolerance { .. } | FactValue::Text { .. } => None,
        }
    }

    /// A word fact (`X7R`, `0402`), else the MPN's decoded segment of that key
    pub fn text(&self, key: &str) -> Option<&str> {
        match self.part.fact(key).map(|f| &f.value) {
            Some(FactValue::Text { text }) => Some(text.as_str()),
            Some(_) => None,
            None => self
                .part
                .decode
                .as_ref()
                .and_then(|d| d.segments.get(key))
                .map(String::as_str),
        }
    }

    /// The tolerance as the larger of its two sides in percent of `nominal`; an absolute one (`±0.25 pF`) is
    /// converted with it
    pub fn tolerance_pct(&self, nominal: Option<f64>) -> Option<f64> {
        let FactValue::Tolerance { minus, plus, unit } = &self.part.fact("tolerance")?.value else {
            return None;
        };
        let side = minus.abs().max(plus.abs());
        if unit == "%" {
            Some(side)
        } else {
            nominal.filter(|n| *n > 0.0).map(|n| side / n * 100.0)
        }
    }

    /// Where a fact of this MPN comes from, for a finding's facts
    pub fn source(&self, key: &str) -> Provenance {
        let by = self
            .part
            .fact(key)
            .map_or("its MPN's scheme", |f| f.source.as_str());
        Provenance::PartDb {
            file: erc_parts::lock::FILE_NAME.to_owned(),
            entry: format!("{} {key} ({by})", self.mpn),
        }
    }
}

/// A field a part states, or the board's default for it
pub(crate) struct Stated {
    pub text: String,
    /// `Voltage`, or `DEFAULT_C_VOLTAGE` for the default
    pub from: String,
    pub source: Provenance,
}

/// A field of a part in the variant checked, by name in any case; `None` when absent, empty or `~`
pub(crate) fn field(cx: &Ctx, p: PartId, name: &str) -> Option<Stated> {
    let view = cx.view();
    let f = cx
        .board()
        .part(p)
        .fields
        .iter()
        .find(|f| f.name.eq_ignore_ascii_case(name))?;
    let fact = view.field_fact(p, &f.name)?;
    let text = fact.value.trim();
    (!text.is_empty() && text != "~").then(|| Stated {
        text: text.to_owned(),
        from: f.name.clone(),
        source: fact.source.clone(),
    })
}

/// A field of a part, else the project's text variable that is the board's default for it (`DEFAULT_C_VOLTAGE`,
/// the Defaults box on the drawing sheet)
pub(crate) fn stated(cx: &Ctx, p: PartId, name: &str, default_var: Option<&str>) -> Option<Stated> {
    field(cx, p, name).or_else(|| default_var.and_then(|var| default_of(cx, var)))
}

/// The board's default in the text variable `var`; `None` when the project has none or it is empty
pub(crate) fn default_of(cx: &Ctx, var: &str) -> Option<Stated> {
    let fact = cx
        .board()
        .settings
        .text_vars
        .iter()
        .find(|v| v.key == var)?
        .value
        .resolve()?;
    let text = fact.value.trim();
    (!text.is_empty()).then(|| Stated {
        text: text.to_owned(),
        from: var.to_owned(),
        source: fact.source.clone(),
    })
}

/// The board default variable of a field for a part of this designator prefix: `DEFAULT_R_POWER` for a
/// resistor's `Power`
pub(crate) fn default_var(reference: &str, name: &str) -> Option<&'static str> {
    let prefix: String = reference
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect();
    Some(match (prefix.as_str(), name) {
        ("R", "Package") => "DEFAULT_R_PACKAGE",
        ("R", "Power") => "DEFAULT_R_POWER",
        ("C", "Package") => "DEFAULT_C_PACKAGE",
        ("C", "Dielectric") => "DEFAULT_C_DIELECTRIC",
        ("C", "Voltage") => "DEFAULT_C_VOLTAGE",
        _ => return None,
    })
}

/// A field's text read as a quantity in `unit` (`50V`, `63mW`, `1/4W`, `3.3`): the value when it is in that unit,
/// else the first rating in it
pub(crate) fn quantity(text: &str, unit: Unit) -> Option<f64> {
    written(text, unit).map(|w| w.value)
}

/// [`quantity`] with the precision the text writes it with
pub(crate) fn written(text: &str, unit: Unit) -> Option<Written> {
    Value::written(text, unit)
}

/// `rating` in whole steps of `step`, half a step rounded up
fn whole_steps(rating: f64, step: f64) -> f64 {
    (rating / step + 0.5 + SAME_REL).floor()
}

/// Whether a display field states a rating: it equals it, or it is the rating rounded half up to the precision
/// the field is written with and differs from it by at most `rounding_max_pct` percent. At 1 %, `63mW` states
/// 62.5 mW; `62mW` (not the rounding) and `0.1W` (60 % apart) do not. A fraction (`1/16W`) is not rounded.
pub(crate) fn states(field: Written, rating: f64, rounding_max_pct: f64) -> bool {
    let apart = (field.value - rating).abs();
    if apart <= SAME_REL * rating.abs() {
        return true;
    }
    // Both sides are whole numbers of steps
    let rounded = field
        .step
        .is_some_and(|step| (whole_steps(rating, step) - (field.value / step).round()).abs() < 0.5);
    rounded && apart <= (rounding_max_pct / 100.0 + SAME_REL) * rating.abs()
}

/// A rating as a display field is written, a text that [`states`] it. A power from 1 mW to below 1 W is in whole
/// mW (`63mW` for 62.5 mW), any other rating has up to three decimals of its SI prefix (`6.3V`, `1.5W`); where
/// that rounds further than `rounding_max_pct`, the text has more digits (`2.5mW`).
pub(crate) fn display_text(rating: f64, unit: Unit, rounding_max_pct: f64) -> String {
    let stated =
        |text: &String| written(text, unit).is_some_and(|w| states(w, rating, rounding_max_pct));
    let milliwatts = (unit == Unit::Watt && (1e-3..1.0).contains(&rating))
        .then(|| format!("{}mW", whole_steps(rating, 1e-3)));
    // Raised past float noise, so a half rounds up as in `states`
    let prefixed = || format_si(rating * (1.0 + 1e-9), unit).replace(' ', "");
    milliwatts
        .filter(stated)
        .or_else(|| Some(prefixed()).filter(stated))
        .unwrap_or_else(|| format!("{rating}{}", unit.name()))
}

/// The imperial chip case code a text names (`0402`, `R0402`, `0402 (1005 Metric)`), else the text trimmed
pub(crate) fn case_code(text: &str) -> String {
    let t = text.trim();
    t.split(|c: char| !c.is_ascii_alphanumeric())
        .find_map(|w| {
            let digits = w.trim_start_matches(|c: char| c.is_ascii_alphabetic());
            (matches!(digits.len(), 4 | 5) && digits.bytes().all(|b| b.is_ascii_digit()))
                .then(|| digits.to_owned())
        })
        .unwrap_or_else(|| t.to_owned())
}

/// The voltage across a part and how it is known
pub(crate) struct Working {
    pub volts: f64,
    /// `Vworking field`, `+5V to GND`
    pub how: String,
    pub source: Provenance,
    /// The inputs it rests on, for a finding's input hash
    pub inputs: Vec<(String, String)>,
}

/// The working voltage of a two-terminal part: its `Vworking` field, else the larger difference between the
/// voltages of its two nets (a rail's range when the project states one, 0 V for a ground). `None` when neither
/// is known.
pub(crate) fn working_voltage(cx: &Ctx, p: PartId) -> Option<Working> {
    let reference = &cx.board().part(p).designator;
    if let Some(f) = field(cx, p, "Vworking") {
        let volts = quantity(&f.text, Unit::Volt)?;
        return Some(Working {
            volts: volts.abs(),
            how: format!("{} field", f.from),
            source: f.source,
            inputs: vec![(format!("{reference}.{}", f.from), f.text)],
        });
    }
    let board = cx.board();
    let mut nets: Vec<erc_model::NetId> = board
        .part(p)
        .pins
        .iter()
        .filter_map(|pin| board.pin(*pin).net)
        .collect();
    nets.sort();
    nets.dedup();
    let [a, b] = nets[..] else {
        return None;
    };
    let power = cx.power();
    let range = |n: erc_model::NetId| -> Option<((f64, f64), Provenance)> {
        if power.is_ground(n) {
            return Some((
                (0.0, 0.0),
                Provenance::Derived {
                    rule: "a ground is 0 V".to_owned(),
                },
            ));
        }
        let rail = power.rails.get(&n)?;
        if let Some(r) = &rail.range {
            return Some((r.value, r.source.clone()));
        }
        let v = rail.voltage.resolve()?;
        Some(((v.value, v.value), v.source.clone()))
    };
    let ((amin, amax), asrc) = range(a)?;
    let ((bmin, bmax), bsrc) = range(b)?;
    let volts = (amax - bmin).abs().max((bmax - amin).abs());
    let (na, nb) = (&board.net(a).name, &board.net(b).name);
    // The rail's source decides how sure the voltage is: a voltage read from a net name is an assumption
    let source = if asrc.is_estimate() || (power.is_ground(b) && !bsrc.is_estimate()) {
        asrc
    } else {
        bsrc
    };
    let show = |lo: f64, hi: f64| {
        if lo == hi {
            format_si(lo, Unit::Volt)
        } else {
            format!(
                "{}..{}",
                format_si(lo, Unit::Volt),
                format_si(hi, Unit::Volt)
            )
        }
    };
    Some(Working {
        volts,
        how: format!("{na} ({}) to {nb} ({})", show(amin, amax), show(bmin, bmax)),
        source,
        inputs: vec![
            (format!("net {na}"), format!("{amin}..{amax}")),
            (format!("net {nb}"), format!("{bmin}..{bmax}")),
        ],
    })
}

/// The verdict on one MPN of a part: what misses, what could not be checked, and what it rests on
pub(crate) struct Judged<'c> {
    pub field: &'c str,
    pub mpn: &'c str,
    /// Whether the part source found the MPN's part
    matched: PartMatch,
    /// One line per requirement missed, with its margin
    pub misses: Vec<String>,
    /// Facts the snapshot lacks for a requirement that applies
    pub missing: Vec<&'static str>,
    inputs: Vec<(String, String)>,
    facts: Vec<(String, String, Provenance)>,
}

impl<'c> Judged<'c> {
    pub fn new(c: &Candidate<'c>) -> Judged<'c> {
        Judged {
            field: c.field,
            mpn: c.mpn,
            matched: c.part.matched,
            misses: Vec::new(),
            missing: Vec::new(),
            inputs: vec![("mpn".to_owned(), c.mpn.to_owned())],
            facts: Vec::new(),
        }
    }

    /// A value the verdict rests on: in the finding's facts and its input hash
    pub fn input(&mut self, name: impl Into<String>, value: impl Into<String>, source: Provenance) {
        let (name, value) = (name.into(), value.into());
        if !self.inputs.iter().any(|(n, _)| *n == name) {
            self.inputs.push((name.clone(), value.clone()));
            self.facts.push((name, value, source));
        }
    }

    /// Inputs without a fact line (net voltages already shown, a derating factor's value)
    pub fn inputs(&mut self, inputs: &[(String, String)]) {
        for (n, v) in inputs {
            if !self.inputs.iter().any(|(m, _)| m == n) {
                self.inputs.push((n.clone(), v.clone()));
            }
        }
    }

    pub fn miss(&mut self, line: impl Into<String>) {
        self.misses.push(line.into());
    }

    pub fn lacks(&mut self, key: &'static str) {
        if !self.missing.contains(&key) {
            self.missing.push(key);
        }
    }
}

/// A requirement against a rating: `rated ≥ k × working`. The miss line names both, the ratio and the factor
/// (`6.3 V rated, 3.3 V working, 1.9× < 2.0×`); `None` when it holds.
pub(crate) fn margin(rated: f64, working: f64, k: f64, unit: Unit, what: &str) -> Option<String> {
    if rated >= k * working {
        return None;
    }
    let ratio = if working > 0.0 { rated / working } else { 0.0 };
    Some(format!(
        "{} rated, {} {what}, {ratio:.2}× < {k}×",
        format_si(rated, unit),
        format_si(working, unit),
    ))
}

/// The findings of one part: one per MPN field with a miss, and one at `info` naming the facts not checked
pub(crate) fn report(
    cx: &Ctx,
    meta: &'static CheckMeta,
    p: PartId,
    judged: Vec<Judged>,
    fix: &str,
) -> Vec<Finding> {
    let board = cx.board();
    let reference = &board.part(p).designator;
    let mut out = Vec::new();
    let mut unchecked = Vec::new();
    for j in judged {
        if !j.missing.is_empty() {
            unchecked.push(match j.matched {
                PartMatch::Found => {
                    format!("{} {} has no {}", j.field, j.mpn, j.missing.join(", no "))
                }
                PartMatch::NotFound => format!("{} {} is not in the part source", j.field, j.mpn),
                PartMatch::NotUnique => format!(
                    "{} {} names several parts in the part source (give its maker)",
                    j.field, j.mpn
                ),
            });
        }
        if j.misses.is_empty() {
            continue;
        }
        let mut f = cx
            .finding(meta)
            .part(p)
            .item(
                Subject::Part(cx.board_id, p),
                format!("{reference} {}", j.field),
            )
            .message(format!(
                "{reference} {} {}: {}",
                j.field,
                j.mpn,
                j.misses.join("; ")
            ))
            .fix(fix);
        for (n, v) in &j.inputs {
            f = f.input(n, v);
        }
        for (n, v, s) in j.facts {
            f = f.fact(n, v, s);
        }
        out.push(f.build());
    }
    if !unchecked.is_empty() {
        out.push(
            cx.finding(meta)
                .part(p)
                .severity(Severity::Info)
                .message(format!(
                    "{reference}: not checked: {}",
                    unchecked.join("; ")
                ))
                .fix("add the facts to the part source and run `berc parts refresh`")
                .build(),
        );
    }
    out
}

/// The ratings of a number fact over a part's MPNs, for a display field (`Voltage`, `Power`) that states the lowest
pub(crate) struct Ratings<'a, 'c> {
    /// Each MPN that has the fact, with it
    pub known: Vec<(f64, &'a Candidate<'c>)>,
    /// The index in the candidates of each MPN that lacks it
    pub lacking: Vec<usize>,
}

impl<'a, 'c> Ratings<'a, 'c> {
    pub fn of(cands: &'a [Candidate<'c>], fact: &str) -> Ratings<'a, 'c> {
        let mut r = Ratings {
            known: Vec::new(),
            lacking: Vec::new(),
        };
        for (i, c) in cands.iter().enumerate() {
            match c.number(fact) {
                Some(v) => r.known.push((v, c)),
                None => r.lacking.push(i),
            }
        }
        r
    }

    /// The lowest rating and its MPN, when every MPN has the fact
    pub fn lowest(&self) -> Option<(f64, &'a Candidate<'c>)> {
        if !self.lacking.is_empty() {
            return None;
        }
        self.known
            .iter()
            .copied()
            .min_by(|a, b| a.0.total_cmp(&b.0))
    }
}

/// The dielectric codes of a part's ceramic MPNs, for a `Dielectric` field that states the worst
pub(crate) struct Dielectrics<'a, 'c> {
    /// Each ceramic MPN (or one whose technology is not known) that has a code, with it
    pub codes: Vec<(&'a str, &'a Candidate<'c>)>,
    /// The index in the candidates of each such MPN that lacks a code
    pub lacking: Vec<usize>,
}

impl<'a, 'c> Dielectrics<'a, 'c> {
    pub fn of(cands: &'a [Candidate<'c>]) -> Dielectrics<'a, 'c> {
        let mut d = Dielectrics {
            codes: Vec::new(),
            lacking: Vec::new(),
        };
        for (i, c) in cands.iter().enumerate() {
            if !ceramic_or_unknown(c) {
                continue;
            }
            match c.text("dielectric") {
                Some(code) => d.codes.push((code, c)),
                None => d.lacking.push(i),
            }
        }
        d
    }

    /// Whether the worst code can be judged: some MPN has one, none lacks it, and every code is one this reads
    pub fn judgeable(&self) -> bool {
        !self.codes.is_empty()
            && self.lacking.is_empty()
            && self
                .codes
                .iter()
                .all(|(c, _)| dielectric::covers(c, c).is_some())
    }

    /// The code every other one covers; `None` when no code is worse than all the others
    pub fn worst(&self) -> Option<&'a str> {
        self.codes.iter().map(|(w, _)| *w).find(|w| {
            self.codes
                .iter()
                .all(|(d, _)| dielectric::covers(w, d) == Some(true))
        })
    }
}

/// Whether two dielectric codes mean the same: each covers the other
pub(crate) fn same_dielectric(a: &str, b: &str) -> bool {
    dielectric::covers(a, b) == Some(true) && dielectric::covers(b, a) == Some(true)
}

/// Whether the MPN is a ceramic capacitor, or its technology is not known: a dielectric code applies
pub(crate) fn ceramic_or_unknown(c: &Candidate) -> bool {
    c.text("technology").is_none_or(|t| {
        let t = t.to_ascii_lowercase();
        t.contains("ceramic") || t.contains("mlcc")
    })
}

/// The fitted parts of the variant checked, by designator
pub(crate) fn fitted_sorted(cx: &Ctx) -> Vec<PartId> {
    let b = cx.board();
    let mut parts: Vec<PartId> = cx.view().fitted_components().collect();
    parts.sort_by(|x, y| natural_cmp(&b.part(*x).designator, &b.part(*y).designator));
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_codes() {
        assert_eq!(case_code("0402"), "0402");
        assert_eq!(case_code("R0805"), "0805");
        assert_eq!(case_code("0402 (1005 Metric)"), "0402");
        assert_eq!(case_code(" SOD-323 "), "SOD-323");
        assert_eq!(case_code("01005"), "01005");
    }

    #[test]
    fn quantities() {
        assert_eq!(quantity("50V", Unit::Volt), Some(50.0));
        assert_eq!(quantity("3.3", Unit::Volt), Some(3.3));
        let mw = quantity("63mW", Unit::Watt).unwrap();
        assert!((mw - 0.063).abs() < 1e-12, "{mw}");
        assert_eq!(quantity("X7R", Unit::Volt), None);
    }

    fn field_states_within(text: &str, unit: Unit, rating: f64, pct: f64) -> bool {
        states(written(text, unit).unwrap(), rating, pct)
    }

    /// With the default limit of 1 %
    fn field_states(text: &str, unit: Unit, rating: f64) -> bool {
        field_states_within(text, unit, rating, RATING_ROUNDING_MAX.default)
    }

    #[test]
    fn a_field_states_a_rating_rounded_to_its_precision() {
        let w = Unit::Watt;
        assert!(field_states("63mW", w, 0.0625));
        assert!(field_states("63 mW", w, 0.063));
        assert!(!field_states("62mW", w, 0.0625));
        assert!(!field_states("64mW", w, 0.0625));
        assert!(field_states("62.5mW", w, 0.0625));
        assert!(!field_states("62.4mW", w, 0.0625));
        assert!(field_states("0.1W", w, 0.1));
        assert!(field_states("100mW", w, 0.1));
        assert!(field_states("1/16W", w, 0.0625));
        assert!(!field_states("1/16W", w, 0.063));
        let v = Unit::Volt;
        assert!(field_states("50V", v, 50.0));
        assert!(field_states("50V", v, 49.6));
        assert!(!field_states("50V", v, 49.4));
        assert!(!field_states("25V", v, 50.0));
        assert!(field_states("6V3", v, 6.3));
        assert!(!field_states("6.3V", v, 6.4));
        assert!(field_states("2.5kV", v, 2500.0));
        assert!(field_states("3.3", v, 3.3));
    }

    /// A rounding past the limit overstates or understates the part, however coarse the text is
    #[test]
    fn a_rounding_past_the_limit_states_nothing() {
        let w = Unit::Watt;
        assert!(!field_states("0.1W", w, 0.0625));
        assert!(!field_states("1W", w, 0.5));
        assert!(!field_states("6V", Unit::Volt, 6.3));
        assert!(!field_states("50V", Unit::Volt, 49.4));
        // The limit is the setting's: 0.8 % apart
        assert!(!field_states_within("63mW", w, 0.0625, 0.0));
        assert!(!field_states_within("63mW", w, 0.0625, 0.5));
        assert!(field_states_within("63mW", w, 0.0625, 0.8));
        assert!(field_states_within("0.1W", w, 0.0625, 60.0));
        // Equal is never too far, and no limit makes another number a rounding
        assert!(field_states_within("62.5mW", w, 0.0625, 0.0));
        assert!(!field_states_within("62mW", w, 0.0625, 60.0));
    }

    #[test]
    fn a_written_rating_states_it() {
        let text = |r, u| display_text(r, u, RATING_ROUNDING_MAX.default);
        assert_eq!(text(0.0625, Unit::Watt), "63mW");
        assert_eq!(text(0.125, Unit::Watt), "125mW");
        assert_eq!(text(0.03125, Unit::Watt), "31mW");
        assert_eq!(text(0.1, Unit::Watt), "100mW");
        assert_eq!(text(0.5, Unit::Watt), "500mW");
        // Whole mW would be 20 % apart
        assert_eq!(text(0.0025, Unit::Watt), "2.5mW");
        assert_eq!(text(1.0, Unit::Watt), "1W");
        assert_eq!(text(1.5, Unit::Watt), "1.5W");
        assert_eq!(text(50.0, Unit::Volt), "50V");
        assert_eq!(text(6.3, Unit::Volt), "6.3V");
        assert_eq!(display_text(0.0625, Unit::Watt, 0.0), "62.5mW");
        for pct in [0.0, 1.0, 50.0] {
            for unit in [Unit::Watt, Unit::Volt] {
                for rating in [
                    0.0005, 0.001, 0.0015, 0.03125, 0.0625, 0.9995, 0.9999999, 1.0025, 1.00005,
                    6.3, 999.9999, 1234.5675, 2500.0,
                ] {
                    let t = display_text(rating, unit, pct);
                    assert!(
                        field_states_within(&t, unit, rating, pct),
                        "{t} for {rating} within {pct} %"
                    );
                }
            }
        }
    }

    #[test]
    fn margins() {
        assert_eq!(margin(6.3, 3.3, 1.5, Unit::Volt, "working"), None);
        assert_eq!(
            margin(6.3, 3.3, 2.0, Unit::Volt, "working").as_deref(),
            Some("6.3 V rated, 3.3 V working, 1.91× < 2×")
        );
        assert_eq!(margin(0.1, 0.05, 2.0, Unit::Watt, "dissipated"), None);
    }
}
