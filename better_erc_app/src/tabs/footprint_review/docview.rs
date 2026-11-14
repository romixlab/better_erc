//! Each datasheet's view of a symbol, kept apart: the page, the zoom, the scale, the footprint's and the model's
//! placement, the ruler and the measurements made on one datasheet are its own. Opening another datasheet (or
//! another symbol) keeps the view left; opening it again restores that view, else its review line's, else a fresh
//! fit. The views are saved with the tab, so work not accepted yet survives a restart too.

use super::*;

/// How many views are kept, the oldest dropped first
const KEPT: usize = 200;

/// One datasheet's view of one symbol (or of a footprint opened alone), as it was left
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DocView {
    /// The symbol under review (`Lib:Name`), or the footprint opened alone
    pub symbol: String,
    pub datasheet: PathBuf,
    /// The review line made on this datasheet when the view was left: a line changed since (accepted elsewhere,
    /// written by `berc fp accept`) is newer than the view, and its view is restored instead
    pub line: Option<String>,
    /// 0-based
    pub page: usize,
    pub zoom: f32,
    pub pan: [f32; 2],
    pub place: Placement,
    pub scale: Option<Scale>,
    pub ruler: Option<[[f32; 2]; 2]>,
    pub model_views: modelviews::ModelViews,
    pub xray: bool,
    /// The review note being written and how many measurements went into it
    pub note: String,
    pub measure_count: usize,
}

impl FootprintReview {
    /// Whose view the datasheet open is: the symbol under review, else the footprint opened alone, and the
    /// datasheet
    pub(super) fn view_key(&self) -> Option<(String, PathBuf)> {
        let d = self.datasheet.clone()?;
        let who = match &self.rt.target {
            Some(t) => t.symbol.clone(),
            None => self.rt.sourced.as_ref()?.lib_id().to_string(),
        };
        Some((who, d))
    }

    /// The value of the review line made on the datasheet open, when there is one
    fn open_line(&mut self) -> Option<String> {
        let doc = self.open_doc()?;
        self.line_for(&doc).map(|l| l.value)
    }

    /// The view shown is left: kept as its datasheet's, for when it is opened again
    pub(super) fn keep_view(&mut self) {
        let Some((symbol, datasheet)) = self.rt.view_key.take() else {
            return;
        };
        let line = self.open_line();
        let v = DocView {
            symbol,
            datasheet,
            line,
            page: self.page,
            zoom: self.zoom,
            pan: self.pan,
            place: self.place.clone(),
            scale: self.scale.clone(),
            ruler: self.ruler,
            model_views: self.model_views.clone(),
            xray: self.xray,
            note: self.rt.note.clone(),
            measure_count: self.rt.measure_count,
        };
        self.views
            .retain(|w| !(w.symbol == v.symbol && w.datasheet == v.datasheet));
        self.views.push(v);
        if self.views.len() > KEPT {
            let extra = self.views.len() - KEPT;
            self.views.drain(..extra);
        }
    }

    /// The datasheet just opened (or the symbol just picked, its datasheet staying open) gets its own view: the
    /// one kept for it, else its review line's ([`Self::restore_recorded`]), else (`fresh`) a fresh fit, nothing
    /// of the last datasheet's placement, scale or measurements carried over
    pub(super) fn enter_view(&mut self, fresh: bool) {
        let key = self.view_key();
        let from_line = self.restore_recorded();
        let line = self.open_line();
        let kept = key.as_ref().and_then(|(s, d)| {
            self.views
                .iter()
                .find(|v| v.symbol == *s && v.datasheet == *d)
                .filter(|v| v.line == line)
                .cloned()
        });
        if let Some(v) = kept {
            self.page = v.page;
            self.zoom = v.zoom;
            self.pan = v.pan;
            self.place = v.place;
            self.scale = v.scale;
            self.ruler = v.ruler;
            self.model_views = v.model_views;
            self.xray = v.xray;
            self.rt.note = v.note;
            self.rt.measure_count = v.measure_count;
            self.rt.auto_page = false;
            self.rt.fit_page = false;
            self.rt.center_footprint = false;
        } else if fresh || from_line {
            if !from_line {
                self.place = Placement::default();
                self.rt.center_footprint = true;
            }
            // The model's places were on the last datasheet's page, at its scale
            self.model_views.place = Default::default();
            self.model_views.solid.place = None;
            self.rt.note.clear();
            self.rt.measure_count = 0;
        }
        self.rt.measured = None;
        self.rt.parked_ruler = None;
        self.rt.view_key = key;
    }
}
