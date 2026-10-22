//! A check run on a project path, the way `berc check`, the MCP server and the GUI start one: open the path through
//! the importer registry ([`erc_import`]), run the engine, compare with a baseline report, and decide the exit code
//! as data ([`Verdict`]). Nothing here prints; every failure is a [`CheckError`].

use crate::ctx::{Availability, Config, Ctx};
use crate::engine::{RunError, RunOptions, run_imported};
use crate::finding::BaselineState;
use crate::meta::{Need, Severity};
use crate::report::{Report, SCHEMA_VERSION};
use erc_import::Registry;
use erc_model::{
    Confidence, DirFiles, Files, ImportError, ImportOptions, Imported, MemFiles, Source,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// A design opened from a path: which file of which directory, read by which importer
pub struct Project {
    /// The directory the design's files are relative to
    pub dir: PathBuf,
    /// The main file in `dir` (`board.kicad_pro`)
    pub main: String,
    /// The importer's name (`kicad`, `altium`)
    pub importer: &'static str,
    pub imported: Imported,
}

/// What a check run does besides the engine's [`RunOptions`]
#[derive(Clone, Debug)]
pub struct CheckOptions {
    /// Variant, the checks to run (`only`), the project's overrides
    pub run: RunOptions,
    /// Needs that must be available on every board; a missing one makes the verdict [`Exit::Unavailable`]
    pub require: Vec<Need>,
    /// Findings of this severity or worse fail the run (with a baseline: only the new ones)
    pub fail_on: Severity,
    /// A previous report to compare with ([`read_baseline`])
    pub baseline: Option<Report>,
}

impl Default for CheckOptions {
    fn default() -> CheckOptions {
        CheckOptions {
            run: RunOptions::default(),
            require: Vec::new(),
            fail_on: Severity::Error,
            baseline: None,
        }
    }
}

/// What a check run produced: the report (compared with the baseline when there was one) and the verdict
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CheckRun {
    pub dir: PathBuf,
    pub main: String,
    pub importer: String,
    pub report: Report,
    pub verdict: Verdict,
}

/// The outcome of a run as a CI job sees it
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub exit: Exit,
    /// The `fail_on` the run had
    pub fail_on: Severity,
    /// Findings at or above `fail_on` (with a baseline: the new ones only)
    pub failing: usize,
    /// Required needs that a board lacks
    pub unavailable: Vec<Unavailable>,
}

/// The exit code of a run. A run that could not start ([`CheckError`]) is [`CheckError::EXIT_CODE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Exit {
    /// 0: nothing at or above `fail_on`, every required need there
    Clean,
    /// 1: findings at or above `fail_on`
    Findings,
    /// 3: a required need is not available (before findings: the run did not check what it was asked to)
    Unavailable,
}

impl Exit {
    pub fn code(self) -> u8 {
        match self {
            Exit::Clean => 0,
            Exit::Findings => 1,
            Exit::Unavailable => 3,
        }
    }
}

/// A required need a board does not have
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unavailable {
    /// [`Need::slug`]
    pub need: String,
    pub board: String,
    /// Why, and what would provide it
    pub why: String,
}

/// Where a check run is, for a progress display
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Reading the design (the main file)
    Importing(String),
    /// Running the checks
    Checking,
}

/// Why a check run could not start: a usage or import error, exit code 2
#[derive(Debug)]
pub enum CheckError {
    /// The path does not exist
    NotFound(PathBuf),
    /// No file in the directory is a design an importer reads
    NoDesign(PathBuf),
    /// Several files in the directory could each be the design
    Ambiguous {
        dir: PathBuf,
        candidates: Vec<String>,
    },
    /// The importer failed on the design
    Import { file: PathBuf, error: ImportError },
    /// The run options do not fit the design (an unknown variant or check)
    Run(RunError),
    /// The baseline report cannot be read
    Baseline { file: PathBuf, message: String },
}

impl CheckError {
    pub const EXIT_CODE: u8 = 2;
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::NotFound(p) => write!(f, "{}: no such file or directory", p.display()),
            CheckError::NoDesign(p) => write!(
                f,
                "{}: no design here (a .kicad_pro, .kicad_sch, KiCad netlist, Altium EDIF, OrCAD pstxnet or PnP file)",
                p.display()
            ),
            CheckError::Ambiguous { dir, candidates } => write!(
                f,
                "{}: several files could be the design, name one: {}",
                dir.display(),
                candidates.join(", ")
            ),
            CheckError::Import { file, error } => match error {
                // The error names the file relative to the project directory; say where that is
                ImportError::Missing(p) => {
                    write!(f, "{}: no such file", file.with_file_name(p).display())
                }
                _ => write!(f, "{} ({})", error, file.display()),
            },
            CheckError::Run(e) => e.fmt(f),
            CheckError::Baseline { file, message } => {
                write!(f, "baseline {}: {message}", file.display())
            }
        }
    }
}

