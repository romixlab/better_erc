//! Test-only: a fake mx3 backend for the tests of the mx3 part source and the document library (VIEW-26).
//!
//! [`FakeMx3::start`] serves `/params/by_mpn`, `/fp/review/check`, `/fp/review/accept`, `/lib/by_mpn` and
//! `/lib/docs` on a local port ([`http::Server`]) from a fixed part catalogue ([`RESISTOR`], [`CAPACITOR`]; any
//! other MPN is not found) and a fixed set of library documents, one per [`By`] source ([`LIB_LINK_MPN`] and the
//! other `LIB_*_MPN` constants), plus the reviews accepted so far. Requests and answers are mx3_api_lab's own
//! types. A request without `Authorization: Bearer` [`TOKEN`] is answered 401, as the backend answers a missing
//! or wrong token.

pub mod http;

use chrono::NaiveDate;
use http::{Answer, Request, Server};
use mx3_api_lab::endpoints::fp_review::{
    EP_FP_REVIEW_ACCEPT, EP_FP_REVIEW_CHECK, FpAccepted, FpCheck, FpCheckItem, FpCheckQuery,
    FpCheckStatus, FpCover, FpCoverSpec, FpReview, FpReviewAccept,
};
use mx3_api_lab::endpoints::library::{
    By, DocFile, Docs, DocsQuery, EP_LIB_BY_MPN, EP_LIB_DOCS, Ident, LibPath, MpnDoc as LibMpnDoc,
    MpnDocs as LibMpnDocs,
};
use mx3_api_lab::endpoints::mpn::{MpnReading, MpnSegment};
use mx3_api_lab::endpoints::parameter::ParameterDefId;
use mx3_api_lab::endpoints::params::{
    ByMpn, ByMpnResult, EP_PARAMS_BY_MPN, FactId, MpnResult, Resolved, SourceId,
};
use mx3_api_lab::endpoints::part::{LookupPartResult, PartId};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

/// The token the fake takes
pub const TOKEN: &str = "mx3_0123456789abcdef-test-token";

/// A Yageo RC 0402 resistor: resistance, tolerance and power; a retracted dielectric
pub const RESISTOR: &str = "RC0402FR-0710KL";
/// Its mx3 part id
pub const RESISTOR_ID: i32 = 12;
/// A Murata GRM 0402 capacitor: capacitance, an absolute tolerance, rated voltage, a dielectric two sources
/// disagree on
pub const CAPACITOR: &str = "GRM155R71C104KA88D";
/// Its mx3 part id
pub const CAPACITOR_ID: i32 = 13;
/// When every fact of the catalogue was stated
pub const FACTS_AT: &str = "2026-11-20T10:00:00Z";

/// The keys the fake has definitions for; any other asked key is unknown
const KNOWN_KEYS: &[&str] = &[
    "capacitance",
    "case_eia",
    "dielectric",
    "power",
    "rated_voltage",
    "resistance",
    "technology",
    "tolerance",
];

/// An MPN whose document a fetch by part number saved or found ([`By::Link`])
pub const LIB_LINK_MPN: &str = "LIB-LINK-MPN";
/// An MPN whose document's origin is one of its datasheet URLs ([`By::Source`])
pub const LIB_SOURCE_MPN: &str = "LIB-SOURCE-MPN";
/// An MPN a document's file name matches, with no origin URL ([`By::Name`])
pub const LIB_NAME_MPN: &str = "LIB-NAME-MPN";
/// An MPN a person covered by part number ([`By::Cover`])
pub const LIB_COVER_MPN: &str = "LIB-COVER-MPN";
/// An MPN a document covers by family, its MPN decode reading in the family ([`By::Family`])
pub const LIB_FAMILY_MPN: &str = "LIB-FAMILY-MPN";
/// An MPN a document's pattern matches ([`By::Pattern`])
pub const LIB_PATTERN_MPN: &str = "LIB-PATTERN-MPN";
/// An MPN printed on a document's first pages ([`By::Text`])
pub const LIB_TEXT_MPN: &str = "LIB-TEXT-MPN";
/// An MPN the library has no document for
pub const LIB_NONE_MPN: &str = "LIB-NONE-MPN";

