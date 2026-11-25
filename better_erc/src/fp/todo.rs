//! `berc fp todo`: what a board's footprints need reviewed, one entry per board footprint hash and family (or MPN
//! reviewed alone), in the order a review steps through them. One review of a footprint against a family's
//! datasheet covers every part of the family placed with it, on this board and every other.

use super::check::{Opened, field_of, is_lib_name};
use super::library::lib_allowed;
use super::pairs::{self, OtherReview, PairStatus};
use super::{FootprintLibraries, FpCheckError, LibrarySearch};
use erc_model::fphash;
use erc_model::layout::FootprintId;
use erc_parts::lock::{Cover, LockedReview};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Where a todo entry stands. Ordered as the list goes: what needs a review first, then what can't be reviewed yet,
/// then what is done.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TodoStatus {
    /// Reviewed under another hash: the footprint changed since
    Changed,
    /// Reviewed only with another version of the review hash
    OtherVersion,
    /// Never reviewed
    Uncovered,
    /// The parts snapshot lacks the pair: refresh it first
    NotInLock,
    /// The part source has no part of the MPN: a review needs the part there first
    UnknownMpn,
    /// The part has no MPN to review against
    NoMpn,
    /// A review of this hash covers it
    Covered,
}

impl TodoStatus {
    pub fn name(self) -> &'static str {
        match self {
            TodoStatus::Changed => "changed",
            TodoStatus::OtherVersion => "other version",
            TodoStatus::Uncovered => "uncovered",
            TodoStatus::NotInLock => "not in lock",
            TodoStatus::UnknownMpn => "unknown MPN",
            TodoStatus::NoMpn => "no MPN",
            TodoStatus::Covered => "ok",
        }
    }

    /// A review is the next step
    pub fn needs_review(self) -> bool {
        matches!(
            self,
            TodoStatus::Changed | TodoStatus::OtherVersion | TodoStatus::Uncovered
        )
    }

    fn of(p: PairStatus) -> TodoStatus {
        match p {
            PairStatus::Changed => TodoStatus::Changed,
            PairStatus::OtherVersion => TodoStatus::OtherVersion,
            PairStatus::NotInLock => TodoStatus::NotInLock,
            PairStatus::Uncovered => TodoStatus::Uncovered,
            PairStatus::UnknownMpn => TodoStatus::UnknownMpn,
            PairStatus::Covered => TodoStatus::Covered,
        }
    }
}

/// One board footprint hash and what one review of it covers
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TodoEntry {
    /// The board footprint's review hash
    pub hash: String,
    /// Its `Lib:Name` on the board, the first reference's
    pub footprint: String,
    pub status: TodoStatus,
    /// What a review of it covers: a family, else one MPN; `None` for [`TodoStatus::NoMpn`]
    pub cover: Option<Cover>,
    /// The board's MPNs it stands for, in board order
    pub mpns: Vec<String>,
    /// The board footprints with this hash and these MPNs, in natural order
    pub references: Vec<String>,
    /// The reviews of this hash that cover these MPNs (ids in [`FpTodo::reviews`])
    #[serde(default)]
    pub covered_by: Vec<i64>,
    /// Earlier reviews of other hashes that cover these MPNs: what the footprint was reviewed as
    #[serde(default)]
    pub other: Vec<OtherReview>,
    /// The first reference's schematic symbol (`Lib:Name`)
    pub symbol: Option<String>,
    /// The first reference's `Datasheet` field, what the footprint is reviewed against
    pub datasheet: Option<String>,
    /// The library `.kicad_mod` its `Lib:Name` names, when the footprint libraries give it
    pub library_file: Option<PathBuf>,
    /// That library footprint's review hash: another than [`hash`](Self::hash) when the board was not updated from
    /// the library
    pub library_hash: Option<String>,
    /// References placed with a footprint other than their symbol's own `Footprint` field (reference, the symbol's
    /// footprint): an override, deliberate or not, shown rather than silently resolved
    #[serde(default)]
    pub overridden: Vec<(String, String)>,
    /// A symbol's own library, when `allowed` named some and it is none of them
    #[serde(default)]
    pub symbol_library_disallowed: Option<String>,
    /// The footprint's library, when `allowed` named some and it is none of them
    #[serde(default)]
    pub footprint_library_disallowed: Option<String>,
}

