//! The mx3 part store as a part source (cargo feature `mx3`): part facts from `params/by_mpn`, footprint reviews
//! from `fp/review/check`, a review recorded with `fp/review/accept`, over mx3_api_lab's blocking client.
//!
//! The backend and its token are found as the mx3 command line finds them ([`Connection::from_env`]): the URL
//! from `MX3_URL`, else tpm's `config.toml` `[share] mx3_url`, else [`DEFAULT_URL`]; the token from `MX3_TOKEN`,
//! else `~/.config/mx3/token`. The token goes only into the `Authorization` header: no error, `Debug` or lock
//! names it.

use crate::keys;
use crate::lock::{
    Cover, Coverage, Decode, FactValue, LockedFact, LockedPair, LockedPart, LockedReview,
    PartMatch, Placement,
};
use crate::source::{
    MPNS_PER_CALL_MAX, NewReview, PartsAnswer, ReviewsAnswer, Source, SourceError,
};
use mx3_api_lab::blocking::BlockingClient;
use mx3_api_lab::endpoints::fp_review::{
    self, FpCheck, FpCheckQuery, FpCheckStatus, FpCover, FpCoverSpec, FpPlacement, FpReview,
    FpReviewAccept, FpSource,
};
use mx3_api_lab::endpoints::params::{self, BY_MPN_KEYS_MAX, BY_MPN_MPNS_MAX, ByMpn, MpnKey};
use mx3_api_lab::endpoints::part::LookupPartResult;
use mx3_api_lab::rvariant::Variant;
use std::collections::BTreeSet;
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

const _: () = assert!(
    MPNS_PER_CALL_MAX <= BY_MPN_MPNS_MAX,
    "a refresh chunk fits one by_mpn call"
);
const _: () = assert!(
    keys::FACT_KEYS.len() <= BY_MPN_KEYS_MAX,
    "every fact key fits one by_mpn call"
);

/// The environment variable naming the backend
pub const URL_ENV: &str = "MX3_URL";
/// The environment variable holding the token; it wins over the token file
pub const TOKEN_ENV: &str = "MX3_TOKEN";
/// The backend when nothing names one
pub const DEFAULT_URL: &str = "http://localhost:3000";
/// The largest tpm `config.toml` read, bytes
pub const CONFIG_SIZE_BYTES_MAX: u64 = 1 << 20;
/// The largest token file read, bytes
pub const TOKEN_FILE_SIZE_BYTES_MAX: u64 = 4096;

/// How the source is found, for `--help`
pub const CONNECTION_HELP: &str = "the mx3 part store: its URL from MX3_URL, else tpm's config.toml [share] \
mx3_url (TPM_ROOT, else ~/git/tpm), else http://localhost:3000; its token from MX3_TOKEN, else \
~/.config/mx3/token (`mx3 token set`). The token is never printed.";

/// Where the backend is and the token sent to it
#[derive(Clone)]
pub struct Connection {
    pub url: String,
    /// Where the URL came from (`MX3_URL`, the config file's path, `default`)
    pub url_from: String,
    token: Option<String>,
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("url", &self.url)
            .field("url_from", &self.url_from)
            .field("token", &self.token.as_ref().map(|_| "<set>"))
            .finish()
    }
}

/// What [`Connection::resolve`] reads: the environment variables, as given
#[derive(Clone, Debug, Default)]
pub struct Environment {
    pub mx3_url: Option<String>,
    pub mx3_token: Option<String>,
    pub tpm_root: Option<String>,
    pub home: Option<String>,
}

impl Environment {
    /// This process's environment
    pub fn current() -> Environment {
        let var = |k: &str| std::env::var_os(k).map(|v| v.to_string_lossy().into_owned());
        Environment {
            mx3_url: var(URL_ENV),
            mx3_token: var(TOKEN_ENV),
            tpm_root: var("TPM_ROOT"),
            home: var("HOME"),
        }
    }
}

