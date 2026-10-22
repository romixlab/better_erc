//! KiCad 8, 9 and 10 into the better_erc board model ([`erc_model`]).
//!
//! - [`sexpr`]: the s-expression reader every KiCad file goes through (zero-copy, iterative, never panics).
//! - [`sch`]: the schematic hierarchy with KiCad's connectivity rules.
//! - [`netlist`]: the KiCad netlist (`kicad-cli sch export netlist`), the oracle of the schematic importer.
//!
//! Every element a reader does not know becomes an [`erc_model::ImportNote`], never a panic.

pub mod netlist;
mod notes;
pub mod sch;
pub mod sexpr;

pub use netlist::NetlistImporter;
pub use sch::SchematicImporter;

/// Every KiCad importer, for a caller that picks one by [`erc_model::Importer::sniff`]
pub fn importers() -> Vec<Box<dyn erc_model::Importer>> {
    vec![Box::new(SchematicImporter), Box::new(NetlistImporter)]
}
