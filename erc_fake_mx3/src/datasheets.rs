//! The fake's datasheet facts (TOOL-18): the parameter sources (`/params/sources`), the facts of the definition
//! with key `datasheet` on the catalogue's parts, and `/params/set` for that definition, as the backend keeps
//! them: one current fact per part and source, the same value again only seen again.

use crate::http::Answer;
use crate::{FACTS_AT, RESISTOR_ID, error, json, parse};
use mx3_api_lab::endpoints::parameter::ParameterDefId;
use mx3_api_lab::endpoints::params::{
    Cond, DefRef, Fact, FactId, ParamOwner, ParamSource, Resolved, SetOutcome, SetParam, SourceId,
    SourceKind,
};
use mx3_api_lab::endpoints::part::PartId;
use mx3_api_lab::rvariant::Variant;

/// The id of the datasheet URL definition (mx3's `Datasheet Url`)
const DATASHEET_DEF_ID: i32 = 11;
/// The resistor's datasheet URL, stated by Digi-Key
pub const RESISTOR_DATASHEET: &str = "https://www.yageo.com/upload/media/product/RC_Series.pdf";
/// The token's client name, a written fact's `by`
const BY: &str = "test client";

/// The parameter sources and datasheet facts the fake holds
#[derive(Debug)]
pub struct Datasheets {
    pub sources: Vec<ParamSource>,
    /// Every fact so far, superseded ones included, in the order written
    pub facts: Vec<Fact>,
}

fn source(id: i32, key: &str, rank: i32, kind: SourceKind, writable_by: &str) -> ParamSource {
    ParamSource {
        id: SourceId(id),
        key: key.to_owned(),
        label: key.to_owned(),
        default_rank: rank,
        kind,
        writable_by: writable_by.to_owned(),
        note: String::new(),
        ranks: Vec::new(),
    }
}

impl Default for Datasheets {
    /// The backend's `manual`, `digikey` and `vendor` sources and berc's `kicad_field`; the resistor's Digi-Key
    /// datasheet URL
    fn default() -> Self {
        let mut d = Datasheets {
            sources: vec![
                source(1, "manual", 100, SourceKind::Person, "lab:write"),
                source(2, "digikey", 90, SourceKind::Harvester, ""),
                source(3, "vendor", 80, SourceKind::Import, ""),
                source(20, "kicad_field", 65, SourceKind::Import, "lab:write"),
            ],
            facts: Vec::new(),
        };
        d.push(RESISTOR_ID, SourceId(2), RESISTOR_DATASHEET, "");
        d
    }
}

/// The part a fact is of, none for another owner
fn part_of(f: &Fact) -> Option<i32> {
    match f.owner {
        ParamOwner::Part(PartId(p)) => Some(p),
        _ => None,
    }
}

impl Datasheets {
    fn push(&mut self, part: i32, source: SourceId, url: &str, note: &str) -> Fact {
        let s = self
            .sources
            .iter()
            .find(|s| s.id == source)
            .expect("a source of the fake");
        let at = parse(&format!("\"{FACTS_AT}\""));
        let fact = Fact {
            id: FactId(500 + self.facts.len() as i64),
            def_id: ParameterDefId(DATASHEET_DEF_ID),
            res_def_id: ParameterDefId(DATASHEET_DEF_ID),
            owner: ParamOwner::Part(PartId(part)),
            source_id: s.id,
            source: s.key.clone(),
            value: Variant::Str(url.to_owned()),
            raw: Some(url.to_owned()),
            unparsed: false,
            warnings: Vec::new(),
            ext: None,
            cond: Cond::new(),
            at,
            seen_at: at,
            recorded: at,
            superseded_at: None,
            retracted: false,
            rank_override: None,
            eff_rank: s.default_rank,
            note: note.to_owned(),
            by: BY.to_owned(),
            reviewed: None,
            derived_from: None,
        };
        self.facts.push(fact.clone());
        fact
    }

    /// The current facts of `part`
    pub fn of_part(&self, part: i32) -> Vec<Fact> {
        self.facts
            .iter()
            .filter(|f| part_of(f) == Some(part) && f.superseded_at.is_none())
            .cloned()
            .collect()
    }

    /// The resolved value of `part`: its best ranked current fact
    pub fn resolved(&self, part: i32) -> Option<Resolved> {
        let best = self
            .of_part(part)
            .into_iter()
            .filter(|f| !f.retracted)
            .max_by_key(|f| (f.eff_rank, f.id))?;
        let mut r = Resolved::new(best.res_def_id, "Datasheet Url", best.value.clone());
        r.key = Some("datasheet".to_owned());
        r.raw = best.raw.clone();
        r.fact_id = best.id;
        r.source_id = best.source_id;
        r.source = best.source.clone();
        r.eff_rank = best.eff_rank;
        r.at = best.at;
        Some(r)
    }

    /// `/params/set` of the datasheet definition by a `lab:write` client, on a part of `parts`
    pub fn set(&mut self, q: &SetParam, parts: &[i32]) -> Answer {
        if !matches!(&q.def, DefRef::Key(k) if k == "datasheet") {
            return error(
                400,
                "the fake states only the definition with key datasheet",
            );
        }
        let ParamOwner::Part(PartId(part)) = q.owner else {
            return error(400, "the fake states facts of parts only");
        };
        if !parts.contains(&part) {
            return error(404, &format!("no part {part}"));
        }
        let key = q.source.as_deref().unwrap_or("manual");
        let Some(source) = self.sources.iter().find(|s| s.key == key) else {
            return error(400, &format!("no source '{key}'"));
        };
        if source.writable_by.is_empty() {
            return error(
                403,
                &format!("source '{key}' is written by the backend only"),
            );
        }
        let source = source.id;
        let Some(Variant::Str(url)) = q.value.clone() else {
            return error(400, "the fake takes a text value");
        };
        let current = self.facts.iter_mut().find(|f| {
            part_of(f) == Some(part) && f.source_id == source && f.superseded_at.is_none()
        });
        if let Some(f) = current {
            if f.value == Variant::Str(url.clone()) && !f.retracted {
                return json(
                    200,
                    &SetOutcome {
                        fact: f.clone(),
                        changed: false,
                    },
                );
            }
            f.superseded_at = Some(f.at);
        }
        let fact = self.push(part, source, &url, &q.note);
        json(
            200,
            &SetOutcome {
                fact,
                changed: true,
            },
        )
    }
}