fn non_empty(v: Option<&String>) -> Option<String> {
    v.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

/// The first `max` bytes of a file as text, `None` when it cannot be read or is longer
fn read_small(path: &Path, max: u64) -> Option<String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(max + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > max {
        return None;
    }
    String::from_utf8(bytes).ok()
}

/// `[share] mx3_url` of the tpm config at `path`
fn tpm_mx3_url(path: &Path) -> Option<String> {
    let text = read_small(path, CONFIG_SIZE_BYTES_MAX)?;
    let table: toml::Table = toml::from_str(&text).ok()?;
    let url = table.get("share")?.get("mx3_url")?.as_str()?.trim();
    (!url.is_empty()).then(|| url.to_owned())
}

impl Connection {
    /// The connection this process's environment names
    pub fn from_env() -> Connection {
        Connection::resolve(&Environment::current())
    }

    /// The connection `env` names: the URL from `MX3_URL`, else `[share] mx3_url` of `config.toml` in `TPM_ROOT`
    /// (else `~/git/tpm`), else [`DEFAULT_URL`]; the token from `MX3_TOKEN`, else `~/.config/mx3/token`
    pub fn resolve(env: &Environment) -> Connection {
        let home = PathBuf::from(env.home.clone().unwrap_or_default());
        let tpm_root =
            non_empty(env.tpm_root.as_ref()).map_or_else(|| home.join("git/tpm"), PathBuf::from);
        let config = tpm_root.join("config.toml");
        let (url, url_from) = if let Some(u) = non_empty(env.mx3_url.as_ref()) {
            (u, URL_ENV.to_owned())
        } else if let Some(u) = tpm_mx3_url(&config) {
            (u, config.display().to_string())
        } else {
            (DEFAULT_URL.to_owned(), "default".to_owned())
        };
        let token = non_empty(env.mx3_token.as_ref()).or_else(|| {
            non_empty(
                read_small(&home.join(".config/mx3/token"), TOKEN_FILE_SIZE_BYTES_MAX).as_ref(),
            )
        });
        Connection {
            url: url.trim_end_matches('/').to_owned(),
            url_from,
            token,
        }
    }

    /// A connection to `url` sending `token`
    pub fn new(url: &str, token: Option<String>) -> Connection {
        Connection {
            url: url.trim_end_matches('/').to_owned(),
            url_from: "given".to_owned(),
            token,
        }
    }

    /// Whether a token was found
    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    /// The token found, for a client of an endpoint this module does not cover itself (the document library's)
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}

/// Why the mx3 source gave no answer
#[derive(Debug)]
pub enum Mx3Error {
    /// No token in `MX3_TOKEN` or the token file
    NoToken,
    /// The call failed or its answer did not decode
    Call {
        url: String,
        endpoint: &'static str,
        error: anyhow::Error,
    },
    /// The answer does not fit what was asked
    Answer(String),
    /// mx3 has no parameter source of this key; an admin adds it once (the text says how)
    NoSource { key: String, setup: String },
    /// A review's date is not `YYYY-MM-DD`
    Date {
        text: String,
        error: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl fmt::Display for Mx3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mx3Error::NoToken => write!(
                f,
                "no mx3 token: set {TOKEN_ENV} or save this PC's token to ~/.config/mx3/token (`mx3 token set`)"
            ),
            Mx3Error::Call {
                url,
                endpoint,
                error,
            } => write!(f, "mx3 at {url}{endpoint}: {error:#}"),
            Mx3Error::Answer(what) => write!(f, "mx3 answered {what}"),
            Mx3Error::NoSource { key, setup } => write!(
                f,
                "mx3 has no parameter source `{key}`; an mx3 admin adds it once: {setup}"
            ),
            Mx3Error::Date { text, error } => {
                write!(f, "review date `{text}` is not YYYY-MM-DD: {error}")
            }
        }
    }
}