impl TodoEntry {
    /// A review is the next step
    pub fn needs_review(&self) -> bool {
        self.status.needs_review()
    }

    /// Reviewed
    pub fn is_ok(&self) -> bool {
        self.status == TodoStatus::Covered
    }

    /// A reference is placed with a footprint other than what its symbol names
    pub fn is_overridden(&self) -> bool {
        !self.overridden.is_empty()
    }

    /// Nothing outside what is allowed
    pub fn is_allowed(&self) -> bool {
        self.symbol_library_disallowed.is_none() && self.footprint_library_disallowed.is_none()
    }

    /// The entry's MPNs and references for the person: `RC0402FR-0710KL, RC0402FR-071KL on R1 R2 R7`
    pub fn describe(&self) -> String {
        format!("{} on {}", self.mpns.join(", "), self.references.join(" "))
    }
}

/// What a board's footprints need reviewed
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FpTodo {
    /// The board file
    pub board: PathBuf,
    /// The parts snapshot read
    pub lock: PathBuf,
    /// By status ([`TodoStatus`]'s order), then footprint and cover
    pub entries: Vec<TodoEntry>,
    /// Every review an entry names (its `covered_by` and `other`), from the parts snapshot, by id
    #[serde(default)]
    pub reviews: Vec<LockedReview>,
    /// Every file this was made from, sorted: the project's files read (its schematics, its board, its
    /// `.kicad_pro`, its parts snapshot), the library tables looked at, the symbol libraries and the footprint files
    /// used or looked for in vain. When one of them changes the list is out of date (the review tab watches them)
    #[serde(default)]
    pub files: Vec<PathBuf>,
}

/// One board footprint as a todo entry takes it in
struct Placed<'a> {
    hash: String,
    lib_id: &'a str,
    reference: &'a str,
    symbol: &'a str,
    datasheet: Option<&'a str>,
    /// (reference, the symbol's own footprint), when they differ
    overridden: Option<(String, String)>,
    symbol_lib: Option<&'a str>,
    footprint_lib: Option<&'a str>,
}

/// One MPN of a board footprint, judged
struct Group {
    status: TodoStatus,
    cover: Option<Cover>,
    mpn: Option<String>,
    covered_by: Vec<i64>,
    other: Vec<OtherReview>,
}

/// `g` of footprint `f` into its entry of `entries`, a new one when there is none: one entry per hash, cover and
/// whether it is reviewed
fn add(entries: &mut Vec<TodoEntry>, f: &Placed, g: Group, fps: &mut FootprintLibraries) {
    let done = g.status == TodoStatus::Covered;
    let at = entries.iter().position(|e| {
        e.hash == f.hash && (e.status == TodoStatus::Covered) == done && e.cover == g.cover
    });
    let e = match at {
        Some(i) => &mut entries[i],
        None => {
            let found = if is_lib_name(f.lib_id) {
                fps.footprint(f.lib_id.trim()).ok()
            } else {
                None
            };
            entries.push(TodoEntry {
                hash: f.hash.clone(),
                footprint: f.lib_id.to_owned(),
                status: g.status,
                cover: g.cover.clone(),
                mpns: Vec::new(),
                references: Vec::new(),
                covered_by: Vec::new(),
                other: Vec::new(),
                symbol: Some(f.symbol.to_owned()),
                datasheet: f.datasheet.map(String::from),
                library_file: found.as_ref().map(|x| x.path.clone()),
                library_hash: found.as_ref().map(|x| fphash::hash(&x.footprint)),
                overridden: Vec::new(),
                symbol_library_disallowed: None,
                footprint_library_disallowed: None,
            });
            entries.last_mut().expect("just pushed")
        }
    };
    e.status = e.status.min(g.status);
    if let Some(m) = g.mpn
        && !e.mpns.contains(&m)
    {
        e.mpns.push(m);
    }
    if !e.references.iter().any(|r| r == f.reference) {
        e.references.push(f.reference.to_owned());
    }
    for id in g.covered_by {
        if !e.covered_by.contains(&id) {
            e.covered_by.push(id);
        }
    }
    for o in g.other {
        if !e.other.contains(&o) {
            e.other.push(o);
        }
    }
    if let Some(o) = &f.overridden
        && !e.overridden.contains(o)
    {
        e.overridden.push(o.clone());
    }
    if e.symbol_library_disallowed.is_none() {
        e.symbol_library_disallowed = f.symbol_lib.map(String::from);
    }
    if e.footprint_library_disallowed.is_none() {
        e.footprint_library_disallowed = f.footprint_lib.map(String::from);
    }
}

