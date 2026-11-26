//! The symbol's datasheets by MPN (VIEW-26): each `Mpn`, `Mpn1`, ... field, the library document that covers it and
//! its review state. Which document covers an MPN is the document library's answer (a setting, `library_mpn`:
//! `tpm lib mpn --json`, from mx3's Datasheet Url facts and the library's index; a series or catalogue file covers
//! several MPNs), asked once per MPN on a worker thread. The review state is from the part source's reviews that name
//! the row ([`super::rows::Row::recorded`]): reviewed, stale, or not yet. An MPN the library has no document for gets the
//! library's fetch command (`library_fetch`: `tpm lib fetch --mpn`), after which it is asked about again.

use super::docindex::IndexedDoc;
use super::recorded::made_on;
use super::rows::{Recorded, review_source};
use super::*;
use std::collections::HashMap;

/// One document the library names for an MPN, or why it has none (`path` empty)
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct MpnDoc {
    pub mpn: String,
    /// The document's file, absolute; none: the library has no document for the MPN
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub maker: Option<String>,
    /// The content hash, blake3 in hex
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub doc: Option<String>,
    #[serde(default)]
    pub rev: Option<String>,
    /// Why there is no document
    #[serde(default)]
    pub why: Option<String>,
}

impl MpnDoc {
    /// As the library's index lists it, for matching with the reviews (content hash, document number)
    pub(super) fn indexed(&self) -> IndexedDoc {
        IndexedDoc {
            path: self.path.clone().unwrap_or_default(),
            hash: self.hash.clone().unwrap_or_default(),
            doc: self.doc.clone(),
            rev: self.rev.clone(),
            gone: false,
        }
    }

    /// Its document number and revision (`DS30154 rev16`), where the library knows them
    pub fn label(&self) -> Option<String> {
        self.indexed().doc_label()
    }
}

/// The library's MPN lookup in place of the settings' command (tests)
pub(super) type MpnLookup = Arc<dyn Fn(&[String]) -> Result<Vec<MpnDoc>, String> + Send + Sync>;

/// The lookup's answer, taken in on the UI thread
type LookupReply = Receiver<Result<Vec<MpnDoc>, String>>;

/// The library's fetch for one MPN in place of the settings' command (tests): what it said, or why it failed
pub(super) type MpnFetch = Arc<dyn Fn(&str) -> Result<String, String> + Send + Sync>;

/// The lookup's answer as its command prints it
pub fn parse_lookup(json: &str) -> Result<Vec<MpnDoc>, String> {
    serde_json::from_str(json).map_err(|e| format!("not a JSON list of MPN documents: {e}"))
}

