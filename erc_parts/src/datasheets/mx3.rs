//! A design's datasheet URLs stated in the mx3 part store (TOOL-18): each MPN's URL as a fact of the definition
//! with key [`DEF_KEY`] on the MPN's part, from the source [`SOURCE`], ranked below the makers' own data.
//!
//! That definition is what mx3's `/lib/by_mpn` and `mx3 lib fetch --mpn` read a part's datasheet URLs from (with
//! any definition merged into it, and the one named `Datasheet Url`). A part that has the URL already, from any
//! source, is left as it is; an MPN with no single part, or whose parts give different URLs, is listed and left.
//! Stating the same URL again from the same source changes nothing in mx3, so a second run writes nothing.

use super::{MpnDatasheets, UrlRefs};
use crate::mx3::{Mx3Error, Mx3Source, answer_error};
use crate::source::{MPNS_PER_CALL_MAX, SourceError};
use mx3_api_lab::endpoints::library::valid_web_url;
use mx3_api_lab::endpoints::params::{
    self, ByMpn, DefRef, Fact, MpnKey, MpnResult, ParamOwner, SetParam,
};
use mx3_api_lab::endpoints::part::LookupPartResult;
use mx3_api_lab::rvariant::Variant;
use serde::Serialize;

/// The curated key of the datasheet URL definition
pub const DEF_KEY: &str = "datasheet";
/// The parameter source of the facts written: a KiCad symbol's field
pub const SOURCE: &str = "kicad_field";
/// Its rank: below the makers' own data (`vendor` 80, `datasheet` 75) and generated data (`stm32_data` 70), above
/// BOM files and the distributors after Digi-Key
pub const SOURCE_RANK: i32 = 65;
/// How an mx3 admin adds [`SOURCE`] once
pub const SOURCE_SETUP: &str = "mx3 param source add kicad_field \"KiCad field\" --rank 65 --kind import \
--writable-by lab:write --note \"a KiCad symbol's Datasheet field (berc parts datasheets --to-mx3)\"";
/// The most references a fact's note names; the rest are counted
pub const NOTE_REFERENCES_MAX: usize = 20;

/// Whether [`state`] writes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Write {
    /// State each missing URL
    Yes,
    /// Ask mx3 what it has, write nothing
    DryRun,
}

/// What became of one MPN
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The URL was stated on the part
    Written,
    /// A dry run: the URL would be stated
    WouldWrite,
    /// The part has the URL already
    Present,
    /// mx3 has no part of the MPN; nothing is created
    NoPart,
    /// mx3 has several parts of the MPN
    SeveralParts,
    /// The board's parts give the MPN different URLs
    Disagree,
    /// The URL is not one mx3 takes (too long, a control character)
    BadUrl,
}

impl Outcome {
    /// The outcome in words, for the text report
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Written => "written",
            Outcome::WouldWrite => "would write",
            Outcome::Present => "present",
            Outcome::NoPart => "no mx3 part",
            Outcome::SeveralParts => "several mx3 parts",
            Outcome::Disagree => "URLs disagree",
            Outcome::BadUrl => "not a URL mx3 takes",
        }
    }
}

