//! KiCad 8, 9 and 10 into the better_erc board model ([`erc_model`]).
//!
//! - [`project`]: a KiCad project as one [`erc_model::Importer`]: project file, schematic, board.
//! - [`sexpr`]: the s-expression reader every KiCad file goes through (zero-copy, iterative, never panics).
//! - [`sch`]: the schematic hierarchy with KiCad's connectivity rules.
//! - [`pcb`]: the board: footprints, pads, tracks, vias, zones, graphics, stackup.
//! - [`pro`]: the project file: net classes, text variables, variants, ERC and DRC settings, bus aliases.
//! - [`netlist`]: the KiCad netlist (`kicad-cli sch export netlist`), the oracle of the schematic importer.
//!
//! Every element a reader does not know becomes an [`erc_model::ImportNote`], never a panic.
//!
//! Connectivity follows KiCad 10's behaviour: rule changes between versions are followed by
//! [`erc_model::ImportOptions::kicad_version`], bugs of older releases (kicad-cli 9.0.9's) are not copied.

pub mod netclass;
pub mod netlist;
mod notes;
pub mod pcb;
pub mod pro;
pub mod project;
pub mod sch;
pub mod sexpr;

pub use netlist::NetlistImporter;
pub use project::KicadImporter;

/// Every KiCad importer, for a caller that picks one by [`erc_model::Importer::sniff`]
pub fn importers() -> Vec<Box<dyn erc_model::Importer>> {
    vec![Box::new(KicadImporter), Box::new(NetlistImporter)]
}