/// The output of `cmd` with `args` added, or why it failed
fn run(cmd: &[String], args: &[String]) -> Result<String, String> {
    let (prog, fixed) = cmd.split_first().ok_or("no command")?;
    let out = std::process::Command::new(prog)
        .args(fixed)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("can't run {prog}: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        // The first line says what went wrong (clap's usage and tips follow it)
        let err = err.trim().lines().next().unwrap_or("").to_string();
        return Err(format!("{} failed: {err}", cmd.join(" ")));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Runs the library's MPN lookup (`cmd`, the MPNs added at the end)
pub fn run_lookup(cmd: &[String], mpns: &[String]) -> Result<Vec<MpnDoc>, String> {
    parse_lookup(&run(cmd, mpns)?).map_err(|e| format!("{}: {e}", cmd.join(" ")))
}

/// Runs the library's fetch (`cmd`, the MPN added at the end): its last line
pub fn run_fetch(cmd: &[String], mpn: &str) -> Result<String, String> {
    let out = run(cmd, &[mpn.to_string()])?;
    Ok(out.trim().lines().last().unwrap_or("").to_string())
}

/// What the reviews say of one MPN on one document
#[derive(Clone, Debug, PartialEq)]
pub enum MpnReview {
    /// No review covers the MPN
    NotYet,
    /// A review of the footprint's hash now covers it
    Reviewed { review: Box<Recorded>, here: bool },
    /// Only a review of an earlier hash covers it
    Stale { review: Box<Recorded>, here: bool },
}

impl MpnReview {
    /// The word for the person, and its tone
    pub fn word(&self) -> (&'static str, ve_theme::Tone) {
        use ve_theme::Tone;
        match self {
            MpnReview::NotYet => ("not yet", Tone::Warn),
            MpnReview::Reviewed { here: true, .. } => ("reviewed", Tone::Good),
            MpnReview::Reviewed { here: false, .. } => ("reviewed elsewhere", Tone::Good),
            MpnReview::Stale { .. } => ("stale", Tone::Crit),
        }
    }

    /// The covering review, said for the person
    pub fn detail(&self) -> String {
        match self {
            MpnReview::NotYet => "no review covers this MPN: accept against its datasheet".into(),
            MpnReview::Reviewed { review, here } | MpnReview::Stale { review, here } => {
                let mut s = review_source(&review.review);
                if !here {
                    s += " (another document than this one)";
                }
                if !review.current {
                    s += &format!(
                        ": made for {}, the footprint changed since",
                        review.review.hash
                    );
                }
                s
            }
        }
    }
}

/// The review state of `mpn` from the row's reviews `rec`, against `doc` where the library names one: a review
/// of the hash now before one of an earlier hash, on that document first, else on any other
pub fn review_state(rec: &[Recorded], mpn: &str, doc: Option<&MpnDoc>) -> MpnReview {
    let covering: Vec<&Recorded> = rec
        .iter()
        .filter(|r| r.review.retracted.is_none() && r.mpns.iter().any(|m| m == mpn))
        .collect();
    let indexed = doc.map(MpnDoc::indexed);
    let on_doc = |r: &&Recorded| indexed.as_ref().is_some_and(|d| made_on(&r.review, d));
    let best = |current: bool| {
        let of = covering.iter().copied().filter(|r| r.current == current);
        let mut of: Vec<&Recorded> = of.collect();
        of.sort_by_key(|r| -r.review.id);
        match of.iter().copied().find(on_doc) {
            Some(r) => Some((r, true)),
            // Elsewhere only when there is a document of the library to compare with
            None => of.first().map(|r| (*r, indexed.is_none())),
        }
    };
    match best(true) {
        Some((r, here)) => MpnReview::Reviewed {
            review: Box::new(r.clone()),
            here,
        },
        None => match best(false) {
            Some((r, here)) => MpnReview::Stale {
                review: Box::new(r.clone()),
                here,
            },
            None => MpnReview::NotYet,
        },
    }
}

/// One row of the table: an MPN, a document of it (none: why not, or still asked), its review state
#[derive(Clone, Debug, PartialEq)]
pub struct MpnRow {
    pub mpn: String,
    pub doc: Option<MpnDoc>,
    /// No document: why (the library's word, or the lookup's error); none while it is asked
    pub missing: Option<String>,
    pub review: MpnReview,
}

/// What is known of each MPN asked: its documents, or why the lookup failed
#[derive(Default)]
pub(super) struct MpnDocs {
    pub known: HashMap<String, Result<Vec<MpnDoc>, String>>,
    /// The lookup running: the MPNs asked and its answer
    pub asking: Option<(Vec<String>, LookupReply)>,
    /// The fetch running: the MPN and what it says
    pub fetching: Option<(String, Receiver<Result<String, String>>)>,
    /// The last fetch: the MPN and what it said
    pub fetched: Option<(String, Result<String, String>)>,
    pub lookup: Option<MpnLookup>,
    pub fetch: Option<MpnFetch>,
}

impl FootprintReview {
    /// The MPNs of the row under review; none for a preview
    pub(super) fn symbol_mpns(&self) -> Vec<String> {
        self.target_row()
            .map(|r| r.mpns().to_vec())
            .unwrap_or_default()
    }

    /// The lookup to use: the test's, else the settings' command when it is on PATH
    fn mpn_lookup(&self) -> Option<MpnLookup> {
        if let Some(f) = &self.rt.mpn_docs.lookup {
            return Some(f.clone());
        }
        let cmd = self.rt.settings.library_mpn.clone();
        if cmd.is_empty() || find_program(&cmd[0]).is_none() {
            return None;
        }
        Some(Arc::new(move |mpns: &[String]| run_lookup(&cmd, mpns)))
    }

    /// Why the lookup can't run, for the person; none when it can
    pub(super) fn mpn_lookup_blocker(&self) -> Option<String> {
        if self.rt.mpn_docs.lookup.is_some() {
            return None;
        }
        let cmd = &self.rt.settings.library_mpn;
        match cmd.first() {
            None => Some("no MPN lookup command is set (Settings: library MPN lookup)".into()),
            Some(p) if find_program(p).is_none() => {
                Some(format!("{p} is not on PATH (Settings: library MPN lookup)"))
            }
            _ => None,
        }
    }

    /// The fetch to use: the test's, else the settings' command when it is on PATH
    fn mpn_fetch(&self) -> Option<MpnFetch> {
        if let Some(f) = &self.rt.mpn_docs.fetch {
            return Some(f.clone());
        }
        let cmd = self.rt.settings.library_fetch.clone();
        if cmd.is_empty() || find_program(&cmd[0]).is_none() {
            return None;
        }
        Some(Arc::new(move |mpn: &str| run_fetch(&cmd, mpn)))
    }

    /// Why the fetch can't run for an MPN now, for the person; none when it can
    pub(super) fn mpn_fetch_blocker(&self) -> Option<String> {
        if let Some((m, _)) = &self.rt.mpn_docs.fetching {
            return Some(format!("fetching {m}…"));
        }
        if self.rt.mpn_docs.fetch.is_some() {
            return None;
        }
        match self.rt.settings.library_fetch.first() {
            None => Some("no fetch command is set (Settings: library fetch)".into()),
            Some(p) if find_program(p).is_none() => {
                Some(format!("{p} is not on PATH (Settings: library fetch)"))
            }
            _ => None,
        }
    }

    /// Takes in the lookup's and the fetch's answers, and asks about the symbol's MPNs not known yet
    pub(super) fn poll_mpn_docs(&mut self, ctx: &egui::Context) {
        let wake = self.threads_wake();
        let d = &mut self.rt.mpn_docs;
        if let Some((asked, rx)) = &d.asking
            && let Ok(r) = rx.try_recv()
        {
            for m in asked {
                let docs = match &r {
                    Ok(all) => Ok(all.iter().filter(|x| x.mpn == *m).cloned().collect()),
                    Err(e) => Err(e.clone()),
                };
                d.known.insert(m.clone(), docs);
            }
            d.asking = None;
            if !wake {
                ctx.request_repaint();
            }
        }
        if let Some((m, rx)) = &d.fetching
            && let Ok(r) = rx.try_recv()
        {
            // Asked again: the library may have it now
            d.known.remove(m);
            d.fetched = Some((m.clone(), r));
            d.fetching = None;
            if !wake {
                ctx.request_repaint();
            }
        }
        if self.rt.mpn_docs.asking.is_some() {
            return;
        }
        let want: Vec<String> = self
            .symbol_mpns()
            .into_iter()
            .filter(|m| !self.rt.mpn_docs.known.contains_key(m))
            .collect();
        if want.is_empty() {
            return;
        }
        let Some(lookup) = self.mpn_lookup() else {
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let wake = wake.then(|| ctx.clone());
        let asked = want.clone();
        std::thread::spawn(move || {
            let _ = tx.send(lookup(&asked));
            if let Some(ctx) = wake {
                ctx.request_repaint();
            }
        });
        self.rt.mpn_docs.asking = Some((want, rx));
    }

    /// Runs the library's fetch for `mpn` on a worker thread
    pub(super) fn fetch_mpn(&mut self, ctx: &egui::Context, mpn: &str) {
        let Some(fetch) = self.mpn_fetch() else {
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let wake = self.threads_wake().then(|| ctx.clone());
        let m = mpn.to_string();
        std::thread::spawn(move || {
            let _ = tx.send(fetch(&m));
            if let Some(ctx) = wake {
                ctx.request_repaint();
            }
        });
        self.rt.mpn_docs.fetching = Some((mpn.to_string(), rx));
        self.rt.mpn_docs.fetched = None;
        self.usage("mpn fetch");
    }

    /// The lookup's error for the symbol's MPNs, when it failed (then each row's missing document is that error)
    pub(super) fn mpn_lookup_error(&self) -> Option<String> {
        self.symbol_mpns()
            .iter()
            .find_map(|m| self.rt.mpn_docs.known.get(m)?.as_ref().err().cloned())
    }

    /// The table's rows: per MPN of the symbol, a row per document the library names, else one saying why none
    pub(super) fn mpn_rows(&self) -> Vec<MpnRow> {
        let rec = self.recorded();
        let mut out = Vec::new();
        for mpn in self.symbol_mpns() {
            let row = |doc: Option<MpnDoc>, missing: Option<String>| MpnRow {
                review: review_state(&rec, &mpn, doc.as_ref()),
                mpn: mpn.clone(),
                doc,
                missing,
            };
            match self.rt.mpn_docs.known.get(&mpn) {
                None => out.push(row(None, None)),
                Some(Err(e)) => out.push(row(None, Some(e.clone()))),
                Some(Ok(docs)) => {
                    let found: Vec<&MpnDoc> = docs.iter().filter(|d| d.path.is_some()).collect();
                    if found.is_empty() {
                        let why = docs
                            .iter()
                            .find_map(|d| d.why.clone())
                            .unwrap_or_else(|| "the library has no document for it".into());
                        out.push(row(None, Some(why)));
                    }
                    for d in found {
                        out.push(row(Some(d.clone()), None));
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use better_erc::parts::lock::LockedReview;

    fn recorded(
        id: i64,
        mpns: &[&str],
        current: bool,
        pdf: Option<&str>,
        doc: Option<&str>,
    ) -> Recorded {
        Recorded {
            review: LockedReview {
                id,
                hash: format!("fp1:{id:016}"),
                pdf_blake3: pdf.map(String::from),
                doc: doc.map(String::from),
                doc_rev: doc.map(|_| "rev2".to_owned()),
                page: Some(1),
                ..LockedReview::default()
            },
            mpns: mpns.iter().map(|m| (*m).to_owned()).collect(),
            current,
        }
    }

    const LOOKUP: &str = r#"[
      {"mpn": "BSS138W-7-F", "maker": "Diodes", "path": "/lib/datasheets/Diodes/BSS138W.pdf",
       "hash": "0123456789abcdef0123456789abcdef", "doc": "DS30154", "rev": "16"},
      {"mpn": "BSS138BKW", "path": null, "why": "not in the library: https://x/a.pdf"},
      {"mpn": "NOPE", "path": null, "why": "not in mx3"}
    ]"#;

    /// The lookup's JSON read, the missing ones with their reason; a broken answer said as such
    #[test]
    fn the_lookup_answer_is_read() {
        let docs = parse_lookup(LOOKUP).unwrap();
        assert_eq!(docs.len(), 3);
        assert_eq!(docs[0].maker.as_deref(), Some("Diodes"));
        assert_eq!(
            docs[0].indexed().blake3().as_deref(),
            Some("0123456789abcdef0123456789abcdef")
        );
        assert_eq!(docs[0].label().as_deref(), Some("DS30154 rev16"));
        assert!(docs[1].path.is_none());
        assert_eq!(docs[2].why.as_deref(), Some("not in mx3"));
        assert!(parse_lookup("usage: tpm lib ...").is_err());
    }

    /// An MPN's state: a review of the hash now on the library's document first, one elsewhere next, stale when
    /// only a review of an earlier hash covers it, not yet with none
    #[test]
    fn review_state_per_mpn() {
        let doc = parse_lookup(LOOKUP).unwrap().remove(0);
        let here = recorded(1, &["BSS138W-7-F"], true, Some("0123456789abcdef"), None);
        let other = recorded(2, &["BSS138W-7-F", "BSS138BKW"], false, None, Some("NX1"));
        let rec = vec![other.clone(), here.clone()];
        let s = review_state(&rec, "BSS138W-7-F", Some(&doc));
        assert_eq!(s.word().0, "reviewed");
        assert!(matches!(&s, MpnReview::Reviewed { review, here: true } if review.review.id == 1));
        // No document from the library: the review covering it
        let s = review_state(&rec, "BSS138BKW", None);
        assert_eq!(s.word().0, "stale");
        assert_eq!(
            s.detail(),
            "#2 NX1 rev2, page 1: made for fp1:0000000000000002, the footprint changed since"
        );
        // The library's document is another than the one the review was made on
        let rec1 = vec![other];
        let s = review_state(&rec1, "BSS138W-7-F", Some(&doc));
        assert_eq!(s.word().0, "stale");
        assert!(
            s.detail().contains("(another document than this one)"),
            "{}",
            s.detail()
        );
        assert_eq!(review_state(&rec1, "OTHER", None), MpnReview::NotYet);
        // Of the hash now, on another document than the library's
        let elsewhere = vec![recorded(3, &["ANY"], true, None, Some("NX1"))];
        assert_eq!(review_state(&elsewhere, "ANY", None).word().0, "reviewed");
        assert_eq!(
            review_state(&elsewhere, "ANY", Some(&doc)).word().0,
            "reviewed elsewhere"
        );
    }
}
