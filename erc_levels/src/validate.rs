//! The checks [`Database::validate`] runs over the data.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::{Corner, Database, Dimension, Family, FamilyKind, Level, Limit, Unit};

/// One thing wrong with the data.
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    /// The family (or `docs.ID`) it is about.
    pub at: String,
    pub what: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.at, self.what)
    }
}

const CORNERS: [Corner; 3] = [Corner::Min, Corner::Typ, Corner::Max];

/// Lower and upper threshold pairs that must not cross: the low one's max below the high one's min.
const ORDERED: [(&str, &str); 2] = [("VIL", "VIH"), ("VOL", "VOH")];

pub(crate) fn validate(db: &Database) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut used = BTreeSet::new();
    for (id, f) in &db.family {
        let mut p = |what: String| {
            out.push(Problem {
                at: id.clone(),
                what,
            })
        };
        family(db, f, &mut used, &mut p);
    }
    for (id, d) in &db.docs {
        let at = format!("docs.{id}");
        if !used.contains(id.as_str()) {
            out.push(Problem {
                at: at.clone(),
                what: "not used by any family".into(),
            });
        }
        if d.url.trim().is_empty() || d.title.trim().is_empty() {
            out.push(Problem {
                at: at.clone(),
                what: "needs a title and a url".into(),
            });
        }
        if let Some(l) = &d.library
            && (l.starts_with('/') || l.starts_with('~'))
        {
            out.push(Problem {
                at,
                what: format!("library path {l:?} must be relative to ~/sync"),
            });
        }
    }
    // The supply corners repeat the same finding; keep each once.
    let mut seen = BTreeSet::new();
    out.retain(|p| seen.insert(p.to_string()));
    out
}

fn family<'a>(
    db: &'a Database,
    f: &'a Family,
    used: &mut BTreeSet<&'a str>,
    p: &mut impl FnMut(String),
) {
    let doc_ok = |id: &str, used: &mut BTreeSet<&'a str>, p: &mut dyn FnMut(String)| match db
        .docs
        .get_key_value(id)
    {
        Some((k, _)) => {
            used.insert(k.as_str());
        }
        None => p(format!("unknown document {id:?}")),
    };
    doc_ok(&f.source.doc, used, p);
    if f.name.trim().is_empty() || f.group.trim().is_empty() {
        p("needs a name and a group".into());
    }
    if f.kind == FamilyKind::NoLevels {
        if f.notes.as_deref().is_none_or(|n| n.trim().is_empty()) {
            p("no-levels family needs notes saying what it uses".into());
        }
    } else if f.level.is_empty() {
        p("no levels".into());
    }
    for s in &f.see {
        if !db.family.contains_key(s) {
            p(format!("see: unknown family {s:?}"));
        }
    }
    if let Some([a, b]) = f.temp_c
        && a >= b
    {
        p(format!("temp_c {a}..{b} is not a range"));
    }

    // Supplies: unique names, expressions of earlier supplies only, min <= typ <= max.
    let mut names: Vec<&str> = Vec::new();
    for s in &f.supply {
        if names.contains(&s.name.as_str()) {
            p(format!("supply {} twice", s.name));
        }
        for l in [&s.min, &s.typ, &s.max].into_iter().flatten() {
            if let Some(v) = l.var()
                && !names.contains(&v)
            {
                p(format!(
                    "supply {} uses {v}, which is not an earlier supply",
                    s.name
                ));
            }
        }
        names.push(&s.name);
        if let Some(d) = &s.doc {
            doc_ok(d, used, p);
        }
        if s.min.is_none() && s.typ.is_none() && s.max.is_none() {
            p(format!("supply {} has no value", s.name));
        }
        if s.verified {
            verified(
                db,
                f,
                s.doc.as_deref(),
                s.loc.as_deref(),
                &format!("supply {}", s.name),
                p,
            );
        }
    }
    let envs: Vec<_> = CORNERS.iter().map(|c| f.env(*c)).collect();
    for (c, env) in CORNERS.iter().zip(&envs) {
        match env {
            Err(e) => p(format!("supplies at {c:?}: {e}")),
            Ok(env) => {
                for s in &f.supply {
                    ordered(
                        &format!("supply {}", s.name),
                        &s.min,
                        &s.typ,
                        &s.max,
                        1.0,
                        env,
                        p,
                    );
                }
            }
        }
    }
    if let Some(t) = &f.termination {
        if t.r_ohm.is_some_and(|r| r <= 0.0) {
            p("termination r_ohm must be positive".into());
        }
        if let Some(v) = t.to.as_ref().and_then(|l| l.var())
            && !names.contains(&v)
        {
            p(format!("termination to {v}, which is not a supply"));
        }
        if let Some(d) = &t.doc {
            doc_ok(d, used, p);
        }
        if t.verified {
            verified(db, f, t.doc.as_deref(), t.loc.as_deref(), "termination", p);
        }
    }

    // Levels.
    let mut units: BTreeMap<&str, Unit> = BTreeMap::new();
    for l in &f.level {
        let what = label(l);
        if let Some(d) = &l.doc {
            doc_ok(d, used, p);
        }
        if l.min.is_none() && l.typ.is_none() && l.max.is_none() {
            p(format!("{what}: no min, typ or max"));
        }
        match Dimension::of_param(&l.param) {
            Some(d) if d != l.unit.dimension() => p(format!(
                "{what}: unit {:?} does not fit the parameter",
                l.unit
            )),
            Some(_) => {}
            None if l.desc.is_none() => p(format!("{what}: unknown parameter needs a desc")),
            None => {}
        }
        if let Some(u) = units.insert(&l.param, l.unit)
            && u != l.unit
        {
            p(format!(
                "{what}: unit {:?}, other rows of {} use {u:?}",
                l.unit, l.param
            ));
        }
        for lim in [&l.min, &l.typ, &l.max].into_iter().flatten() {
            if let Some(v) = lim.var() {
                if !names.contains(&v) {
                    p(format!("{what}: uses {v}, which is not a supply"));
                }
                if l.unit != Unit::V {
                    p(format!("{what}: an expression of a supply needs unit V"));
                }
            }
        }
        if let Some([a, b]) = l.temp_c
            && a >= b
        {
            p(format!("{what}: temp_c {a}..{b} is not a range"));
        }
        if l.verified {
            verified(db, f, l.doc.as_deref(), l.loc.as_deref(), &what, p);
        }
        for env in envs.iter().flatten() {
            let env = with_vsupply(f, l, env);
            ordered(&what, &l.min, &l.typ, &l.max, l.unit.to_si(), &env, p);
        }
    }

    // Thresholds that must not cross, within rows of the same supply, state and temperature.
    for (lo, hi) in ORDERED {
        for a in f.levels(lo) {
            for b in f.levels(hi).filter(|b| same_case(a, b)) {
                for env in envs.iter().flatten() {
                    let env = with_vsupply(f, a, env);
                    let (Some(x), Some(y)) = (si(a, &a.max, &env), si(b, &b.min, &env)) else {
                        continue;
                    };
                    if x >= y {
                        p(format!(
                            "{} max {x} V is not below {} min {y} V",
                            label(a),
                            label(b)
                        ));
                        break;
                    }
                }
            }
        }
    }
    for a in f.levels("VT-") {
        for b in f.levels("VT+").filter(|b| same_case(a, b)) {
            for env in envs.iter().flatten() {
                let env = with_vsupply(f, a, env);
                let x = si(a, &a.typ, &env).or(si(a, &a.min, &env));
                let y = si(b, &b.typ, &env).or(si(b, &b.max, &env));
                if let (Some(x), Some(y)) = (x, y)
                    && x >= y
                {
                    p(format!(
                        "{} {x} V is not below {} {y} V",
                        label(a),
                        label(b)
                    ));
                    break;
                }
            }
        }
    }
}

