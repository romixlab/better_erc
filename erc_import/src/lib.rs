//! Every importer of better_erc behind one [`Registry`]: KiCad (through [`erc_kicad`]), Altium (EDIF plus
//! WireList), OrCAD Capture (pstxnet, pstxprt, pstchip), pick-and-place CSV files from KiCad, Altium and Allegro,
//! and schematic PDFs (Altium Smart PDF).
//!
//! Each importer implements [`erc_model::Importer`] and fills the same [`erc_model::Design`]: every attribute a
//! [`erc_model::Fact`] with the file it came from, everything read past an [`erc_model::ImportNote`], no panic on
//! any input. The registry picks the importer for a [`Source`] by its main file's name and contents, so callers
//! (`berc`, the MCP server, a GUI) never name a tool.
//!
//! The dependency points this way: `erc_import` → `erc_kicad` → `erc_model`. The trait lives in `erc_model`, so
//! `erc_kicad` stands alone (a KiCad-only tool needs nothing else) and registers here through its public
//! [`erc_kicad::importers`]. Each importer is a cargo feature (`kicad`, `altium`, `orcad`, `pnp`, `pdf`, all on by
//! default), so a build that only reads KiCad, or a wasm GUI, leaves the rest out.

#[cfg(feature = "altium")]
pub mod altium;
#[cfg(any(feature = "altium", feature = "orcad", feature = "pdf"))]
mod build;
mod notes;
#[cfg(feature = "orcad")]
pub mod orcad;
#[cfg(feature = "pdf")]
pub mod pdf;
#[cfg(feature = "pnp")]
pub mod pnp;
#[cfg(feature = "text")]
mod text;

use erc_model::{Confidence, Files, ImportError, ImportOptions, Imported, Importer, Source};

/// The importers a caller can choose from, in the order they were registered
pub struct Registry {
    importers: Vec<Box<dyn Importer>>,
}

/// A file of a set that an importer reads, and how sure it is
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The file, as [`Files::list`] names it
    pub main: String,
    /// The importer's [`Importer::name`]
    pub importer: &'static str,
    pub confidence: Confidence,
}

impl Default for Registry {
    /// Every importer this build has
    fn default() -> Registry {
        // Without any importer feature nothing is registered
        #[allow(unused_mut)]
        let mut r = Registry::empty();
        #[cfg(feature = "kicad")]
        for i in erc_kicad::importers() {
            r.register(i);
        }
        #[cfg(feature = "altium")]
        r.register(Box::new(altium::AltiumImporter));
        #[cfg(feature = "orcad")]
        r.register(Box::new(orcad::OrcadImporter));
        #[cfg(feature = "pnp")]
        r.register(Box::new(pnp::PnpImporter));
        #[cfg(feature = "pdf")]
        r.register(Box::new(pdf::PdfImporter));
        r
    }
}

impl Registry {
    /// No importers yet
    pub fn empty() -> Registry {
        Registry {
            importers: Vec::new(),
        }
    }

    /// Adds an importer; on equal confidence an earlier one wins
    pub fn register(&mut self, importer: Box<dyn Importer>) {
        self.importers.push(importer);
    }

    pub fn importers(&self) -> impl Iterator<Item = &dyn Importer> {
        self.importers.iter().map(|i| i.as_ref())
    }

    /// The importer by its name (`kicad`, `altium`)
    pub fn by_name(&self, name: &str) -> Option<&dyn Importer> {
        self.importers().find(|i| i.name() == name)
    }

    /// The importer most sure it reads `src`, with its confidence; `None` when none reads it
    pub fn pick(&self, src: &Source) -> Option<(&dyn Importer, Confidence)> {
        let mut best: Option<(&dyn Importer, Confidence)> = None;
        for i in self.importers() {
            let c = i.sniff(src);
            if c > Confidence::No && best.is_none_or(|(_, b)| c > b) {
                best = Some((i, c));
            }
        }
        best
    }

    /// Imports `src` with the importer [`Registry::pick`] chooses
    pub fn import(&self, src: &Source, opts: &ImportOptions) -> Result<Imported, ImportError> {
        match self.pick(src) {
            Some((i, _)) => i.import(src, opts),
            None => Err(ImportError::Unsupported(src.main.clone())),
        }
    }

    /// Every file of `files` some importer reads, the surest first (then in file order): what a caller offers
    /// when it is given a directory
    pub fn scan(&self, files: &dyn Files) -> Vec<Candidate> {
        let mut out = Vec::new();
        for f in files.list() {
            let src = Source::new(f.clone(), files);
            if let Some((i, c)) = self.pick(&src) {
                out.push(Candidate {
                    main: f,
                    importer: i.name(),
                    confidence: c,
                });
            }
        }
        out.sort_by_key(|c| std::cmp::Reverse(c.confidence));
        out
    }
}

/// The directory part of a path in a file set (`a/b/x.edf` → `a/b/`), empty at the top
#[cfg_attr(not(any(feature = "altium", feature = "orcad")), allow(dead_code))]
fn dir_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..=i],
        None => "",
    }
}

/// The file name of a path in a file set
#[cfg_attr(
    not(any(
        feature = "altium",
        feature = "orcad",
        feature = "pnp",
        feature = "pdf"
    )),
    allow(dead_code)
)]
fn file_name(path: &str) -> &str {
    &path[dir_of(path).len()..]
}
