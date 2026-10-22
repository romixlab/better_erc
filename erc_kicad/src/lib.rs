//! KiCad 8, 9 and 10 into the better_erc board model ([`erc_model`]).
//!
//! - [`project`]: a KiCad project as one [`erc_model::Importer`]: project file, schematic, board.
//! - [`sexpr`]: the s-expression reader every KiCad file goes through (zero-copy, iterative, never panics).
//! - [`sch`]: the schematic hierarchy with KiCad's connectivity rules.
//! - [`pro`]: the project file: net classes, text variables, variants, ERC and DRC settings, bus aliases.
//! - [`netlist`]: the KiCad netlist (`kicad-cli sch export netlist`), the oracle of the schematic importer.
//!
//! Every element a reader does not know becomes an [`erc_model::ImportNote`], never a panic.

pub mod netlist;
mod notes;
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
