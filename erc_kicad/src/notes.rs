//! Import notes gathered while reading one file

use crate::sexpr::Sexp;
use erc_model::{ImportNote, NoteKind};

/// The notes of one file
pub(crate) struct Notes {
    file: String,
    notes: Vec<ImportNote>,
}

impl Notes {
    pub fn new(file: &str) -> Notes {
        Notes {
            file: file.to_string(),
            notes: Vec::new(),
        }
    }

    pub fn add(&mut self, at: &str, kind: NoteKind, message: String) {
        self.notes.push(ImportNote {
            file: self.file.clone(),
            at: at.to_string(),
            kind,
            message,
        });
    }

    /// An element this reader does not know, named by its head
    pub fn unknown(&mut self, at: &str, item: &Sexp) {
        let what = match item.head() {
            Some(h) => format!("({h} ...)"),
            None => item.text().map_or("()".into(), |t| format!("`{t}`")),
        };
        self.add(at, NoteKind::Unknown, format!("{what} not read"));
    }

    pub fn invalid(&mut self, at: &str, message: &str) {
        self.add(at, NoteKind::Invalid, message.to_string());
    }

    pub fn missing(&mut self, at: &str, message: &str) {
        self.add(at, NoteKind::Missing, message.to_string());
    }

    pub fn ignored(&mut self, at: &str, message: &str) {
        self.add(at, NoteKind::Ignored, message.to_string());
    }

    pub fn into_vec(self) -> Vec<ImportNote> {
        self.notes
    }
}