impl std::error::Error for Mx3Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Mx3Error::Call { error, .. } => Some(error.as_ref()),
            Mx3Error::Date { error, .. } => Some(error.as_ref()),
            Mx3Error::NoToken | Mx3Error::Answer(_) | Mx3Error::NoSource { .. } => None,
        }
    }
}

/// The mx3 part store
pub struct Mx3Source {
    client: BlockingClient,
    url: String,
}

impl fmt::Debug for Mx3Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mx3Source").field("url", &self.url).finish()
    }
}

impl Mx3Source {
    /// A source on `conn`; refused without a token, since every endpoint needs one
    ///
    /// # Panics
    ///
    /// When the HTTP client cannot be built (no TLS backend), as mx3_api_lab's `BlockingClient::new`.
    pub fn connect(conn: Connection) -> Result<Mx3Source, SourceError> {
        let Some(token) = conn.token else {
            return Err(SourceError {
                what: "the mx3 part source".to_owned(),
                cause: Box::new(Mx3Error::NoToken),
            });
        };
        Ok(Mx3Source {
            client: BlockingClient::new(conn.url.clone()).with_token(Some(token)),
            url: conn.url,
        })
    }

    /// The HTTP client, for calls this module does not cover itself (the datasheet facts')
    pub(crate) fn client(&self) -> &BlockingClient {
        &self.client
    }

    pub(crate) fn call_error(
        &self,
        what: String,
        endpoint: &'static str,
        error: anyhow::Error,
    ) -> SourceError {
        SourceError {
            what,
            cause: Box::new(Mx3Error::Call {
                url: self.url.clone(),
                endpoint,
                error,
            }),
        }
    }
}

pub(crate) fn answer_error(what: String, wrong: String) -> SourceError {
    SourceError {
        what,
        cause: Box::new(Mx3Error::Answer(wrong)),
    }
}

impl Source for Mx3Source {
    fn name(&self) -> String {
        "mx3".to_owned()
    }

    fn parts(&self, mpns: &[String], keys: &[String]) -> Result<PartsAnswer, SourceError> {
        let what = format!("parts of {} MPNs", mpns.len());
        let q = ByMpn {
            mpns: mpns
                .iter()
                .map(|m| MpnKey {
                    manufacturer: None,
                    mpn: m.clone(),
                })
                .collect(),
            keys: keys.to_vec(),
            with_facts: false,
        };
        let answer = params::by_mpn(&self.client, &q)
            .map_err(|e| self.call_error(what.clone(), params::EP_PARAMS_BY_MPN, e))?;
        if answer.results.len() != mpns.len() {
            return Err(answer_error(
                what,
                format!("{} results for {} MPNs", answer.results.len(), mpns.len()),
            ));
        }
        let mut unknown_keys = answer.unknown_keys;
        unknown_keys.sort();
        let known: Vec<String> = keys
            .iter()
            .filter(|k| !unknown_keys.contains(k))
            .cloned()
            .collect();
        let parts = answer
            .results
            .into_iter()
            .map(|r| locked_part(r, &known))
            .collect();
        Ok(PartsAnswer {
            unknown_keys,
            parts,
        })
    }

    fn reviews(&self, hash: &str, mpns: &[String]) -> Result<ReviewsAnswer, SourceError> {
        let what = format!("reviews of {hash} for {} MPNs", mpns.len());
        let q = FpCheckQuery {
            hash: hash.to_owned(),
            mpns: mpns.to_vec(),
        };
        let check: FpCheck = fp_review::check(&self.client, &q)
            .map_err(|e| self.call_error(what.clone(), fp_review::EP_FP_REVIEW_CHECK, e))?;
        if check.hash != hash {
            return Err(answer_error(what, format!("for hash {}", check.hash)));
        }
        Ok(reviews_answer(check))
    }

