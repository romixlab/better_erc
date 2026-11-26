//! Where a review was made, kept by the part source with the review ([`better_erc::parts::lock::LockedReview`]):
//! Accept records the datasheet (its document number and revision, the PDF's content hash from the document
//! library), the page, the scale and the footprint's placement, who and when. Picking a row whose reviews name
//! their datasheet opens that PDF and restores the review's view; a review of an earlier hash, or a PDF the library
//! doesn't have, is said in [`Runtime::record_note`] and the row's list of reviews.

use super::docindex::{self, IndexedDoc};
use super::rows::{Recorded, review_source};
use super::*;
use better_erc::parts::NewReview;
use better_erc::parts::lock::{LockedReview, Placement as LockPlacement};

/// The library's index in place of the settings' docs command (tests)
pub(super) type Indexer = Arc<dyn Fn() -> Result<Vec<IndexedDoc>, String> + Send + Sync>;

/// Finds a library file (relative to the library's root) on disk, in place of the settings' open command (tests)
pub(super) type Locator = Arc<dyn Fn(&Path) -> Result<Option<PathBuf>, String> + Send + Sync>;

/// One of the row's MPNs in Accept's question: whether the review covers it, and how
#[derive(Clone, Debug, PartialEq)]
pub(super) struct MpnChoice {
    pub mpn: String,
    pub ticked: bool,
    /// Found in the open PDF's text
    pub in_pdf: bool,
    /// The family its MPN decodes into, for the person; none: it is covered alone
    pub family: Option<String>,
    /// Covered by its family (every part of it), not alone
    pub by_family: bool,
}

