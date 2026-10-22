//! Import notes gathered while reading one file

use erc_model::{ImportNote, NoteKind};
use std::collections::BTreeSet;

/// The notes of one import; each note names its file
#[derive(Default)]
pub(crate) struct Notes {
    notes: Vec<ImportNote>,
    /// (file, kind, message) said once per file: "property X not kept" for every part would drown the rest
    once: BTreeSet<(String, String)>,
}

#[allow(dead_code)]
impl Notes {
    pub fn add(&mut self, file: &str, at: &str, kind: NoteKind, message: impl Into<String>) {
        self.notes.push(ImportNote {
            file: file.to_string(),
            at: at.to_string(),
            kind,
            message: message.into(),
        });
    }

    /// A note said only the first time for this file and message
    pub fn once(&mut self, file: &str, at: &str, kind: NoteKind, message: impl Into<String>) {
        let message = message.into();
        if self.once.insert((file.to_string(), message.clone())) {
            self.add(file, at, kind, message);
        }
    }

    pub fn unknown(&mut self, file: &str, at: &str, message: impl Into<String>) {
        self.add(file, at, NoteKind::Unknown, message);
    }

    pub fn invalid(&mut self, file: &str, at: &str, message: impl Into<String>) {
        self.add(file, at, NoteKind::Invalid, message);
    }

    pub fn missing(&mut self, file: &str, at: &str, message: impl Into<String>) {
        self.add(file, at, NoteKind::Missing, message);
    }

    pub fn ignored(&mut self, file: &str, at: &str, message: impl Into<String>) {
        self.once(file, at, NoteKind::Ignored, message);
    }

    pub fn into_vec(mut self) -> Vec<ImportNote> {
        self.notes.sort();
        self.notes
    }
}