    fn accept(&self, review: &NewReview) -> Result<LockedReview, SourceError> {
        let what = format!("review of {}", review.hash);
        review.check().map_err(|e| SourceError {
            what: what.clone(),
            cause: Box::new(e),
        })?;
        let q = FpReviewAccept {
            hash: review.hash.clone(),
            footprint: review.footprint.clone(),
            source: FpSource {
                doc: review.doc.clone(),
                doc_rev: review.doc_rev.clone(),
                pdf_blake3: review.pdf_blake3.clone(),
                page: review.page,
                scale: review.scale,
                at: review.at.map(|p| FpPlacement {
                    x: p.x,
                    y: p.y,
                    rotation: p.rotation,
                    mirror: p.mirror,
                }),
            },
            reviewer: Some(review.reviewer.clone()),
            reviewed_on: Some(review.reviewed_on.trim().parse().map_err(|e| SourceError {
                what: what.clone(),
                cause: Box::new(Mx3Error::Date {
                    text: review.reviewed_on.clone(),
                    error: Box::new(e),
                }),
            })?),
            note: review.note.clone(),
            covers: review
                .covers
                .iter()
                .map(|c| match c {
                    Cover::Mpn { mpn } => FpCoverSpec::Mpn {
                        mpn: mpn.clone(),
                        part_id: None,
                    },
                    Cover::Family { scheme, segments } => FpCoverSpec::Family {
                        scheme: scheme.clone(),
                        segments: segments.clone(),
                    },
                })
                .collect(),
        };
        let accepted = fp_review::accept(&self.client, &q)
            .map_err(|e| self.call_error(what.clone(), fp_review::EP_FP_REVIEW_ACCEPT, e))?;
        if accepted.review.hash != review.hash {
            return Err(answer_error(
                what,
                format!("with a review of {}", accepted.review.hash),
            ));
        }
        Ok(locked_review(accepted.review))
    }
}

/// One by_mpn result in the lock's shape: the current values of `keys` (the asked keys mx3 has a definition
/// for), every other one of them missing
fn locked_part(r: params::MpnResult, keys: &[String]) -> LockedPart {
    let (matched, part) = match &r.part {
        LookupPartResult::Found { id, .. } => (PartMatch::Found, Some(id.0.to_string())),
        LookupPartResult::NotFound => (PartMatch::NotFound, None),
        LookupPartResult::NotUnique(_) => (PartMatch::NotUnique, None),
    };
    let d = &r.mpn_decode;
    let decode = Decode {
        outcome: d.outcome.clone(),
        scheme: d.scheme.clone(),
        segments: d
            .segments
            .iter()
            .filter_map(|s| Some((s.key.clone()?, s.value.clone()?)))
            .collect(),
    };
    let mut facts: Vec<LockedFact> = Vec::new();
    for v in r.values.iter().filter(|v| !v.retracted) {
        let Some(key) = v.key.as_ref().filter(|k| keys.contains(k)) else {
            continue;
        };
        if facts.iter().any(|f| f.key == *key) {
            continue;
        }
        facts.push(LockedFact {
            key: key.clone(),
            value: fact_value(key, &v.value, v.unparsed),
            raw: v.raw.clone(),
            source: v.source.clone(),
            rank: v.eff_rank,
            at: v.at.to_rfc3339(),
            conflict: !v.conflict.is_empty(),
        });
    }
    facts.sort_by(|a, b| a.key.cmp(&b.key));
    let missing: BTreeSet<String> = keys
        .iter()
        .filter(|k| !facts.iter().any(|f| f.key == **k))
        .cloned()
        .collect();
    LockedPart {
        mpn: r.mpn.mpn,
        matched,
        part,
        decode: Some(decode),
        missing: missing.into_iter().collect(),
        facts,
    }
}

/// The unit [`keys::FACT_KEYS`] gives `key`, empty for a word
fn unit_of(key: &str) -> &'static str {
    keys::FACT_KEYS
        .iter()
        .find(|k| k.key == key)
        .map_or("", |k| k.unit)
}

