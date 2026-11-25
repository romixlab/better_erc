//! The display fields of resistors and capacitors as the parts snapshot states them (`TOOL-14`): `Voltage` and
//! `Power` the lowest rating over a part's MPNs, `Dielectric` the worst code, `Package` the case they share, the
//! plan `berc fields sync` writes. A field that states what the board's `DEFAULT_*` text variable states is taken
//! off the part; a part without the field and off the default gets one. An MPN without the fact leaves the field as
//! it is. A rating is stated as `BOM-12` reads it: as written, or rounded at most
//! `drawing.fields.rating_rounding_max` apart.

use crate::Ctx;
use crate::bom::rounding_max_pct;
use crate::component::{self, Dielectrics, Ratings};
use erc_analysis::PartClass;
use erc_model::{PartId, Unit};

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
                found @ (Worst::Rating(..) | Worst::Code(_)) => {
                    if let Some((old, op)) = decide(cx, p, &reference, name, &found) {
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
    /// The lowest rating, in its unit
    Rating(f64, Unit),
    /// The code to write: a dielectric or a case
    Code(String),
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
                Some((low, _)) => Worst::Rating(low, unit),
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
                Some(w) => Worst::Code(w.to_owned()),
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
                Worst::Code(first.clone())
            } else {
                let list: Vec<String> = cases.iter().map(|(f, c)| format!("{f} {c}")).collect();
                Worst::Unknown(format!("the MPNs differ in case: {}", list.join(", ")))
            }
        }
    }
}

/// Whether the text of a field states what the MPNs say it is, a rating rounded at most `rounding_max_pct` apart
fn says(name: &str, text: &str, worst: &Worst, rounding_max_pct: f64) -> bool {
    match worst {
        Worst::Rating(low, unit) => component::written(text, *unit)
            .is_some_and(|w| component::states(w, *low, rounding_max_pct)),
        Worst::Code(code) if name == "Dielectric" => {
            component::same_dielectric(text, code) || text.trim().eq_ignore_ascii_case(code.trim())
        }
        Worst::Code(code) => {
            component::case_code(text).eq_ignore_ascii_case(&component::case_code(code))
        }
        Worst::None | Worst::Unknown(_) => false,
    }
}

/// The field's text now and what to do with it, `None` when it is right or the MPNs do not say what it is
fn decide(
    cx: &Ctx,
    p: PartId,
    reference: &str,
    name: &str,
    worst: &Worst,
) -> Option<(Option<String>, FieldOp)> {
    let max_pct = rounding_max_pct(cx);
    let write = match worst {
        Worst::Rating(low, unit) => component::display_text(*low, *unit, max_pct),
        Worst::Code(code) => code.clone(),
        Worst::None | Worst::Unknown(_) => return None,
    };
    let now = component::field(cx, p, name).map(|s| s.text);
    let default = component::default_var(reference, name)
        .and_then(|v| component::default_of(cx, v))
        .map(|d| d.text);
    if default.is_some_and(|d| says(name, &d, worst, max_pct)) {
        return now.map(|n| (Some(n), FieldOp::Remove));
    }
    match now {
        Some(n) if says(name, &n, worst, max_pct) => None,
        Some(n) => Some((Some(n), FieldOp::Set(write))),
        None => Some((None, FieldOp::Add(write))),
    }
}
