//! Where a review was made, kept in the symbol ([`better_erc::fp::record`]): Accept writes the datasheet's review
//! line (the PDF by the document library's content hash, its document number, the page, the scale, the
//! footprint's placement, who and when) for the MPNs the person ticks, those found in the PDF's text ticked
//! already; opening a reviewed symbol opens the PDF its review was made on and restores that view. A stale line, or
//! a PDF the library doesn't have, is said in [`Runtime::record_note`] and the symbol's list of review lines.

use super::docindex::{self, IndexedDoc};
use super::*;
use better_erc::fp::record::{self, At, CheckedLine, LineStatus, ReviewLine};
use better_erc::model::fphash;

/// The library's index in place of the settings' docs command (tests)
pub(super) type Indexer = Arc<dyn Fn() -> Result<Vec<IndexedDoc>, String> + Send + Sync>;

/// Finds a library file (relative to the library's root) on disk, in place of the settings' open command (tests)
pub(super) type Locator = Arc<dyn Fn(&Path) -> Result<Option<PathBuf>, String> + Send + Sync>;

/// One of the symbol's MPNs in Accept's question: whether the datasheet covers it
#[derive(Clone, Debug, PartialEq)]
pub(super) struct MpnChoice {
    pub mpn: String,
    pub ticked: bool,
    /// Found in the open PDF's text
    pub in_pdf: bool,
    /// Another datasheet's review line names it (that datasheet, for the person): not ticked by itself, as
    /// ticking it moves the MPN off that line
    pub elsewhere: Option<String>,
}

/// What a line says it was reviewed against, for the person: the document, else the PDF, else that it says none
pub(super) fn line_source(l: &ReviewLine) -> String {
    let what = l
        .doc
        .clone()
        .or_else(|| l.pdf.clone())
        .unwrap_or_else(|| "no source recorded".into());
    match l.page {
        Some(p) => format!("{what}, page {p}"),
        None => what,
    }
}

/// A line's status for the person
pub(super) fn line_status(s: LineStatus) -> (&'static str, Color32) {
    match s {
        LineStatus::Ok => ("ok", Color32::from_rgb(0, 170, 60)),
        LineStatus::Changed => (
            "stale: the footprint changed since",
            Color32::from_rgb(220, 50, 50),
        ),
        LineStatus::OtherVersion => (
            "made by another version of the review hash",
            Color32::from_rgb(230, 140, 0),
        ),
        LineStatus::Malformed => ("not a review hash", Color32::from_rgb(220, 50, 50)),
    }
}

impl FootprintReview {
    /// The document library's index, read once per session; empty without a docs command
    pub(super) fn doc_index(&mut self) -> Result<&[IndexedDoc], String> {
        if self.rt.doc_index.is_none() {
            let cmd = self.rt.settings.library_docs.clone();
            let index = match &self.rt.indexer {
                Some(f) => f(),
                None if cmd.is_empty() || find_program(&cmd[0]).is_none() => Ok(Vec::new()),
                None => docindex::run_index(&cmd),
            };
            self.rt.doc_index = Some(index);
        }
        match self.rt.doc_index.as_ref().expect("read above") {
            Ok(i) => Ok(i),
            Err(e) => Err(e.clone()),
        }
    }

    /// The datasheet open as the document library knows it: its index, else what the MPN lookup said of that file
    /// (a document fetched since the index was read)
    pub(super) fn open_doc(&mut self) -> Option<IndexedDoc> {
        let path = self.datasheet.clone()?;
        let indexed = self
            .doc_index()
            .ok()
            .and_then(|i| docindex::identify(i, &path).cloned());
        indexed.or_else(|| {
            self.rt
                .mpn_docs
                .known
                .values()
                .flatten()
                .flatten()
                .find(|d| d.path.as_deref() == Some(path.as_path()) && d.hash.is_some())
                .map(|d| d.indexed())
        })
    }

    /// The review lines of the symbol under review, each against the footprint shown; none for a preview
    pub(super) fn recorded(&self) -> record::Assessed {
        let (Some(s), Some(_)) = (&self.rt.sourced, &self.rt.target) else {
            return record::Assessed::default();
        };
        let compare = |stored: &str| fphash::compare(stored, &s.footprint);
        record::assess(&s.fields, Some(&compare))
    }