/// What the footprints of the KiCad project at `path` (a directory, its `.kicad_pro`, `.kicad_sch` or
/// `.kicad_pcb`) need reviewed, judged against its parts snapshot; the libraries found through `search` and the
/// project's tables
pub fn todo(path: &Path, search: &LibrarySearch) -> Result<FpTodo, FpCheckError> {
    let mut opened = Opened::open(path, search)?;
    let Opened {
        project,
        lock,
        syms,
        fps,
    } = &mut opened;
    let board = &project.imported.design.boards[0];
    let lay = board.layout.as_ref().expect("checked in open");
    let mut order: Vec<usize> = (0..lay.footprints.len()).collect();
    order.sort_by(|a, b| {
        erc_model::natural_cmp(&lay.footprints[*a].reference, &lay.footprints[*b].reference)
    });
    let mut entries: Vec<TodoEntry> = Vec::new();
    for i in order {
        let f = &lay.footprints[i];
        let Some(part_id) = f.part else { continue };
        let Some(mpns) = erc_parts::board::part_mpns(board, part_id) else {
            continue;
        };
        let part = board.part(part_id);
        let hash = fphash::hash(&lay.local_footprint(FootprintId::from_index(i)));
        let copy = part.symbol.map(|s| &board.symbols[s.index()]);
        let lib_fields = syms.fields(&part.lib_id).ok();
        let symbol_footprint = lib_fields
            .as_deref()
            .and_then(|fs| field_of(fs, "Footprint"))
            .or_else(|| copy.and_then(|s| field_of(&s.fields, "Footprint")));
        let overridden = symbol_footprint
            .filter(|sym| is_lib_name(f.lib_id.trim()) && sym.trim() != f.lib_id.trim())
            .map(|sym| (f.reference.clone(), sym.to_owned()));
        let symbol_lib = part
            .lib_id
            .split_once(':')
            .map(|(l, _)| l)
            .filter(|l| !lib_allowed(l, &search.allowed));
        let footprint_lib = f
            .lib_id
            .split_once(':')
            .map(|(l, _)| l)
            .filter(|l| !lib_allowed(l, &search.allowed));
        let datasheet = part
            .field("Datasheet")
            .map(str::trim)
            .filter(|d| !d.is_empty() && *d != "~");
        let placed = Placed {
            hash,
            lib_id: &f.lib_id,
            reference: &f.reference,
            symbol: &part.lib_id,
            datasheet,
            overridden,
            symbol_lib,
            footprint_lib,
        };
        if mpns.is_empty() {
            let g = Group {
                status: TodoStatus::NoMpn,
                cover: None,
                mpn: None,
                covered_by: Vec::new(),
                other: Vec::new(),
            };
            add(&mut entries, &placed, g, fps);
        }
        for m in &mpns {
            let p = pairs::judge(lock, &placed.hash, m);
            let g = Group {
                status: TodoStatus::of(p.status),
                cover: Some(p.cover()),
                mpn: Some(p.mpn),
                covered_by: p.covered_by,
                other: p.other,
            };
            add(&mut entries, &placed, g, fps);
        }
    }
    entries.sort_by(|a, b| {
        a.status
            .cmp(&b.status)
            .then_with(|| erc_model::natural_cmp(&a.footprint, &b.footprint))
            .then_with(|| a.cover.cmp(&b.cover))
    });
    let mut files: Vec<PathBuf> = board
        .source
        .files
        .iter()
        .map(|f| project.dir.join(f))
        .chain([project.dir.join(&lay.file), project.parts_lock_path()])
        .chain(syms.files())
        .chain(fps.files())
        .collect();
    files.sort();
    files.dedup();
    let mut ids: Vec<i64> = entries
        .iter()
        .flat_map(|e| {
            e.covered_by
                .iter()
                .copied()
                .chain(e.other.iter().map(|o| o.id))
        })
        .collect();
    ids.sort_unstable();
    ids.dedup();
    let reviews = ids
        .iter()
        .filter_map(|id| lock.review(*id).cloned())
        .collect();
    Ok(FpTodo {
        board: project.dir.join(&lay.file),
        lock: project.parts_lock_path(),
        entries,
        reviews,
        files,
    })
}

