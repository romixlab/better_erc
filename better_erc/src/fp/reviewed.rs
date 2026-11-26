//! Which footprint a board footprint's review is of: the library footprint the board names, found through the
//! footprint libraries, else the board copy as placed. A board copy that differs from its library footprint is
//! not part of what is reviewed: its differences ([`fphash::differences`]) are reported apart, so an override on
//! the board is seen and judged, never signed.

use super::check::{field_of, is_lib_name};
use super::outline::Margins;
use super::{FootprintLibraries, SymbolLibraries};
use erc_model::fphash::{self, Differences};
use erc_model::layout::{FootprintId, Layout};
use erc_model::{Board, LocalFootprint};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// What a board footprint's review hash is taken of
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewedAs {
    /// The library footprint the board names
    Library,
    /// The board copy as placed: no library file was found for it, so the board's own edits are reviewed too
    Placed,
}

impl ReviewedAs {
    pub fn name(self) -> &'static str {
        match self {
            ReviewedAs::Library => "library",
            ReviewedAs::Placed => "as placed",
        }
    }
}

/// A board footprint resolved to the footprint its review is of
#[derive(Clone, Debug, PartialEq)]
pub struct Reviewed {
    /// The footprint reviewed, in its own frame
    pub footprint: LocalFootprint,
    /// Its review hash: the library footprint's, else the board copy's
    pub hash: String,
    pub reviewed_as: ReviewedAs,
    /// The `Lib:Name` looked up in the footprint libraries: the board's own, else (a board footprint without a
    /// library nickname) its symbol's `Footprint` field
    pub library_footprint: Option<String>,
    /// The library `.kicad_mod`, when found
    pub library_file: Option<PathBuf>,
    /// The board copy's own review hash
    pub board_hash: String,
    /// How the board copy differs from its library footprint; empty when reviewed as placed
    pub differences: Differences,
    /// Why no library footprint was found, when none was
    pub note: Option<String>,
}

impl Reviewed {
    /// The board copy differs from its library footprint in what a review covers
    pub fn board_differs(&self) -> bool {
        !self.differences.is_empty()
    }

    /// The board copy's differences for the person, one line each
    pub fn describe_differences(&self) -> Vec<String> {
        self.differences.describe("library", "board")
    }
}

/// The `Lib:Name` a board footprint is looked up by in the footprint libraries: its own when it names a library,
/// else its symbol's `Footprint` field
pub(super) fn library_footprint(
    board_lib_id: &str,
    symbol_footprint: Option<&str>,
) -> Option<String> {
    [Some(board_lib_id), symbol_footprint]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|id| is_lib_name(id))
        .map(String::from)
}

/// Footprint `i` of `lay` resolved through `fps`; `symbol_footprint` is its symbol's `Footprint` field, the name
/// looked up when the board copy has no library nickname
pub fn resolve(
    lay: &Layout,
    i: FootprintId,
    symbol_footprint: Option<&str>,
    fps: &mut FootprintLibraries,
) -> Reviewed {
    let copy = lay.local_footprint(i);
    let board_hash = fphash::hash(&copy);
    let lib_id = &lay.footprints[i.index()].lib_id;
    let library_footprint = library_footprint(lib_id, symbol_footprint);
    let found = match &library_footprint {
        None => Err(format!(
            "footprint {lib_id} names no library, and its symbol none either"
        )),
        Some(id) => fps.footprint(id).map_err(|e| e.to_string()),
    };
    match found {
        Ok(lib) => Reviewed {
            hash: fphash::hash(&lib.footprint),
            differences: fphash::differences(&lib.footprint, &copy),
            footprint: lib.footprint.clone(),
            reviewed_as: ReviewedAs::Library,
            library_footprint,
            library_file: Some(lib.path.clone()),
            board_hash,
            note: None,
        },
        Err(why) => Reviewed {
            footprint: copy,
            hash: board_hash.clone(),
            reviewed_as: ReviewedAs::Placed,
            library_footprint,
            library_file: None,
            board_hash,
            differences: Differences::default(),
            note: Some(why),
        },
    }
}

/// The symbol and footprint libraries a board's footprints are resolved through
pub struct Resolver<'a> {
    pub syms: &'a mut SymbolLibraries,
    pub fps: &'a mut FootprintLibraries,
}

impl Resolver<'_> {
    /// The `Footprint` field of the part placed as footprint `i`: the schematic part's, its symbol copy's, else
    /// its library symbol's
    pub fn symbol_footprint(&mut self, board: &Board, i: FootprintId) -> Option<String> {
        let lay = board.layout.as_ref()?;
        let part_id = lay.footprints[i.index()].part?;
        let part = board.part(part_id);
        let copy = part.symbol.map(|s| &board.symbols[s.index()]);
        let lib_fields = self.syms.fields(&part.lib_id).ok();
        part.field("Footprint")
            .or_else(|| copy.and_then(|s| field_of(&s.fields, "Footprint")))
            .or_else(|| lib_fields.as_deref().and_then(|f| field_of(f, "Footprint")))
            .map(String::from)
    }

    /// Footprint `i` of `board`'s layout resolved ([`resolve`])
    ///
    /// # Panics
    ///
    /// When `board` has no layout.
    pub fn resolve(&mut self, board: &Board, i: FootprintId) -> Reviewed {
        let lay = board
            .layout
            .as_ref()
            .expect("a board footprint is of a layout");
        let sym = self.symbol_footprint(board, i);
        resolve(lay, i, sym.as_deref(), self.fps)
    }
}

/// The board setup's mask and paste margins for pads that set none, the margins that apply to every footprint
/// of the board alike, for the person: `mask 0.05 mm, paste -0.02 mm`; `None` when the setup sets none
pub fn board_margins_note(m: &Margins) -> Option<String> {
    let mut parts = Vec::new();
    if m.mask != 0.0 {
        parts.push(format!("mask {} mm", m.mask));
    }
    if m.paste != 0.0 {
        parts.push(format!("paste {} mm", m.paste));
    }
    if m.paste_ratio != 0.0 {
        parts.push(format!("paste ratio {}", m.paste_ratio));
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}