/// A verified value needs a fetched document and a location in it.
fn verified(
    db: &Database,
    f: &Family,
    doc: Option<&str>,
    loc: Option<&str>,
    what: &str,
    p: &mut dyn FnMut(String),
) {
    let id = doc.unwrap_or(&f.source.doc);
    if db.docs.get(id).is_some_and(|d| d.library.is_none()) {
        p(format!(
            "{what}: verified, but document {id} is not in the library"
        ));
    }
    let loc = if doc.is_some() {
        loc
    } else {
        loc.or(f.source.loc.as_deref())
    };
    if loc.is_none_or(|l| l.trim().is_empty()) {
        p(format!("{what}: verified needs a location (loc) in {id}"));
    }
}

fn label(l: &Level) -> String {
    let mut s = l.param.clone();
    if let Some(v) = l.vsupply {
        s += &format!(" @{v} V");
    }
    if let Some(st) = &l.state {
        s += &format!(" ({st})");
    }
    if let Some(t) = l.t_c {
        s += &format!(" at {t} °C");
    }
    if let Some(i) = l.iout_ma {
        s += &format!(" at {i} mA");
    }
    s
}

fn same_case(a: &Level, b: &Level) -> bool {
    a.vsupply == b.vsupply && a.state == b.state && a.temp_c == b.temp_c && a.t_c == b.t_c
}

/// A row specified at one supply voltage evaluates the main supply's expressions there.
fn with_vsupply(f: &Family, l: &Level, env: &BTreeMap<String, f64>) -> BTreeMap<String, f64> {
    let mut env = env.clone();
    if let (Some(v), Some(s)) = (l.vsupply, f.supply.first()) {
        env.insert(s.name.clone(), v);
    }
    env
}

fn si(l: &Level, lim: &Option<Limit>, env: &BTreeMap<String, f64>) -> Option<f64> {
    let lim = lim.as_ref()?;
    let v = lim.eval(env).ok()?;
    Some(if lim.is_expr() { v } else { v * l.unit.to_si() })
}

fn ordered(
    what: &str,
    min: &Option<Limit>,
    typ: &Option<Limit>,
    max: &Option<Limit>,
    factor: f64,
    env: &BTreeMap<String, f64>,
    p: &mut dyn FnMut(String),
) {
    let v = |l: &Option<Limit>| {
        l.as_ref().and_then(|l| {
            l.eval(env)
                .ok()
                .map(|v| if l.is_expr() { v } else { v * factor })
        })
    };
    let (a, t, b) = (v(min), v(typ), v(max));
    let bad =
        |x: Option<f64>, y: Option<f64>| matches!((x, y), (Some(x), Some(y)) if x > y + 1e-12);
    if bad(a, t) || bad(t, b) || bad(a, b) {
        p(format!(
            "{what}: min {a:?} <= typ {t:?} <= max {b:?} does not hold"
        ));
    }
}
