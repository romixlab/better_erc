//! The display fields of resistors and capacitors as the parts snapshot states them (`TOOL-14`): `Voltage` and
//! `Power` the lowest rating over a part's MPNs, `Dielectric` the worst code, `Package` the case they share, the
//! plan `berc fields sync` writes. A field equal to the board's `DEFAULT_*` text variable is taken off the part;
//! a part without the field and off the default gets one. An MPN without the fact leaves the field as it is.

use crate::Ctx;
use crate::component::{self, Dielectrics, Ratings};
use erc_analysis::PartClass;
use erc_analysis::value::format_si;
use erc_model::{PartId, Unit};

/// Two ratings closer than this, relative, are the same: the tolerance BOM-12 holds a field to
const SAME_REL: f64 = 1e-6;

/// What happens to one field of a part
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldOp {
    /// The field exists with another text: it gets this one
    Set(String),
    /// The part has no such field: it gets one with this text
    Add(String),
    /// The field is at the board default: it goes
    Remove,
}

/// One field of one placed symbol to write
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSync {
    /// The sheet file, relative to the project directory
    pub file: String,
    /// The UUID of the placed symbol in its file
    pub uuid: String,
    pub reference: String,
    /// `Voltage`, `Power`, `Dielectric` or `Package`
    pub field: &'static str,
    /// The text now, when the part has the field
    pub old: Option<String>,
    pub op: FieldOp,
}

/// A field left alone because the facts do not say what it should be
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skipped {
    pub reference: String,
    pub field: &'static str,
    /// `Mpn2 GRM155R60J105KE19D has no rated_voltage`
    pub why: String,
}

/// The fields to write on one board
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyncPlan {
    pub fields: Vec<FieldSync>,
    pub skipped: Vec<Skipped>,
}

/// The plan for the fitted resistors and capacitors of the board of `cx`, which needs its parts snapshot
pub fn plan_sync(cx: &Ctx) -> SyncPlan {
    let mut plan = SyncPlan::default();
    for p in component::fitted_sorted(cx) {
        let class = cx.classes().get(p);
        let names: &[&'static str] = match class {
            PartClass::Resistor => &["Power", "Package"],
            PartClass::Capacitor { .. } => &["Voltage", "Dielectric", "Package"],
            _ => continue,
        };
        let reference = cx.board().part(p).designator.clone();
        let Some(symbol) = symbol_of(cx, &reference) else {
            continue;
        };
        for name in names {
            match worst(cx, p, name) {
                Worst::None => {}
                Worst::Unknown(why) => plan.skipped.push(Skipped {
                    reference: reference.clone(),
                    field: name,
                    why,
                }),
                Worst::Is(text) => {
                    if let Some((old, op)) = decide(cx, p, &reference, name, &text) {
                        let sync = FieldSync {
                            file: symbol.0.clone(),
                            uuid: symbol.1.clone(),
                            reference: reference.clone(),
                            field: name,
                            old,
                            op,
                        };
                        // Instances of one sheet file share a symbol: its field is written once
                        if !plan
                            .fields
                            .iter()
                            .any(|f| f.file == sync.file && f.uuid == sync.uuid && f.field == *name)
                        {
                            plan.fields.push(sync);
                        }
                    }
                }
            }
        }
    }
    plan
}

/// The sheet file and UUID of the symbol a reference is drawn as
fn symbol_of(cx: &Ctx, reference: &str) -> Option<(String, String)> {
    cx.scenes().iter().find_map(|s| {
        s.geometry
            .symbols
            .iter()
            .find(|y| !y.power && y.reference == reference)
            .map(|y| (s.geometry.file.clone(), y.uuid.clone()))
    })
}

/// What the MPNs say a field is
enum Worst {
    /// The part has no MPN in the snapshot, or the field does not apply to it
    None,
    /// The text to write
    Is(String),
    /// The MPNs do not settle it: why
    Unknown(String),
}

