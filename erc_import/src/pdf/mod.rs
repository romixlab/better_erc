//! Schematic PDFs. An Altium Smart PDF carries the netlist in its bookmarks and the part properties in popups
//! ([`smart`]): read into a board whose connectivity is [`PdfConfidence::Bookmarks`] and whose popup properties are
//! [`PdfConfidence::Position`]. A PDF without that netlist (an Allegro or a plain print) is not read yet: its
//! connectivity would have to be traced from the drawing.
//!
//! A PDF is untrusted input. The file is parsed by `hayro-syntax` (pure Rust, lazy, no panics by design) and every
//! walk over it is bounded ([`doc::Limits`]); scripts are read as text for their literals and never run.

mod doc;
mod js;
mod smart;

use crate::notes::Notes;
use doc::{Doc, Limits, OpenError};
use erc_model::*;

/// The schematic PDF importer
pub struct PdfImporter;

/// Bookmarks looked at when sniffing: a Smart PDF's first sheet has its sections long before this
const SNIFF_BOOKMARKS: usize = 20_000;

impl Importer for PdfImporter {
    fn name(&self) -> &'static str {
        "pdf"
    }

    fn sniff(&self, src: &Source) -> Confidence {
        let Some(bytes) = src.files.read(&src.main) else {
            return Confidence::No;
        };
        if !doc::is_pdf(&bytes) {
            return Confidence::No;
        }
        match Doc::open(bytes, Limits::default()) {
            Ok(d) if smart::looks_like(&d.outline(Some(SNIFF_BOOKMARKS))) => Confidence::Yes,
            // A PDF this importer cannot read (yet): offered, so that the error says why
            _ => Confidence::Maybe,
        }
    }

    fn import(&self, src: &Source, _opts: &ImportOptions) -> Result<Imported, ImportError> {
        let file = src.main.as_str();
        let bytes = src
            .files
            .read(file)
            .ok_or_else(|| ImportError::Missing(file.to_string()))?;
        let invalid = |message: String| ImportError::Invalid {
            file: file.to_string(),
            message,
        };
        let doc = Doc::open(bytes, Limits::default()).map_err(|e| {
            invalid(match e {
                OpenError::TooLarge { bytes, limit } => {
                    format!("{bytes} bytes, more than the {limit} a PDF may have here")
                }
                OpenError::NotPdf => "not a PDF".into(),
                OpenError::Encrypted => "encrypted with a password".into(),
                OpenError::Invalid => "not a readable PDF".into(),
            })
        })?;
        let outline = doc.outline(None);
        if !smart::looks_like(&outline) {
            return Err(invalid(
                "no schematic netlist in the bookmarks (an Altium Smart PDF has Components and Nets under each \
                 sheet); connectivity from the drawing is not read yet"
                    .into(),
            ));
        }
        let mut notes = Notes::default();
        let extras = smart::Extras::read(&doc, file, &mut notes);
        let (creator, _) = doc.creator();
        let board = smart::read(file, &outline, &extras, creator, &mut notes);
        Ok(Imported {
            design: Design {
                boards: vec![board],
                ..Design::default()
            },
            notes: notes.into_vec(),
        })
    }
}

/// The file name without directory and extension
fn file_stem(path: &str) -> &str {
    let name = crate::file_name(path);
    name.rsplit_once('.').map_or(name, |(s, _)| s)
}