impl std::error::Error for CheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CheckError::Import { error, .. } => Some(error),
            CheckError::Run(e) => Some(e),
            _ => None,
        }
    }
}

impl From<RunError> for CheckError {
    fn from(e: RunError) -> CheckError {
        CheckError::Run(e)
    }
}

impl Project {
    /// Opens a design: a file (any file an importer reads), or a directory with one design in it (its
    /// `.kicad_pro` when there is exactly one, else the one file an importer is surest about)
    pub fn open(path: &Path) -> Result<Project, CheckError> {
        Project::open_with(path, &mut |_| {})
    }

    /// [`Project::open`], telling `progress` when the import starts
    pub fn open_with(path: &Path, progress: &mut dyn FnMut(Stage)) -> Result<Project, CheckError> {
        let registry = Registry::default();
        let meta = std::fs::metadata(path).map_err(|_| CheckError::NotFound(path.to_path_buf()))?;
        let (dir, main) = if meta.is_dir() {
            let main = pick_main(&registry, path)?;
            (path.to_path_buf(), main)
        } else {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| CheckError::NotFound(path.to_path_buf()))?
                .to_string();
            let dir = match path.parent() {
                Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
                _ => PathBuf::from("."),
            };
            (dir, name)
        };
        let files = DirFiles(dir.clone());
        Project::import(&registry, dir, main, &files, progress)
    }

    /// Opens a design whose files were picked one by one, from anywhere (a GUI's file dialogs): `main` (an Altium
    /// EDIF, an OrCAD pstxnet) with `others` beside it (the WireList, pstxprt, pstchip), each by its file name, as
    /// if they were in `main`'s directory
    pub fn open_files(main: &Path, others: &[PathBuf]) -> Result<Project, CheckError> {
        let mut files = MemFiles::default();
        let mut name_of = |p: &Path| -> Result<String, CheckError> {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| CheckError::NotFound(p.to_path_buf()))?
                .to_string();
            let bytes = std::fs::read(p).map_err(|_| CheckError::NotFound(p.to_path_buf()))?;
            files.0.insert(name.clone(), bytes);
            Ok(name)
        };
        let main_name = name_of(main)?;
        for o in others {
            name_of(o)?;
        }
        let dir = match main.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        };
        Project::import(&Registry::default(), dir, main_name, &files, &mut |_| {})
    }

    fn import(
        registry: &Registry,
        dir: PathBuf,
        main: String,
        files: &dyn Files,
        progress: &mut dyn FnMut(Stage),
    ) -> Result<Project, CheckError> {
        let src = Source::new(main.clone(), files);
        let file = dir.join(&main);
        let Some((importer, _)) = registry.pick(&src) else {
            return Err(CheckError::Import {
                error: ImportError::Unsupported(main),
                file,
            });
        };
        progress(Stage::Importing(main.clone()));
        let imported = importer
            .import(&src, &ImportOptions::default())
            .map_err(|error| CheckError::Import { file, error })?;
        Ok(Project {
            dir,
            main,
            importer: importer.name(),
            imported,
        })
    }

    /// The variant names of every board, sorted, once each
    pub fn variants(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .imported
            .design
            .boards
            .iter()
            .flat_map(|b| b.variants.iter().map(|v| v.name.clone()))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// Runs the checks, compares with the baseline and decides the verdict
    pub fn check(&self, opts: &CheckOptions) -> Result<CheckRun, CheckError> {
        let mut report = run_imported(&self.imported, &opts.run)?;
        if let Some(b) = &opts.baseline {
            report.compare(b);
        }
        let mut unavailable = Vec::new();
        for need in &opts.require {
            for (bi, board) in self.imported.design.boards.iter().enumerate() {
                let config = Config::default();
                let variant = opts
                    .run
                    .variant
                    .as_ref()
                    .and_then(|v| board.variant_by_name(v));
                let cx = Ctx::new(
                    &self.imported.design,
                    erc_model::BoardId::from_index(bi),
                    variant,
                    &config,
                );
                if let Availability::Missing { why } = cx.availability(*need) {
                    unavailable.push(Unavailable {
                        need: need.slug(),
                        board: board.name.clone(),
                        why,
                    });
                }
            }
        }
        let verdict = verdict(&report, opts.fail_on, unavailable);
        Ok(CheckRun {
            dir: self.dir.clone(),
            main: self.main.clone(),
            importer: self.importer.to_string(),
            report,
            verdict,
        })
    }
}