impl FpTodo {
    /// The entries that need a review, in the order to review them
    pub fn to_review(&self) -> impl Iterator<Item = &TodoEntry> {
        self.entries.iter().filter(|e| e.needs_review())
    }

    /// Nothing to review and nothing that can't be, no overridden footprint and no library outside what is
    /// allowed
    pub fn is_ok(&self) -> bool {
        self.entries
            .iter()
            .all(|e| e.is_ok() && !e.is_overridden() && e.is_allowed())
    }

    /// How many entries have each status, the statuses that occur, in list order
    pub fn counts(&self) -> Vec<(TodoStatus, usize)> {
        let mut out: Vec<(TodoStatus, usize)> = Vec::new();
        for e in &self.entries {
            match out.iter_mut().find(|(s, _)| *s == e.status) {
                Some(n) => n.1 += 1,
                None => out.push((e.status, 1)),
            }
        }
        out
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialises")
    }

    /// One line per entry to review, then those that can't be under their own heading, then (with `all`) the
    /// reviewed ones, else their count and how to list them; statuses in colour (the caller's stream drops it off a
    /// terminal)
    pub fn to_text_color(&self, all: bool) -> String {
        use anstyle::{AnsiColor, Style};
        use std::fmt::Write as _;
        let style = |s: TodoStatus| match s {
            TodoStatus::Covered => AnsiColor::Green.on_default(),
            TodoStatus::Changed => AnsiColor::Red.on_default().bold(),
            TodoStatus::OtherVersion | TodoStatus::Uncovered => {
                AnsiColor::Yellow.on_default().bold()
            }
            TodoStatus::NotInLock | TodoStatus::UnknownMpn | TodoStatus::NoMpn => {
                AnsiColor::Cyan.on_default()
            }
        };
        let dim = Style::new().dimmed();
        let shown: Vec<&TodoEntry> = self
            .entries
            .iter()
            .filter(|e| all || !e.is_ok() || e.is_overridden() || !e.is_allowed())
            .collect();
        let fp_width = shown.iter().map(|e| e.footprint.len()).max().unwrap_or(0);
        let what = |e: &TodoEntry| match &e.cover {
            Some(c @ Cover::Family { .. }) => format!("family {}", c.describe()),
            Some(Cover::Mpn { mpn }) => mpn.clone(),
            None => e.symbol.clone().unwrap_or_default(),
        };
        let what_width = shown.iter().map(|e| what(e).len()).max().unwrap_or(0);
        let mut out = String::new();
        let blocked = shown
            .iter()
            .filter(|e| !e.needs_review() && !e.is_ok())
            .count();
        let done = self.entries.iter().filter(|e| e.is_ok()).count();
        let mut heading_for = None;
        for e in &shown {
            let group = if e.needs_review() {
                0
            } else if e.is_ok() {
                2
            } else {
                1
            };
            if heading_for != Some(group) {
                heading_for = Some(group);
                match group {
                    1 => {
                        let _ = writeln!(
                            out,
                            "{dim}can't be reviewed until fixed ({blocked}):{dim:#}"
                        );
                    }
                    2 => {
                        let _ = writeln!(out, "{dim}reviewed ({done}):{dim:#}");
                    }
                    _ => {}
                }
            }
            let st = style(e.status);
            let _ = write!(
                out,
                "{st}{:<13}{st:#} {:<fp_width$} {:<what_width$} {}",
                e.status.name(),
                e.footprint,
                what(e),
                e.hash
            );
            if matches!(e.cover, Some(Cover::Family { .. })) {
                let _ = write!(out, "  {dim}MPNs{dim:#} {}", e.mpns.join(" "));
            }
            let _ = write!(out, "  {dim}on{dim:#} {}", e.references.join(" "));
            if !e.other.is_empty() {
                let others: Vec<String> = e
                    .other
                    .iter()
                    .map(|o| format!("#{} {}", o.id, o.hash))
                    .collect();
                let _ = write!(
                    out,
                    "\n{:13} {dim}reviewed as{dim:#} {}",
                    "",
                    others.join(", ")
                );
            }
            if let (Some(lib), Some(file)) = (&e.library_hash, &e.library_file)
                && *lib != e.hash
            {
                let warn = AnsiColor::Yellow.on_default();
                let _ = write!(
                    out,
                    "\n{:13} {warn}library footprint {} has {lib}: update the board from the library before \
                     reviewing{warn:#}",
                    "",
                    file.display()
                );
            }
            if e.status == TodoStatus::NotInLock {
                let _ = write!(
                    out,
                    "\n{:13} {dim}not in {}: run `berc parts refresh`{dim:#}",
                    "",
                    self.lock.display()
                );
            }
            if !e.overridden.is_empty() {
                let note = AnsiColor::Cyan.on_default();
                let placed: Vec<String> = e
                    .overridden
                    .iter()
                    .map(|(r, fp)| format!("{r} (symbol sets {fp})"))
                    .collect();
                let _ = write!(
                    out,
                    "\n{:13} {note}placed as {}: {}: the review follows the board, not the symbol's own \
                     Footprint{note:#}",
                    "",
                    e.footprint,
                    placed.join(", "),
                );
            }
            if let Some(lib) = &e.symbol_library_disallowed {
                let warn = AnsiColor::Yellow.on_default();
                let _ = write!(
                    out,
                    "\n{:13} {warn}symbol library {lib} is not in the allowed libraries{warn:#}",
                    "",
                );
            }
            if let Some(lib) = &e.footprint_library_disallowed {
                let warn = AnsiColor::Yellow.on_default();
                let _ = write!(
                    out,
                    "\n{:13} {warn}footprint library {lib} is not in the allowed libraries{warn:#}",
                    "",
                );
            }
            out.push('\n');
        }
        if !all && done > 0 {
            let _ = writeln!(
                out,
                "{dim}{done} reviewed, not listed: --all lists them{dim:#}"
            );
        }
        let counts: Vec<String> = self
            .counts()
            .iter()
            .map(|(s, n)| {
                let st = style(*s);
                format!("{st}{n} {}{st:#}", s.name())
            })
            .collect();
        let _ = write!(
            out,
            "{}: {} footprint pairs, {} to review: {}",
            self.board.display(),
            self.entries.len(),
            self.to_review().count(),
            if counts.is_empty() {
                "none".to_string()
            } else {
                counts.join(", ")
            }
        );
        let overridden = self.entries.iter().filter(|e| e.is_overridden()).count();
        if overridden > 0 {
            let _ = write!(out, "; {overridden} with an overridden footprint");
        }
        let disallowed = self.entries.iter().filter(|e| !e.is_allowed()).count();
        if disallowed > 0 {
            let _ = write!(out, "; {disallowed} outside the allowed libraries");
        }
        out.push('\n');
        out
    }
}