/// Which MPNs the ticked choices cover, alone or by family
pub(super) fn covers_of(choices: &[MpnChoice]) -> better_erc::fp::Covers {
    let mut c = better_erc::fp::Covers::default();
    for m in choices.iter().filter(|m| m.ticked) {
        if m.by_family && m.family.is_some() {
            c.family_of.push(m.mpn.clone());
        } else {
            c.alone.push(m.mpn.clone());
        }
    }
    c
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

    /// The row under review, when there is one
    pub(super) fn target_row(&self) -> Option<&rows::Row> {
        let t = self.rt.target.as_ref()?;
        let i = self.symbol_row(&t.key)?;
        self.rt.symbols.as_ref()?.rows.get(i)
    }

    /// The reviews of the part source that name the row under review; none for a preview
    pub(super) fn recorded(&self) -> Vec<Recorded> {
        match (self.target_row(), &self.rt.symbols) {
            (Some(r), Some(l)) => r.recorded(&l.reviews),
            _ => Vec::new(),
        }
    }

    /// The review made on `doc`: the one with its content hash, else the one with its document number; a review of
    /// the hash now before one of an earlier hash
    pub(super) fn review_on(&self, doc: &IndexedDoc) -> Option<Recorded> {
        let mut rec = self.recorded();
        rec.sort_by_key(|r| (!r.current, -r.review.id));
        let by_pdf = rec
            .iter()
            .find(|r| docindex::rank(doc, &r.review) == Some(0));
        by_pdf
            .or_else(|| {
                rec.iter()
                    .find(|r| docindex::rank(doc, &r.review).is_some())
            })
            .cloned()
    }

    /// A row just picked whose reviews name their datasheets: the first such PDF the document library has opened,
    /// which restores its view ([`Self::restore_recorded`]). `false` when none was opened, said in the record note
    /// when the reviews name one; the usual datasheet search then goes on.
    pub(super) fn open_recorded(&mut self, ctx: &egui::Context) -> bool {
        let mut reviews: Vec<Recorded> = self
            .recorded()
            .into_iter()
            .filter(|r| r.review.pdf_blake3.is_some() || r.review.doc.is_some())
            .collect();
        if reviews.is_empty() {
            return false;
        }
        reviews.sort_by_key(|r| (!r.current, -r.review.id));
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
        for r in &reviews {
            for d in docindex::candidates(&index, &r.review) {
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
        let names: Vec<String> = reviews.iter().map(|r| review_source(&r.review)).collect();
        self.rt.record_note = Some(format!(
            "reviewed against {}: not found in the document library; open that PDF to see the review's view",
            names.join("; ")
        ));
        false
    }

    /// A datasheet just opened: the view of the review made on it restored (page, scale, placement), said in the
    /// record note when the review is of an earlier hash; or why there is none to restore. `true` when a review's
    /// view was restored
    pub(super) fn restore_recorded(&mut self) -> bool {
        let rec = self.recorded();
        let sourced: Vec<String> = rec
            .iter()
            .filter(|r| r.review.pdf_blake3.is_some() || r.review.doc.is_some())
            .map(|r| review_source(&r.review))
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
        let Some(r) = self.review_on(&doc) else {
            if self.rt.record_note.is_none() {
                self.rt.record_note = Some(format!(
                    "this datasheet is none of those the reviews were made against ({})",
                    sourced.join("; ")
                ));
            }
            return false;
        };
        let review = &r.review;
        if let Some(p) = review.page {
            self.page = (p as usize).saturating_sub(1);
            self.rt.auto_page = false;
        }
        if let Some(mm_per_point) = review.scale {
            self.scale = Some(Scale {
                mm_per_point,
                page: self.page,
                mm: 0.0,
                note: None,
            });
        }
        if let Some(a) = &review.at {
            self.place = Placement {
                origin: [a.x as f32, a.y as f32],
                rotation: a.rotation as f32,
                mirror: a.mirror,
            };
            self.rt.center_footprint = false;
        }
        let mut note = format!(
            "the view the review was made in: {}, by {} on {}",
            review_source(review),
            review.reviewer,
            review.reviewed_on
        );
        if !r.current {
            let now = self.rt.sourced.as_ref().map_or("", |s| s.hash.as_str());
            note += &format!(
                ". It was made for {}, the footprint is {now} now: review it again",
                review.hash
            );
        }
        self.rt.record_note = Some(note);
        true
    }

    /// Who reviews: the setting, else the login name
    pub(super) fn reviewer(&self) -> Option<String> {
        Some(self.rt.settings.reviewer.trim().to_string())
            .filter(|r| !r.is_empty())
            .or_else(|| std::env::var("USER").ok())
            .filter(|r| !r.trim().is_empty())
    }

    /// The review Accept records for what is shown, without its hash, footprint and covers (the accept sets
    /// those): the datasheet as the document library knows it, the page, the scale, the footprint's placement,
    /// the review note, who and today
    pub(super) fn new_review(&mut self) -> NewReview {
        let doc = self.open_doc();
        NewReview {
            doc: doc
                .as_ref()
                .and_then(|d| d.doc.clone())
                .filter(|d| !d.is_empty()),
            doc_rev: doc
                .as_ref()
                .and_then(|d| d.rev.as_deref())
                .filter(|r| !r.is_empty())
                .map(|r| {
                    if r.starts_with("rev") {
                        r.to_owned()
                    } else {
                        format!("rev{r}")
                    }
                }),
            pdf_blake3: doc.as_ref().and_then(IndexedDoc::blake3),
            page: self.rt.pdf.as_ref().map(|_| self.page as u32 + 1),
            scale: self.scale.as_ref().map(|s| s.mm_per_point),
            at: Some(LockPlacement {
                x: f64::from(self.place.origin[0]),
                y: f64::from(self.place.origin[1]),
                rotation: f64::from(self.place.rotation),
                mirror: self.place.mirror,
            }),
            reviewer: self.reviewer().unwrap_or_default(),
            reviewed_on: better_erc::fp::today(),
            note: self.rt.note.trim().to_owned(),
            ..NewReview::default()
        }
    }

    /// The row's MPNs for Accept's question, each by its family where its MPN decodes into one. Ticked when a
    /// review of the hash now made on this datasheet covers them, or the open PDF's text has them and no review of
    /// the hash now covers them on another datasheet (a series document names its siblings: accepting on it never
    /// claims them unless ticked by hand); none so: every one no review of the hash now covers. Another maker's MPN
    /// with its own datasheet is unticked by hand and reviewed on that one.
    pub(super) fn mpn_choices(&mut self) -> Vec<MpnChoice> {
        let Some(row) = self.target_row().cloned() else {
            return Vec::new();
        };
        let doc = self.open_doc();
        let current: Vec<Recorded> = self.recorded().into_iter().filter(|r| r.current).collect();
        let here = |m: &String| {
            current
                .iter()
                .any(|r| r.mpns.contains(m) && doc.as_ref().is_some_and(|d| made_on(&r.review, d)))
        };
        let elsewhere = |m: &String| current.iter().any(|r| r.mpns.contains(m)) && !here(m);
        let mut choices: Vec<MpnChoice> = row
            .mpns()
            .iter()
            .map(|mpn| {
                let in_pdf = self
                    .rt
                    .pdf
                    .as_ref()
                    .is_some_and(|p| !p.find_pages(std::slice::from_ref(mpn)).is_empty());
                let family = row.family_of(mpn).map(|c| c.describe());
                MpnChoice {
                    mpn: mpn.clone(),
                    ticked: here(mpn) || (in_pdf && !elsewhere(mpn)),
                    in_pdf,
                    by_family: family.is_some(),
                    family,
                }
            })
            .collect();
        if choices.iter().all(|c| !c.ticked) {
            for c in &mut choices {
                c.ticked = !current.iter().any(|r| r.mpns.contains(&c.mpn));
            }
        }
        choices
    }
}

/// What a review says of the row, for the person: of its hash now, of an earlier one, or retracted
pub(super) fn review_word(r: &Recorded) -> (&'static str, ve_theme::Tone) {
    if r.review.retracted.is_some() {
        ("retracted", ve_theme::Tone::Off)
    } else if r.current {
        ("ok", ve_theme::Tone::Good)
    } else {
        (
            "earlier hash: the footprint changed since",
            ve_theme::Tone::Crit,
        )
    }
}

/// Whether `review` was made on the document `doc`
pub(super) fn made_on(review: &LockedReview, doc: &IndexedDoc) -> bool {
    docindex::rank(doc, review).is_some()
}