/// A value in the lock's shape: numbers in the key's unit without prefix, a tolerance in `%` or in its own unit
/// without prefix (`±0.25 pF` as 2.5e-13 F), anything else as its text
fn fact_value(key: &str, v: &Variant, unparsed: bool) -> FactValue {
    let unit = unit_of(key);
    let text = || FactValue::Text {
        text: match v {
            Variant::Str(s) => s.clone(),
            Variant::Enum { selected, .. } => selected.clone(),
            other => other.to_string(),
        },
    };
    if unparsed {
        return text();
    }
    match v {
        Variant::Tolerance {
            min,
            min_unit,
            max,
            max_unit,
        } => {
            if min_unit.to_string() == "%" && max_unit.to_string() == "%" {
                return FactValue::Tolerance {
                    minus: -min.to_f64_lossy(),
                    plus: max.to_f64_lossy(),
                    unit: "%".to_owned(),
                };
            }
            match v.bounds_base_f64() {
                Some((lo, hi)) if min_unit.dim().is_some_and(|d| !d.is_one()) => {
                    FactValue::Tolerance {
                        minus: -lo,
                        plus: hi,
                        unit: min_unit.without_prefixes().to_string(),
                    }
                }
                _ => text(),
            }
        }
        _ if unit.is_empty() => text(),
        Variant::SIRange { .. } => match v.bounds_base_f64() {
            Some((min, max)) => FactValue::Range {
                min,
                max,
                unit: unit.to_owned(),
            },
            None => text(),
        },
        _ => match v.num_base_f64() {
            Some(value) => FactValue::Number {
                value,
                unit: unit.to_owned(),
            },
            None => text(),
        },
    }
}

/// A check's items as the lock's pairs (a maker named when several parts have the MPN), with its reviews
fn reviews_answer(check: FpCheck) -> ReviewsAnswer {
    let pairs = check
        .items
        .iter()
        .map(|item| {
            let several = check.items.iter().filter(|i| i.mpn == item.mpn).count() > 1;
            let mut covered_by: Vec<i64> = item.covered_by.iter().map(|&i| i64::from(i)).collect();
            covered_by.sort_unstable();
            let mut other_hashes: Vec<i64> =
                item.other_hashes.iter().map(|&i| i64::from(i)).collect();
            other_hashes.sort_unstable();
            LockedPair {
                hash: check.hash.clone(),
                mpn: item.mpn.clone(),
                footprint: String::new(),
                maker: item.maker.clone().filter(|_| several),
                status: match item.status {
                    FpCheckStatus::Covered => Coverage::Covered,
                    FpCheckStatus::Uncovered => Coverage::Uncovered,
                    FpCheckStatus::UnknownMpn => Coverage::UnknownMpn,
                },
                covered_by,
                other_hashes,
            }
        })
        .collect();
    ReviewsAnswer {
        pairs,
        reviews: check.reviews.into_iter().map(locked_review).collect(),
    }
}

fn locked_review(r: FpReview) -> LockedReview {
    LockedReview {
        id: i64::from(r.id),
        hash: r.hash,
        footprint: r.footprint,
        doc: r.source.doc,
        doc_rev: r.source.doc_rev,
        pdf_blake3: r.source.pdf_blake3,
        page: r.source.page,
        scale: r.source.scale,
        at: r.source.at.map(|p| Placement {
            x: p.x,
            y: p.y,
            rotation: p.rotation,
            mirror: p.mirror,
        }),
        reviewer: r.reviewer,
        reviewed_on: r.reviewed_on.to_string(),
        note: r.note,
        covers: r
            .covers
            .into_iter()
            .map(|c| match c {
                FpCover::Mpn { mpn, .. } => Cover::Mpn { mpn },
                FpCover::Family { scheme, segments } => Cover::Family { scheme, segments },
            })
            .collect(),
        retracted: r
            .retracted
            .map(|x| format!("{} by {}: {}", x.at.format("%Y-%m-%d"), x.by, x.why)),
    }
}