/// Every `LIB_*_MPN` the fake has a document for, by [`By`] source
pub const LIB_MPNS: &[&str] = &[
    LIB_LINK_MPN,
    LIB_SOURCE_MPN,
    LIB_NAME_MPN,
    LIB_COVER_MPN,
    LIB_FAMILY_MPN,
    LIB_PATTERN_MPN,
    LIB_TEXT_MPN,
];

/// A fake mx3 backend on a local port
pub struct FakeMx3 {
    server: Server,
    reviews: Arc<Mutex<Vec<FpReview>>>,
}

impl FakeMx3 {
    /// Serves on a free port of 127.0.0.1, with no review yet
    pub fn start() -> FakeMx3 {
        let reviews: Arc<Mutex<Vec<FpReview>>> = Arc::new(Mutex::new(Vec::new()));
        let held = Arc::clone(&reviews);
        let server = Server::start(move |req| answer(req, &held));
        FakeMx3 { server, reviews }
    }

    /// `http://127.0.0.1:PORT`
    pub fn url(&self) -> &str {
        self.server.url()
    }

    /// Every request so far, in order
    pub fn requests(&self) -> Vec<Request> {
        self.server.requests()
    }

    /// The reviews accepted so far
    ///
    /// # Panics
    ///
    /// When the server thread panicked while holding them.
    pub fn reviews(&self) -> Vec<FpReview> {
        self.reviews.lock().expect("no panic while held").clone()
    }
}

fn json<T: Serialize>(status: u16, v: &T) -> Answer {
    (
        status,
        serde_json::to_string(v).expect("the answer serialises"),
    )
}

fn error(status: u16, msg: &str) -> Answer {
    json(status, &serde_json::json!({ "error": msg }))
}

fn body<T: DeserializeOwned>(req: &Request) -> Result<T, Answer> {
    serde_json::from_slice(&req.body).map_err(|e| error(400, &e.to_string()))
}

fn answer(req: &Request, reviews: &Mutex<Vec<FpReview>>) -> Answer {
    if req.authorization.as_deref() != Some(&format!("Bearer {TOKEN}")) {
        return error(401, "no valid token: run `mx3 token set`");
    }
    let mut reviews = reviews.lock().expect("no panic while held");
    let r = match (req.method.as_str(), req.path.as_str()) {
        ("POST", EP_PARAMS_BY_MPN) => body(req).map(|q| json(200, &by_mpn(&q))),
        ("POST", EP_FP_REVIEW_CHECK) => body(req).map(|q| json(200, &check(&q, &reviews))),
        ("POST", EP_FP_REVIEW_ACCEPT) => body(req).and_then(|q| accept(q, &mut reviews)),
        ("POST", EP_LIB_BY_MPN) => {
            body(req).map(|q: mx3_api_lab::endpoints::library::ByMpnQuery| {
                json(
                    200,
                    &q.mpns.iter().map(|m| library_docs(m)).collect::<Vec<_>>(),
                )
            })
        }
        ("POST", EP_LIB_DOCS) => body(req).map(|q: DocsQuery| json(200, &library_catalog(&q))),
        _ => Err(error(404, "no such route")),
    };
    r.unwrap_or_else(|e| e)
}

fn parse<T: DeserializeOwned>(text: &str) -> T {
    serde_json::from_str(text).expect("a fixture value")
}

fn resolved(key: &str, value: &str, source: &str, rank: i32) -> Resolved {
    let mut r = Resolved::new(ParameterDefId(1), key, parse(value));
    r.key = Some(key.to_owned());
    r.fact_id = FactId(100);
    r.source_id = SourceId(2);
    r.source = source.to_owned();
    r.eff_rank = rank;
    r.at = parse(&format!("\"{FACTS_AT}\""));
    r
}

fn segment(key: &str, value: &str) -> MpnSegment {
    MpnSegment {
        name: key.to_owned(),
        start: 0,
        end: 0,
        code: value.to_owned(),
        key: Some(key.to_owned()),
        value: Some(value.to_owned()),
        free: false,
    }
}

fn decoded(scheme: &str, segments: Vec<MpnSegment>) -> MpnReading {
    MpnReading {
        outcome: "decoded".to_owned(),
        scheme: Some(scheme.to_owned()),
        version: Some(1),
        class: None,
        segments,
        schemes: Vec::new(),
    }
}

