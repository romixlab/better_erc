use better_erc::analysis::{I2cBuses, PartClasses, Power};
use better_erc::model::{Board, Design};
use better_erc::{CheckOptions, CheckRun, Project};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Clone)]
pub struct Context {
    shared: Arc<RwLock<ContextShared>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(RwLock::new(Default::default())),
        }
    }

    pub fn blocking_read(&self) -> RwLockReadGuard<'_, ContextShared> {
        self.shared.blocking_read()
    }

    pub fn blocking_write(&mut self) -> RwLockWriteGuard<'_, ContextShared> {
        self.shared.blocking_write()
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

/// The designs loaded, each with its check report
#[derive(Default)]
pub struct ContextShared {
    pub designs: Vec<Loaded>,
}

impl ContextShared {
    /// Adds a design, replacing one loaded from the same file
    pub fn load(&mut self, loaded: Loaded) {
        match self.designs.iter_mut().find(|d| d.file == loaded.file) {
            Some(d) => *d = loaded,
            None => self.designs.push(loaded),
        }
    }

    /// Every board of every design with its analyses
    pub fn boards(&self) -> impl Iterator<Item = (&Board, &Analyses)> {
        self.designs
            .iter()
            .flat_map(|d| d.design.boards.iter().zip(&d.analyses))
    }
}

/// One design: what was read, the report of a check run on it, and the analyses the views show
pub struct Loaded {
    /// The main file
    pub file: PathBuf,
    /// The importer that read it (`kicad`, `altium`)
    pub importer: &'static str,
    pub design: Design,
    pub run: CheckRun,
    /// Per board, in the design's order
    pub analyses: Vec<Analyses>,
}

/// What the views show of a board besides the findings
pub struct Analyses {
    pub classes: PartClasses,
    pub i2c: I2cBuses,
    pub power: Power,
}

impl Loaded {
    /// Runs the checks on an opened design and computes its analyses
    pub fn new(project: Project) -> Result<Loaded, better_erc::CheckError> {
        let run = project.check(&CheckOptions::default())?;
        let analyses = project
            .imported
            .design
            .boards
            .iter()
            .map(|b| {
                let classes = PartClasses::of(b);
                let i2c = I2cBuses::of(b, &classes);
                let power = Power::of(b, &classes, &i2c);
                Analyses {
                    classes,
                    i2c,
                    power,
                }
            })
            .collect();
        Ok(Loaded {
            file: project.dir.join(&project.main),
            importer: project.importer,
            design: project.imported.design,
            run,
            analyses,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn root() -> &'static Path {
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
    }

    /// The import tab's two paths: any design through the registry, and an Altium EDIF with its WireList
    #[test]
    fn load_and_replace() {
        let mut s = ContextShared::default();
        let sources = root().join("test_schematics/sources");
        let open = || Project::open(&sources.join("i2c-1_no_pull_ups.kicad_sch")).unwrap();
        s.load(Loaded::new(open()).unwrap());
        s.load(Loaded::new(open()).unwrap());
        assert_eq!(s.designs.len(), 1, "the same file again replaces it");
        let d = &s.designs[0];
        assert_eq!(d.importer, "kicad");
        assert!(d.run.report.findings.iter().any(|f| f.check == "I2C-1"));
        let fixtures = root().join("erc_import/tests/fixtures");
        let altium = Project::open_files(
            &fixtures.join("netlist_altium_edif.edf"),
            &[fixtures.join("netlist_altium_wirelist.net")],
        )
        .unwrap();
        s.load(Loaded::new(altium).unwrap());
        assert_eq!(s.designs.len(), 2);
        assert_eq!(s.designs[1].importer, "altium");
        assert_eq!(s.boards().count(), 2);
    }
}