/// One MPN of the design and what became of its URL
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Stated {
    pub mpn: String,
    pub outcome: Outcome,
    /// The mx3 part, when there is one
    pub part: Option<i32>,
    /// The URLs the board gives it, with their references
    pub urls: Vec<UrlRefs>,
    /// For [`Outcome::Present`]: the source of the fact that has the URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// A fact's datasheet URL as mx3's library reads it: its text trimmed, `//host/x` taken as https
fn fact_url(f: &Fact) -> Option<String> {
    let text = match (&f.value, &f.raw) {
        (Variant::Str(s), _) => s.as_str(),
        (_, Some(raw)) => raw.as_str(),
        _ => return None,
    };
    let t = text.trim();
    Some(match t.strip_prefix("//") {
        Some(rest) => format!("https://{rest}"),
        None => t.to_owned(),
    })
}

/// The note of a written fact: where it came from and the references that give it
fn note_of(origin: &str, refs: &UrlRefs) -> String {
    let shown: Vec<&str> = refs
        .references
        .iter()
        .take(NOTE_REFERENCES_MAX)
        .map(String::as_str)
        .collect();
    let more = refs.references.len().saturating_sub(NOTE_REFERENCES_MAX);
    let tail = if more > 0 {
        format!(" and {more} more")
    } else {
        String::new()
    };
    format!(
        "KiCad Datasheet field of {origin}: {}{tail}",
        shown.join(" ")
    )
}

/// Each of `board`'s MPNs with one URL stated in mx3 (none on a dry run), the note naming `origin` (the project)
/// and the references
///
/// Fails before writing anything when mx3 has no source [`SOURCE`] or no definition [`DEF_KEY`]; a failed write
/// stops the run, and running it again takes up where it stopped.
pub fn state(
    src: &Mx3Source,
    board: &[MpnDatasheets],
    origin: &str,
    write: Write,
) -> Result<Vec<Stated>, SourceError> {
    let sources = params::sources(src.client()).map_err(|e| {
        src.call_error("parameter sources".to_owned(), params::EP_PARAMS_SOURCES, e)
    })?;
    if !sources.iter().any(|s| s.key.eq_ignore_ascii_case(SOURCE)) {
        return Err(SourceError {
            what: "datasheet facts".to_owned(),
            cause: Box::new(Mx3Error::NoSource {
                key: SOURCE.to_owned(),
                setup: SOURCE_SETUP.to_owned(),
            }),
        });
    }
    let mut out: Vec<Stated> = board
        .iter()
        .map(|m| Stated {
            mpn: m.mpn.clone(),
            outcome: match m.urls.as_slice() {
                [one] if valid_web_url(&one.url) => Outcome::WouldWrite,
                [_] => Outcome::BadUrl,
                _ => Outcome::Disagree,
            },
            part: None,
            urls: m.urls.clone(),
            source: None,
        })
        .collect();
    let asked: Vec<usize> = (0..out.len())
        .filter(|&i| out[i].outcome == Outcome::WouldWrite)
        .collect();
    for chunk in asked.chunks(MPNS_PER_CALL_MAX) {
        let results = lookup(src, chunk.iter().map(|&i| out[i].mpn.as_str()))?;
        for (&i, r) in chunk.iter().zip(results) {
            decide(src, &mut out[i], &r, origin, write)?;
        }
    }
    Ok(out)
}

/// The by_mpn answer of `mpns` for [`DEF_KEY`], with the facts, one per MPN
fn lookup<'a>(
    src: &Mx3Source,
    mpns: impl Iterator<Item = &'a str>,
) -> Result<Vec<MpnResult>, SourceError> {
    let q = ByMpn {
        mpns: mpns
            .map(|m| MpnKey {
                manufacturer: None,
                mpn: m.to_owned(),
            })
            .collect(),
        keys: vec![DEF_KEY.to_owned()],
        with_facts: true,
    };
    let what = format!("datasheet facts of {} MPNs", q.mpns.len());
    let answer = params::by_mpn(src.client(), &q)
        .map_err(|e| src.call_error(what.clone(), params::EP_PARAMS_BY_MPN, e))?;
    if answer.unknown_keys.iter().any(|k| k == DEF_KEY) {
        return Err(answer_error(
            what,
            format!("no parameter definition with key `{DEF_KEY}`"),
        ));
    }
    if answer.results.len() != q.mpns.len() {
        return Err(answer_error(
            what,
            format!("{} results for {} MPNs", answer.results.len(), q.mpns.len()),
        ));
    }
    Ok(answer.results)
}

/// The outcome of one MPN with one valid URL, written when `write` says so
fn decide(
    src: &Mx3Source,
    s: &mut Stated,
    r: &MpnResult,
    origin: &str,
    write: Write,
) -> Result<(), SourceError> {
    let id = match &r.part {
        LookupPartResult::Found { id, .. } => *id,
        LookupPartResult::NotFound => {
            s.outcome = Outcome::NoPart;
            return Ok(());
        }
        LookupPartResult::NotUnique(_) => {
            s.outcome = Outcome::SeveralParts;
            return Ok(());
        }
    };
    s.part = Some(id.0);
    let refs = s.urls.first().expect("only one-URL MPNs are looked up");
    let present = r
        .facts
        .iter()
        .flatten()
        .filter(|f| f.superseded_at.is_none() && !f.retracted)
        .find(|f| fact_url(f).as_deref() == Some(refs.url.as_str()));
    if let Some(f) = present {
        s.outcome = Outcome::Present;
        s.source = Some(f.source.clone());
        return Ok(());
    }
    if write == Write::DryRun {
        return Ok(());
    }
    let q = SetParam {
        owner: ParamOwner::Part(id),
        def: DefRef::Key(DEF_KEY.to_owned()),
        value: Some(Variant::Str(refs.url.clone())),
        raw: Some(refs.url.clone()),
        source: Some(SOURCE.to_owned()),
        cond: params::Cond::new(),
        note: note_of(origin, refs),
        store_unparsed: false,
        create_def: false,
    };
    params::set(src.client(), &q).map_err(|e| {
        src.call_error(
            format!("the datasheet URL of {}", s.mpn),
            params::EP_PARAMS_SET,
            e,
        )
    })?;
    s.outcome = Outcome::Written;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{NOTE_REFERENCES_MAX, UrlRefs, note_of};

    #[test]
    fn note_names_references_up_to_the_limit() {
        let few = UrlRefs {
            url: "https://x".to_owned(),
            references: vec!["R1".to_owned(), "R2".to_owned()],
        };
        assert_eq!(
            note_of("b153b", &few),
            "KiCad Datasheet field of b153b: R1 R2"
        );
        let many = UrlRefs {
            url: "https://x".to_owned(),
            references: (1..=NOTE_REFERENCES_MAX + 3)
                .map(|i| format!("C{i}"))
                .collect(),
        };
        assert!(
            note_of("b", &many).ends_with(" C20 and 3 more"),
            "{}",
            note_of("b", &many)
        );
    }
}