/// The catalogue's part of `mpn`: its id, every value and its MPN reading
fn catalogue(mpn: &str) -> Option<(i32, Vec<Resolved>, MpnReading)> {
    let pct = r#"{"Unit":{"prefix":"Centi","base":"Ratio","exp":1}}"#;
    let pf = r#"{"Unit":{"prefix":"Pico","base":"Farad","exp":1}}"#;
    match mpn {
        RESISTOR => {
            let mut dielectric = resolved("dielectric", r#"{"Str":"C0G"}"#, "manual", 90);
            dielectric.retracted = true;
            Some((
                RESISTOR_ID,
                vec![
                    resolved(
                        "resistance",
                        r#"{"SI":{"value":{"F64":10.0},"unit":{"Unit":{"prefix":"Kilo","base":"Ohm","exp":1}}}}"#,
                        "digikey",
                        50,
                    ),
                    resolved(
                        "tolerance",
                        &format!(
                            r#"{{"Tolerance":{{"min":{{"F64":-1.0}},"min_unit":{pct},"max":{{"F64":1.0}},"max_unit":{pct}}}}}"#
                        ),
                        "mpn_decode",
                        60,
                    ),
                    resolved(
                        "power",
                        r#"{"SI":{"value":{"F64":62.5},"unit":{"Unit":{"prefix":"Milli","base":"Watt","exp":1}}}}"#,
                        "digikey",
                        50,
                    ),
                    dielectric,
                ],
                decoded(
                    "yageo.rc",
                    vec![segment("case_eia", "0402"), segment("tolerance", "F")],
                ),
            ))
        }
        CAPACITOR => {
            let mut dielectric = resolved("dielectric", r#"{"Str":"X7R"}"#, "digikey", 50);
            dielectric.conflict = vec![FactId(101)];
            dielectric.raw = Some("X7R (class 2)".to_owned());
            Some((
                CAPACITOR_ID,
                vec![
                    resolved(
                        "capacitance",
                        r#"{"SI":{"value":{"F64":100.0},"unit":{"Unit":{"prefix":"Nano","base":"Farad","exp":1}}}}"#,
                        "digikey",
                        50,
                    ),
                    resolved(
                        "tolerance",
                        &format!(
                            r#"{{"Tolerance":{{"min":{{"F64":-0.25}},"min_unit":{pf},"max":{{"F64":0.25}},"max_unit":{pf}}}}}"#
                        ),
                        "digikey",
                        50,
                    ),
                    resolved(
                        "rated_voltage",
                        r#"{"SI":{"value":{"F64":16.0},"unit":{"Unit":{"prefix":"Unit","base":"Volt","exp":1}}}}"#,
                        "digikey",
                        50,
                    ),
                    dielectric,
                ],
                decoded("murata.grm", vec![segment("case_eia", "0402")]),
            ))
        }
        _ => None,
    }
}

fn by_mpn(q: &ByMpn) -> ByMpnResult {
    let unknown_keys: Vec<String> = q
        .keys
        .iter()
        .filter(|k| !KNOWN_KEYS.contains(&k.as_str()))
        .cloned()
        .collect();
    let known: Vec<&String> = q
        .keys
        .iter()
        .filter(|k| !unknown_keys.contains(k))
        .collect();
    let results = q
        .mpns
        .iter()
        .map(|key| {
            let Some((id, values, reading)) = catalogue(&key.mpn) else {
                return MpnResult {
                    mpn: key.clone(),
                    part: LookupPartResult::NotFound,
                    values: Vec::new(),
                    missing: Vec::new(),
                    facts: None,
                    mpn_decode: MpnReading {
                        outcome: "no_scheme".to_owned(),
                        scheme: None,
                        version: None,
                        class: None,
                        segments: Vec::new(),
                        schemes: Vec::new(),
                    },
                };
            };
            let values: Vec<Resolved> = values
                .into_iter()
                .filter(|v| v.key.as_ref().is_some_and(|k| known.contains(&k)))
                .collect();
            let missing = known
                .iter()
                .filter(|k| {
                    !values
                        .iter()
                        .any(|v| v.key.as_ref() == Some(**k) && !v.retracted)
                })
                .map(|k| (*k).clone())
                .collect();
            MpnResult {
                mpn: key.clone(),
                part: LookupPartResult::Found {
                    id: PartId(id),
                    pn: key.mpn.clone(),
                },
                values,
                missing,
                facts: None,
                mpn_decode: reading,
            }
        })
        .collect();
    ByMpnResult {
        unknown_keys,
        results,
    }
}

fn lib_hash(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}

/// One document of the fixture library: `share/path` under `datasheets/Fixture`, its hash and document number
fn lib_doc(
    byte: u8,
    file: &str,
    doc: &str,
    by: By,
    url: Option<&str>,
    cover: Option<&str>,
) -> LibMpnDoc {
    LibMpnDoc {
        hash: lib_hash(byte),
        ident: Ident {
            doc: Some(doc.to_owned()),
            rev: Some("1".to_owned()),
            ..Ident::default()
        },
        url: url.map(str::to_owned),
        by,
        cover: cover.map(str::to_owned),
        paths: vec![LibPath {
            share: "datasheets".to_owned(),
            path: format!("Fixture/{file}"),
        }],
    }
}

/// The library's documents of `mpn` (`/lib/by_mpn`): one of the `LIB_*_MPN` fixtures, each a different [`By`]
/// source, [`LIB_NONE_MPN`] with none, any other MPN not in the library at all
fn library_docs(mpn: &str) -> LibMpnDocs {
    let doc = match mpn {
        LIB_LINK_MPN => Some(lib_doc(
            1,
            "link.pdf",
            "DSL1",
            By::Link,
            Some("https://x/link.pdf"),
            None,
        )),
        LIB_SOURCE_MPN => Some(lib_doc(
            2,
            "source.pdf",
            "DSS1",
            By::Source,
            Some("https://x/source.pdf"),
            None,
        )),
        LIB_NAME_MPN => Some(lib_doc(
            3,
            "lib-name-mpn-datasheet.pdf",
            "DSN1",
            By::Name,
            None,
            None,
        )),
        LIB_COVER_MPN => Some(lib_doc(4, "cover.pdf", "DSC1", By::Cover, None, None)),
        LIB_FAMILY_MPN => Some(lib_doc(
            5,
            "family.pdf",
            "DSF1",
            By::Family,
            None,
            Some("yageo.rc case_eia=0402"),
        )),
        LIB_PATTERN_MPN => Some(lib_doc(
            6,
            "pattern.pdf",
            "DSP1",
            By::Pattern,
            None,
            Some("LIB-PATTERN-*"),
        )),
        LIB_TEXT_MPN => Some(lib_doc(7, "text.pdf", "DST1", By::Text, None, None)),
        LIB_NONE_MPN => None,
        _ => {
            return LibMpnDocs {
                mpn: mpn.to_owned(),
                maker: None,
                known: false,
                urls: Vec::new(),
                docs: Vec::new(),
                why: Some("not in the library".to_owned()),
            };
        }
    };
    let urls = doc
        .as_ref()
        .and_then(|d| d.url.clone())
        .into_iter()
        .collect();
    let why = doc
        .is_none()
        .then(|| "the library has no document for it".to_owned());
    LibMpnDocs {
        mpn: mpn.to_owned(),
        maker: Some("Maker".to_owned()),
        known: true,
        urls,
        docs: doc.into_iter().collect(),
        why,
    }
}

/// The catalog (`/lib/docs`): the `LIB_*_MPN` fixtures' files, and one gone
fn library_catalog(q: &DocsQuery) -> Docs {
    let since = NaiveDate::from_ymd_opt(2026, 1, 1).expect("a valid date");
    let file = |byte: u8, name: &str, doc: &str, gone: bool| DocFile {
        share: "datasheets".to_owned(),
        path: format!("Fixture/{name}"),
        hash: lib_hash(byte),
        size: 1000,
        since,
        gone: gone.then_some(since),
        ident: Ident {
            doc: Some(doc.to_owned()),
            rev: Some("1".to_owned()),
            ..Ident::default()
        },
        place: mx3_api_lab::endpoints::library::Place::default(),
        title: None,
        origin: None,
        seq: 1,
    };
    let mut files = vec![
        file(1, "link.pdf", "DSL1", false),
        file(4, "cover.pdf", "DSC1", false),
    ];
    if q.gone {
        files.push(file(9, "old.pdf", "DSOLD", true));
    }
    Docs {
        seq: 1,
        unchanged: false,
        files,
    }
}

/// Whether `cover` takes in the catalogue's part of `mpn`, as the backend judges it: the part, or its MPN
/// reading of the family's scheme with each of its values
fn takes_in(cover: &FpCover, mpn: &str) -> bool {
    let Some((id, _, reading)) = catalogue(mpn) else {
        return false;
    };
    match cover {
        FpCover::Mpn { part_id, .. } => part_id.0 == id,
        FpCover::Family { scheme, segments } => {
            reading.scheme.as_ref() == Some(scheme)
                && segments.iter().all(|(k, v)| {
                    reading
                        .segments
                        .iter()
                        .any(|s| s.key.as_ref() == Some(k) && s.value.as_ref() == Some(v))
                })
        }
    }
}

fn check(q: &FpCheckQuery, reviews: &[FpReview]) -> FpCheck {
    let mut named: Vec<i32> = Vec::new();
    let items = q
        .mpns
        .iter()
        .map(|mpn| {
            let Some((id, _, _)) = catalogue(mpn) else {
                return FpCheckItem {
                    mpn: mpn.clone(),
                    part_id: None,
                    maker: None,
                    status: FpCheckStatus::UnknownMpn,
                    covered_by: Vec::new(),
                    other_hashes: Vec::new(),
                };
            };
            let holding = reviews
                .iter()
                .filter(|r| r.retracted.is_none() && r.covers.iter().any(|c| takes_in(c, mpn)));
            let (mut covered_by, mut other_hashes) = (Vec::new(), Vec::new());
            for r in holding {
                if r.hash == q.hash {
                    covered_by.push(r.id);
                } else {
                    other_hashes.push(r.id);
                }
                named.push(r.id);
            }
            FpCheckItem {
                mpn: mpn.clone(),
                part_id: Some(PartId(id)),
                maker: Some("Maker".to_owned()),
                status: if covered_by.is_empty() {
                    FpCheckStatus::Uncovered
                } else {
                    FpCheckStatus::Covered
                },
                covered_by,
                other_hashes,
            }
        })
        .collect();
    FpCheck {
        hash: q.hash.clone(),
        items,
        reviews: reviews
            .iter()
            .filter(|r| named.contains(&r.id))
            .cloned()
            .collect(),
    }
}

fn accept(q: FpReviewAccept, reviews: &mut Vec<FpReview>) -> Result<Answer, Answer> {
    if !mx3_api_lab::endpoints::fp_review::valid_hash(&q.hash) {
        return Err(error(400, "not a review hash"));
    }
    let mut covers = Vec::new();
    for c in q.covers {
        covers.push(match c {
            FpCoverSpec::Mpn { mpn, part_id } => {
                let id = part_id
                    .map(|p| p.0)
                    .or_else(|| catalogue(&mpn).map(|(id, _, _)| id))
                    .ok_or_else(|| error(400, &format!("no part has MPN {mpn}")))?;
                FpCover::Mpn {
                    part_id: PartId(id),
                    mpn,
                }
            }
            FpCoverSpec::Family { scheme, segments } => FpCover::Family { scheme, segments },
        });
    }
    let review = FpReview {
        id: 7 + reviews.len() as i32,
        hash: q.hash,
        footprint: q.footprint,
        source: q.source,
        reviewer: q.reviewer.unwrap_or_else(|| "fake".to_owned()),
        reviewed_on: q.reviewed_on.unwrap_or_else(|| parse("\"2026-11-25\"")),
        note: q.note,
        covers,
        recorded: parse(&format!("\"{FACTS_AT}\"")),
        by: "test client".to_owned(),
        retracted: None,
    };
    let parts_now = review
        .covers
        .iter()
        .map(|c| {
            [RESISTOR, CAPACITOR]
                .iter()
                .filter(|m| takes_in(c, m))
                .count() as u32
        })
        .collect();
    reviews.push(review.clone());
    Ok(json(200, &FpAccepted { review, parts_now }))
}
