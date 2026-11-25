//! What of a design a parts snapshot covers: every MPN of a fitted part in the base design and in each variant,
//! and each (board footprint hash, MPN) pair where the design has a board file.

use crate::mpn::mpns;
use erc_model::layout::FootprintId;
use erc_model::{Design, VariantId, View, fphash, natural_cmp};
use std::collections::{BTreeMap, BTreeSet};

/// The MPNs and footprint pairs of a design
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoardMpns {
    /// MPN, as the board writes it, to the references carrying it
    pub mpns: BTreeMap<String, Vec<String>>,
    /// Board footprint hash to its `Lib:Name` and the MPNs of the parts placed with it
    pub footprints: BTreeMap<String, HashedFootprint>,
}

/// A board footprint hash, by the parts placed with it
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HashedFootprint {
    /// The `Lib:Name` of the first footprint with this hash, by reference
    pub lib_id: String,
    pub mpns: BTreeSet<String>,
}

impl BoardMpns {
    /// Every `MpnN` field of the fitted parts of every board, in the base design and in each variant
    pub fn of(design: &Design) -> BoardMpns {
        let mut refs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut footprints: BTreeMap<String, HashedFootprint> = BTreeMap::new();
        for board in &design.boards {
            // The footprint hash of each part placed on the board file, in reference order
            let mut hashes = BTreeMap::new();
            if let Some(lay) = &board.layout {
                let mut placed: Vec<(usize, &erc_model::layout::Footprint)> =
                    lay.footprints.iter().enumerate().collect();
                placed.sort_by(|a, b| natural_cmp(&a.1.reference, &b.1.reference));
                for (i, f) in placed {
                    let Some(part) = f.part else { continue };
                    let hash = fphash::hash(&lay.local_footprint(FootprintId::from_index(i)));
                    hashes
                        .entry(part)
                        .or_insert_with(Vec::new)
                        .push((hash, f.lib_id.clone()));
                }
            }
            let variants = std::iter::once(None)
                .chain((0..board.variants.len()).map(|i| Some(VariantId::from_index(i))));
            for variant in variants {
                let view = View::new(board, variant);
                for p in view.fitted_components() {
                    let part_mpns = mpns(view, p);
                    for (_, mpn) in &part_mpns {
                        refs.entry((*mpn).to_owned())
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
            .map(|(mpn, set)| {
                let mut v: Vec<String> = set.into_iter().collect();
                v.sort_by(|a, b| natural_cmp(a, b));
                (mpn, v)
            })
            .collect();
        BoardMpns { mpns, footprints }
    }
}