    /// The review line made on `doc`: the one with its content hash, else the one with its document number
    pub(super) fn line_for(&self, doc: &IndexedDoc) -> Option<CheckedLine> {
        let rec = self.recorded();
        let pdf = doc.pdf();
        let by_pdf = rec
            .lines
            .iter()
            .find(|l| l.line.pdf.is_some() && l.line.pdf == pdf);
        let probe = ReviewLine {
            pdf: pdf.clone(),
            doc: doc.doc_label(),
            ..ReviewLine::default()
        };
        by_pdf
            .or_else(|| rec.lines.iter().find(|l| l.line.same_document(&probe)))
            .cloned()
    }

    /// A symbol just picked whose review lines name their datasheets: the first such PDF the document library has
    /// opened, which restores its view ([`Self::restore_recorded`]). `false` when none was opened, said in the
    /// record note when the lines name one; the usual datasheet search then goes on.
    pub(super) fn open_recorded(&mut self, ctx: &egui::Context) -> bool {
        let lines: Vec<ReviewLine> = self
            .recorded()
            .lines
            .into_iter()
            .map(|l| l.line)
            .filter(|l| l.pdf.is_some() || l.doc.is_some())
            .collect();
        if lines.is_empty() {
            return false;
        }
        let index = match self.doc_index() {
            Ok(i) => i.to_vec(),
            Err(e) => {
                self.rt.record_note = Some(format!(
                    "the review recorded its datasheet, but the document library can't be read: {e}"
                ));
                return false;
            }
        };
        let cmd = self.rt.settings.library_name.clone();
        for line in &lines {
            for d in docindex::candidates(&index, line) {
                let found = match &self.rt.locator {
                    Some(f) => f(&d.path),
                    None if cmd.is_empty() || find_program(&cmd[0]).is_none() => Ok(None),
                    None => docindex::run_locate(&cmd, &d.path),
                };
                if let Ok(Some(path)) = found {
                    self.open_datasheet(ctx, &path);
                    self.usage("datasheet recorded");
                    return true;
                }
            }
        }
        let names: Vec<String> = lines.iter().map(line_source).collect();
        self.rt.record_note = Some(format!(
            "reviewed against {}: not found in the document library; open that PDF to see the review's view",
            names.join("; ")
        ));
        false
    }

    /// A datasheet just opened: the view of the review line made on it restored (page, scale, placement), said in
    /// the record note with a stale hash; or why there is none to restore. `true` when a line's view was restored
    pub(super) fn restore_recorded(&mut self) -> bool {
        let rec = self.recorded();
        let sourced: Vec<String> = rec
            .lines
            .iter()
            .filter(|l| l.line.has_source())
            .map(|l| line_source(&l.line))
            .collect();
        if sourced.is_empty() {
            return false;
        }
        let Some(doc) = self.open_doc() else {
            if self.rt.record_note.is_none() {
                self.rt.record_note = Some(format!(
                    "this datasheet is not in the document library, so it can't be matched with the review \
                     against {}",
                    sourced.join("; ")
                ));
            }
            return false;
        };
        let Some(l) = self.line_for(&doc) else {
            if self.rt.record_note.is_none() {
                self.rt.record_note = Some(format!(
                    "this datasheet is none of those the review was made against ({})",
                    sourced.join("; ")
                ));
            }
            return false;
        };
        let line = &l.line;
        if let Some(p) = line.page {
            self.page = (p as usize).saturating_sub(1);
            self.rt.auto_page = false;
        }
        if let Some(mm_per_point) = line.scale {
            self.scale = Some(Scale {
                mm_per_point,
                page: self.page,
                mm: 0.0,
                note: None,
            });
        }
        if let Some(a) = &line.at {
            self.place = Placement {
                origin: [a.x as f32, a.y as f32],
                rotation: a.rot as f32,
                mirror: a.mirror,
            };
            self.rt.center_footprint = false;
        }
        let mut note = format!("the view the review was made in: {}", line_source(line));
        match (&line.by, &line.on) {
            (Some(by), Some(on)) => note += &format!(", by {by} on {on}"),
            (Some(by), None) => note += &format!(", by {by}"),
            (None, Some(on)) => note += &format!(", on {on}"),
            (None, None) => {}
        }
        if l.status != LineStatus::Ok {
            let now = self.rt.sourced.as_ref().map_or("", |s| s.hash.as_str());
            note += &format!(
                ". {} (reviewed {}, now {now}): review it again",
                line_status(l.status).0,
                line.hash
            );
        }
        self.rt.record_note = Some(note);
        true
    }

