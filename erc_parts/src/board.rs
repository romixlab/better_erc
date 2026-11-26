//! What of a design a parts snapshot covers: every MPN of a fitted part in the base design and in each variant,
//! and each (footprint hash, MPN) pair where the design has a board file, the hash of the footprint a review of
//! the board footprint is of (the caller's [`Reviewed`] says which: its library footprint where one is found).

use crate::lock::PartKey;
use crate::mpn::{mpn_entries, mpns};
use erc_model::layout::FootprintId;
use erc_model::{Board, Design, PartId, VariantId, View, fphash, natural_cmp};

/// The footprint a review of board footprint `i` of `board` is of: its review hash and `Lib:Name`
pub type Reviewed<'a> = dyn FnMut(&Board, FootprintId) -> (String, String) + 'a;

/// A board footprint reviewed as placed: the board copy's own hash and `Lib:Name`, for a design without footprint
/// libraries
///
/// # Panics
///
/// When `board` has no layout.
pub fn as_placed(board: &Board, i: FootprintId) -> (String, String) {
    let lay = board
        .layout
        .as_ref()
        .expect("a board footprint is of a layout");
    (
        fphash::hash(&lay.local_footprint(i)),
        lay.footprints[i.index()].lib_id.clone(),
    )
}
use std::collections::{BTreeMap, BTreeSet};

/// The MPNs and footprint pairs of a design
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoardMpns {
    /// MPN and the maker named beside it, as the board writes them, to the references carrying it
    pub mpns: BTreeMap<PartKey, Vec<String>>,
    /// Reviewed footprint hash to its `Lib:Name` and the MPNs of the parts placed with it
    pub footprints: BTreeMap<String, HashedFootprint>,
}

/// A reviewed footprint hash, by the parts placed with it
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HashedFootprint {
    /// The `Lib:Name` of the first footprint reviewed with this hash, by reference
    pub lib_id: String,
    pub mpns: BTreeSet<String>,
}

/// The MPNs of `part` in every view of `board` (the base design and each variant) it is fitted in, each once in
/// first-seen order; `None` when it is fitted in none
pub fn part_mpns(board: &Board, part: PartId) -> Option<Vec<String>> {
    let mut fitted = false;
    let mut out: Vec<String> = Vec::new();
    for view in views(board) {
        if !view.fitted(part) {
            continue;
        }
        fitted = true;
        for (_, m) in mpns(view, part) {
            if !out.iter().any(|o| o == m) {
                out.push(m.to_owned());
            }
        }
    }
    fitted.then_some(out)
}

/// The base design's view of `board`, then each variant's
fn views(board: &Board) -> impl Iterator<Item = View<'_>> {
    std::iter::once(None)
        .chain((0..board.variants.len()).map(|i| Some(VariantId::from_index(i))))
        .map(move |v| View::new(board, v))
}

impl BoardMpns {
    /// Every `MpnN` field (with its `MfgN`) of the fitted parts of every board, in the base design and in each variant, each placed
    /// footprint by what `reviewed` says its review is of
    pub fn of(design: &Design, reviewed: &mut Reviewed) -> BoardMpns {
        let mut refs: BTreeMap<PartKey, BTreeSet<String>> = BTreeMap::new();
        let mut footprints: BTreeMap<String, HashedFootprint> = BTreeMap::new();
        for board in &design.boards {
            // The reviewed footprint hash of each part placed on the board file, in reference order
            let mut hashes = BTreeMap::new();
            if let Some(lay) = &board.layout {
                let mut placed: Vec<(usize, &erc_model::layout::Footprint)> =
                    lay.footprints.iter().enumerate().collect();
                placed.sort_by(|a, b| natural_cmp(&a.1.reference, &b.1.reference));
                for (i, f) in placed {
                    let Some(part) = f.part else { continue };
                    hashes
                        .entry(part)
                        .or_insert_with(Vec::new)
                        .push(reviewed(board, FootprintId::from_index(i)));
                }
            }
            for view in views(board) {
                for p in view.fitted_components() {
                    let part_mpns = mpns(view, p);
                    for e in mpn_entries(view, p) {
                        refs.entry(e.key())
                            .or_default()
                            .insert(board.part(p).designator.clone());
                    }
                    if part_mpns.is_empty() {
                        continue;
                    }
                    for (hash, lib_id) in hashes.get(&p).into_iter().flatten() {
                        let e = footprints
                            .entry(hash.clone())
                            .or_insert_with(|| HashedFootprint {
                                lib_id: lib_id.clone(),
                                mpns: BTreeSet::new(),
                            });
                        e.mpns
                            .extend(part_mpns.iter().map(|(_, m)| (*m).to_owned()));
                    }
                }
            }
        }
        let mpns = refs
            .into_iter()
            .map(|(key, set)| {
                let mut v: Vec<String> = set.into_iter().collect();
                v.sort_by(|a, b| natural_cmp(a, b));
                (key, v)
            })
            .collect();
        BoardMpns { mpns, footprints }
    }
}
