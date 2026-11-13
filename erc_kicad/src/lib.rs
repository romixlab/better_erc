//! KiCad 8, 9 and 10 into the better_erc board model ([`erc_model`]).
//!
//! - [`project`]: a KiCad project as one [`erc_model::Importer`]: project file, schematic, board.
//! - [`sexpr`]: the s-expression reader every KiCad file goes through (zero-copy, iterative, never panics).
//! - [`sch`]: the schematic hierarchy with KiCad's connectivity rules.
//! - [`pcb`]: the board: footprints, pads, tracks, vias, zones, graphics, stackup.
//! - [`footprint`]: a library footprint (`.kicad_mod`) in its own frame, comparable with a board copy.
//! - [`libtable`]: the library tables (`sym-lib-table`, `fp-lib-table`, project and global, `${VAR}`), and
//!   `Lib:Name` to a library symbol's fields or a parsed library footprint.
//! - [`model3d`]: a footprint's `(model ...)` path resolved: the same `${VAR}` as [`libtable::KicadVars`], a
//!   `.wrl` falling back to `.step`/`.stp`.
//! - [`symlib`]: the fields of a symbol in a `.kicad_sym`, for the footprint review, and a property written back.
//! - [`edit`]: formatting-preserving edits: only the changed tokens are written, the rest stays byte for byte.
//! - [`pro`]: the project file: net classes, text variables, variants, ERC and DRC settings, bus aliases.
//! - [`netlist`]: the KiCad netlist (`kicad-cli sch export netlist`), the oracle of the schematic importer.
//! - [`theme`]: a KiCad colour theme (`colors/*.json`).
//!
//! Every element a reader does not know becomes an [`erc_model::ImportNote`], never a panic.
//!
//! Connectivity follows KiCad 10's behaviour: rule changes between versions are followed by
//! [`erc_model::ImportOptions::kicad_version`], bugs of older releases (kicad-cli 9.0.9's) are not copied.

pub mod edit;
pub mod footprint;
pub mod libtable;
pub mod model3d;
pub mod netclass;
pub mod netlist;
mod notes;
pub mod pcb;
pub mod pro;
pub mod project;
pub mod sch;
pub mod sexpr;
pub mod symlib;
pub mod theme;

pub use netlist::NetlistImporter;
pub use project::KicadImporter;

/// Every KiCad importer, for a caller that picks one by [`erc_model::Importer::sniff`]
pub fn importers() -> Vec<Box<dyn erc_model::Importer>> {
    vec![Box::new(KicadImporter), Box::new(NetlistImporter)]
}