fn worst(cx: &Ctx, p: PartId, name: &str) -> Worst {
    let cands = component::candidates(cx, p);
    if cands.is_empty() {
        return Worst::None;
    }
    let lacking = |idx: &[usize], key: &str| {
        let list: Vec<String> = idx
            .iter()
            .map(|i| format!("{} {}", cands[*i].field, cands[*i].mpn))
            .collect();
        Worst::Unknown(format!("{} has no {key}", list.join(", ")))
    };
    match name {
        "Voltage" | "Power" => {
            let (fact, unit) = if name == "Power" {
                ("power", Unit::Watt)
            } else {
                ("rated_voltage", Unit::Volt)
            };
            let ratings = Ratings::of(&cands, fact);
            match ratings.lowest() {
                Some((low, _)) => Worst::Is(format_si(low, unit).replace(' ', "")),
                None => lacking(&ratings.lacking, fact),
            }
        }
        "Dielectric" => {
            let found = Dielectrics::of(&cands);
            if found.codes.is_empty() && found.lacking.is_empty() {
                return Worst::None;
            }
            if !found.lacking.is_empty() {
                return lacking(&found.lacking, "dielectric");
            }
            if !found.judgeable() {
                return Worst::Unknown("a dielectric code it does not read".to_owned());
            }
            match found.worst() {
                Some(w) => Worst::Is(w.to_owned()),
                None => {
                    Worst::Unknown("no dielectric of its MPNs is worse than the others".to_owned())
                }
            }
        }
        _ => {
            let missing: Vec<usize> = (0..cands.len())
                .filter(|i| cands[*i].text("case_eia").is_none())
                .collect();
            if !missing.is_empty() {
                return lacking(&missing, "case_eia");
            }
            let cases: Vec<(&str, String)> = cands
                .iter()
                .map(|c| {
                    let t = c.text("case_eia").unwrap_or_default();
                    (c.field, component::case_code(t))
                })
                .collect();
            let first = &cases[0].1;
            if cases.iter().all(|(_, c)| c.eq_ignore_ascii_case(first)) {
                Worst::Is(first.clone())
            } else {
                let list: Vec<String> = cases.iter().map(|(f, c)| format!("{f} {c}")).collect();
                Worst::Unknown(format!("the MPNs differ in case: {}", list.join(", ")))
            }
        }
    }
}

/// Whether two texts of a field name the same value
fn same(name: &str, a: &str, b: &str) -> bool {
    match name {
        "Voltage" | "Power" => {
            let unit = if name == "Power" {
                Unit::Watt
            } else {
                Unit::Volt
            };
            match (component::quantity(a, unit), component::quantity(b, unit)) {
                (Some(x), Some(y)) => (x - y).abs() <= SAME_REL * y.abs(),
                _ => a.trim() == b.trim(),
            }
        }
        "Dielectric" => component::same_dielectric(a, b) || a.trim().eq_ignore_ascii_case(b.trim()),
        _ => component::case_code(a).eq_ignore_ascii_case(&component::case_code(b)),
    }
}

/// The field's text now and what to do with it, `None` when it is right
fn decide(
    cx: &Ctx,
    p: PartId,
    reference: &str,
    name: &str,
    worst: &str,
) -> Option<(Option<String>, FieldOp)> {
    let now = component::field(cx, p, name).map(|s| s.text);
    let default = component::default_var(reference, name)
        .and_then(|v| component::default_of(cx, v))
        .map(|d| d.text);
    if default.as_deref().is_some_and(|d| same(name, d, worst)) {
        return now.map(|n| (Some(n), FieldOp::Remove));
    }
    match now {
        Some(n) if same(name, &n, worst) => None,
        Some(n) => Some((Some(n), FieldOp::Set(worst.to_owned()))),
        None => Some((None, FieldOp::Add(worst.to_owned()))),
    }
}