/// An output format of a check run
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// [`Report::to_text`], coloured on request
    Text,
    /// [`Report::to_json`], schema [`SCHEMA_VERSION`]
    Json,
    /// [`Report::to_sarif`] with the project directory as the URI base
    Sarif,
    /// [`Report::to_markdown`], for a PR comment
    Md,
}

impl CheckRun {
    /// The report in `format`; `color` only changes text
    pub fn render(&self, format: Format, color: bool) -> String {
        match format {
            Format::Text if color => self.report.to_text_color(),
            Format::Text => self.report.to_text(),
            Format::Json => self.report.to_json() + "\n",
            Format::Sarif => self.report.to_sarif(&uri_base(&self.dir), &self.main) + "\n",
            Format::Md => self.report.to_markdown(),
        }
    }
}

/// The project directory as the prefix of SARIF URIs: a relative directory as it is (`hw/board/`, empty for
/// `.`), an absolute one relative to the current directory when it is inside it (CI runs from the repository
/// root), else a `file://` URI
pub fn uri_base(dir: &Path) -> String {
    let rel = if dir.is_absolute() {
        match std::env::current_dir()
            .ok()
            .and_then(|cwd| dir.strip_prefix(cwd).ok().map(Path::to_path_buf))
        {
            Some(r) => r,
            None => {
                let abs = dir.to_string_lossy().replace('\\', "/");
                let abs = abs.trim_end_matches('/');
                return format!("file://{}/", crate::report::sarif::uri_encode(abs));
            }
        }
    } else {
        dir.to_path_buf()
    };
    let parts: Vec<String> = rel
        .components()
        .filter(|c| !matches!(c, std::path::Component::CurDir))
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if parts.is_empty() {
        String::new()
    } else {
        format!("{}/", crate::report::sarif::uri_encode(&parts.join("/")))
    }
}

/// Opens `path` and runs the checks on it: [`Project::open_with`] and [`Project::check`] in one
pub fn check_path(
    path: &Path,
    opts: &CheckOptions,
    progress: &mut dyn FnMut(Stage),
) -> Result<CheckRun, CheckError> {
    let project = Project::open_with(path, progress)?;
    progress(Stage::Checking);
    project.check(opts)
}

/// Reads a previous JSON report as a baseline; a report of another schema version is refused
pub fn read_baseline(file: &Path) -> Result<Report, CheckError> {
    let err = |message: String| CheckError::Baseline {
        file: file.to_path_buf(),
        message,
    };
    let text = std::fs::read_to_string(file).map_err(|e| err(e.to_string()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| err(format!("not JSON: {e}")))?;
    match value.get("schema").and_then(|s| s.as_u64()) {
        Some(v) if v == u64::from(SCHEMA_VERSION) => {}
        Some(v) => {
            return Err(err(format!(
                "report schema {v}, this better_erc reads schema {SCHEMA_VERSION}"
            )));
        }
        None => return Err(err("not a better_erc report (no `schema`)".into())),
    }
    Report::from_json(&text).map_err(|e| err(e.to_string()))
}

/// The verdict on a report: a missing required need first, then findings at or above `fail_on` (only the new
/// ones when the report was compared with a baseline); candidates never count
pub fn verdict(report: &Report, fail_on: Severity, unavailable: Vec<Unavailable>) -> Verdict {
    let failing = report
        .findings
        .iter()
        .filter(|f| f.severity <= fail_on)
        .filter(|f| f.baseline != Some(BaselineState::Unchanged))
        .count();
    let exit = if !unavailable.is_empty() {
        Exit::Unavailable
    } else if failing > 0 {
        Exit::Findings
    } else {
        Exit::Clean
    };
    Verdict {
        exit,
        fail_on,
        failing,
        unavailable,
    }
}

/// The design file of a directory
fn pick_main(registry: &Registry, dir: &Path) -> Result<String, CheckError> {
    let candidates = registry.scan(&DirFiles(dir.to_path_buf()));
    let projects: Vec<&str> = candidates
        .iter()
        .filter(|c| c.main.ends_with(".kicad_pro"))
        .map(|c| c.main.as_str())
        .collect();
    if let [one] = projects[..] {
        return Ok(one.to_string());
    }
    let best = candidates.first().map_or(Confidence::No, |c| c.confidence);
    let top: Vec<String> = candidates
        .iter()
        .filter(|c| c.confidence == best)
        .map(|c| c.main.clone())
        .collect();
    match top.len() {
        0 => Err(CheckError::NoDesign(dir.to_path_buf())),
        1 => Ok(top.into_iter().next().expect("one")),
        _ => Err(CheckError::Ambiguous {
            dir: dir.to_path_buf(),
            candidates: top,
        }),
    }
}
