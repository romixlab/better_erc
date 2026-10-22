//! The better_erc board model.
//!
//! A [`Design`] is one board or a system of boards. A [`Board`] holds arenas of sheets, library symbols, parts,
//! pin instances, nets, labels and no-connect flags, indexed by small integer ids ([`PartId`], [`PinId`],
//! [`NetId`], ...), with adjacency both ways, plus the project settings and the [`Layout`] when a board file was
//! read. Importers (KiCad, the KiCad netlist, later Altium and OrCAD) implement [`Importer`] and produce a
//! `Design`; checks and analyses read it.
//!
//! Every attribute is a [`Fact`] with its [`Provenance`] (a schematic file, the project file, a person, a
//! distributor, a derivation rule), a [`Rank`], a time and a note; [`Facts`] keeps one per source and resolves
//! to the highest rank, the newest on a tie, so a finding can say where each number came from and a correction
//! by a person survives the next import.
//!
//! [`dump::write`] renders a design as canonical text, one fact per line, sorted; [`dump::read`] reads it back.
//! It is the golden-file format of the importer tests. No I/O here beyond [`DirFiles`] (feature `fs`).

pub mod canon;
pub mod design;
pub mod dump;
pub mod fact;
pub mod import;
pub mod layout;
pub mod netcmp;
pub mod value;

pub use canon::{link_layout, natural_cmp};
pub use design::*;
pub use fact::{Fact, Facts, Provenance, Rank, Time};
#[cfg(feature = "fs")]
pub use import::DirFiles;
pub use import::{
    Confidence, Files, ImportError, ImportNote, ImportOptions, Imported, Importer, MemFiles,
    NoteKind, Source,
};
pub use layout::{Layout, Nm, Pt};
pub use value::{Quantity, Unit, Value, ValueWarning};
