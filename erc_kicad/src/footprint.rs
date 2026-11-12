//! A KiCad library footprint (`.kicad_mod`, KiCad 8, 9 and 10) into the footprint and pad types the board
//! reader fills ([`crate::pcb`]), in the footprint's own frame: a [`LocalFootprint`], the same a board copy
//! becomes with [`erc_model::Layout::local_footprint`].

use crate::pcb::Reader;
use crate::sexpr;
use erc_model::{ImportError, ImportNote, LocalFootprint};

/// Reads a library footprint file; `file` names it in the notes
pub fn read(text: &str, file: &str) -> Result<(LocalFootprint, Vec<ImportNote>), ImportError> {
    let invalid = |message: String| ImportError::Invalid {
        file: file.to_string(),
        message,
    };
    let parsed = sexpr::parse(text).map_err(|e| invalid(e.to_string()))?;
    let root = match parsed.as_slice() {
        [r] if r.head() == Some("footprint") => r,
        [r] if r.head() == Some("module") => {
            return Err(invalid(
                "a KiCad 5 footprint `(module ...)`: save it again with KiCad 8 or later".into(),
            ));
        }
        _ => {
            return Err(invalid(
                "not a KiCad footprint: no `(footprint ...)`".into(),
            ));
        }
    };
    let mut r = Reader::new(file);
    r.footprint(root);
    let (mut lay, notes) = r.finish();
    let footprint = lay.footprints.pop().expect("one footprint read");
    Ok((
        LocalFootprint {
            footprint,
            pads: lay.pads,
        },
        notes,
    ))
}