    /// Who reviews: the setting, else the login name
    fn reviewer(&self) -> Option<String> {
        Some(self.rt.settings.reviewer.trim().to_string())
            .filter(|r| !r.is_empty())
            .or_else(|| std::env::var("USER").ok())
            .filter(|r| !r.trim().is_empty())
    }

    /// The review line Accept writes for what is shown, without its MPNs (the confirmation's choice) and its hash
    /// (the accepted one's): the datasheet as the document library knows it, the page, the scale, the footprint's
    /// placement, who and today
    pub(super) fn review_line(&mut self) -> ReviewLine {
        let doc = self.open_doc();
        ReviewLine {
            doc: doc.as_ref().and_then(IndexedDoc::doc_label),
            pdf: doc.as_ref().and_then(IndexedDoc::pdf),
            page: self.rt.pdf.as_ref().map(|_| self.page as u32 + 1),
            scale: self.scale.as_ref().map(|s| s.mm_per_point),
            at: Some(At {
                x: self.place.origin[0] as f64,
                y: self.place.origin[1] as f64,
                rot: self.place.rotation as f64,
                mirror: self.place.mirror,
            }),
            by: self.reviewer(),
            on: Some(record::today()),
            ..ReviewLine::default()
        }
    }

    /// The symbol's MPNs for Accept's question, ticked when the review line made on this datasheet already names
    /// them, or the open PDF's text has them and no other datasheet's line does (a series document names its
    /// siblings: accepting on it never takes them off their own datasheet's line unless ticked by hand)
    pub(super) fn mpn_choices(&mut self) -> Vec<MpnChoice> {
        let Some(s) = &self.rt.sourced else {
            return Vec::new();
        };
        let mpns = record::mpns(&s.fields);
        let this = self.open_doc().and_then(|d| self.line_for(&d));
        let here = this
            .as_ref()
            .map(|l| l.line.mpns.clone())
            .unwrap_or_default();
        let others: Vec<ReviewLine> = self
            .recorded()
            .lines
            .into_iter()
            .filter(|l| this.as_ref().is_none_or(|t| t.property != l.property))
            .map(|l| l.line)
            .collect();
        mpns.into_iter()
            .map(|mpn| {
                let in_pdf = self
                    .rt
                    .pdf
                    .as_ref()
                    .is_some_and(|p| !p.find_pages(std::slice::from_ref(&mpn)).is_empty());
                let elsewhere = (!here.contains(&mpn))
                    .then(|| others.iter().find(|l| l.mpns.contains(&mpn)))
                    .flatten()
                    .map(line_source);
                MpnChoice {
                    ticked: here.contains(&mpn) || (in_pdf && elsewhere.is_none()),
                    in_pdf,
                    elsewhere,
                    mpn,
                }
            })
            .collect()
    }

    /// The symbol under review holds `lines` now: its fields, its row's lines and status say so
    pub(super) fn reviewed(&mut self, lines: &[String]) {
        let Some(t) = &self.rt.target else { return };
        let symbol = t.symbol.clone();
        if let Some(s) = &mut self.rt.sourced {
            s.fields
                .retain(|(k, _)| record::property_index(k).is_none());
            for (i, l) in lines.iter().enumerate() {
                s.fields.push((record::property_name(i), l.clone()));
            }
        }
        let Some(fields) = self.rt.sourced.as_ref().map(|s| s.fields.clone()) else {
            return;
        };
        if let Some(row) = self
            .rt
            .symbols
            .as_mut()
            .and_then(|l| l.rows.iter_mut().find(|r| r.symbol == symbol))
        {
            // The row's footprint is the library's: what the board places may differ ([`TodoEntry::overridden`])
            let now = row.hash.clone();
            let compare = |stored: &str| match &now {
                Some(n) if n == stored => fphash::Compared::Same,
                _ if fphash::is_hash(stored) => fphash::Compared::Changed {
                    now: now.clone().unwrap_or_default(),
                },
                _ => fphash::Compared::Malformed,
            };
            row.reviews = record::assess(&fields, Some(&compare));
            row.stored = row.reviews.stored();
            row.status = match row.reviews.overall() {
                record::Overall::Ok => SymbolStatus::Ok,
                record::Overall::Unreviewed => SymbolStatus::Unreviewed,
                record::Overall::Partial => SymbolStatus::Partial,
                record::Overall::Changed => SymbolStatus::Changed,
                record::Overall::OtherVersion => SymbolStatus::OtherVersion,
                record::Overall::Malformed => SymbolStatus::Malformed,
            };
        }
    }
}
