//! `berc fp todo`: what a board's footprints need reviewed, one entry per reviewed footprint hash (the library
//! footprint's, [`super::reviewed`]) and family (or MPN reviewed alone), in the order a review steps through them. One review of a footprint against a family's
//! datasheet covers every part of the family placed with it, on this board and every other.

use super::check::{Opened, field_of, is_lib_name};
use super::group::{self, FootprintTodo};
use super::library::lib_allowed;
use super::pairs::{self, OtherReview, PairStatus};
use super::reviewed::{self, Resolver, Reviewed, ReviewedAs};
use super::{FpCheckError, LibrarySearch};
use erc_model::fphash::Differences;
use erc_model::layout::FootprintId;
use erc_parts::lock::{Cover, LockedPart, LockedReview};
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

/// A board copy that differs from the library footprint reviewed: not part of the review, reported apart
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoardCopy {
    pub reference: String,
    /// The copy's own review hash
    pub hash: String,
    /// What it differs in, against the library footprint
    pub differences: Differences,
}

impl BoardCopy {
    /// The differences for the person, one line each
    pub fn describe(&self) -> Vec<String> {
        self.differences.describe("library", "board")
    }
}

/// One reviewed footprint hash and what one review of it covers
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TodoEntry {
    /// The review hash of the footprint reviewed: the library footprint's, else the board copy's as placed
    pub hash: String,
    /// Which footprint [`hash`](Self::hash) is of
    pub reviewed_as: ReviewedAs,
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
    /// The library `.kicad_mod` reviewed, when the footprint libraries give it
    pub library_file: Option<PathBuf>,
    /// Why the footprint is reviewed as placed: no library file was found
    #[serde(default)]
    pub placed_note: Option<String>,
    /// The references' board copies that differ from the library footprint, in natural order
    #[serde(default)]
    pub board_copies: Vec<BoardCopy>,
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
    /// The board setup's mask and paste margins for pads that set none, when it sets any: one board-wide setting,
    /// not a difference of any footprint
    #[serde(default)]
    pub board_margins: Option<String>,
    /// The parts snapshot's parts of the entries' MPNs, for [`FpTodo::by_footprint`]
    #[serde(skip)]
    pub parts: Vec<LockedPart>,
}

/// One board footprint as a todo entry takes it in
struct Placed<'a> {
    reviewed: Reviewed,
    lib_id: &'a str,
    reference: &'a str,
    symbol: &'a str,
    datasheet: Option<String>,
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
fn add(entries: &mut Vec<TodoEntry>, f: &Placed, g: Group) {
    let done = g.status == TodoStatus::Covered;
    let r = &f.reviewed;
    let at = entries.iter().position(|e| {
        e.hash == r.hash && (e.status == TodoStatus::Covered) == done && e.cover == g.cover
    });
    let e = match at {
        Some(i) => &mut entries[i],
        None => {
            entries.push(TodoEntry {
                hash: r.hash.clone(),
                reviewed_as: r.reviewed_as,
                footprint: f.lib_id.to_owned(),
                status: g.status,
                cover: g.cover.clone(),
                mpns: Vec::new(),
                references: Vec::new(),
                covered_by: Vec::new(),
                other: Vec::new(),
                symbol: Some(f.symbol.to_owned()),
                datasheet: f.datasheet.clone(),
                library_file: r.library_file.clone(),
                placed_note: r.note.clone(),
                board_copies: Vec::new(),
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
    if !e.references.iter().any(|x| x == f.reference) {
        e.references.push(f.reference.to_owned());
        if r.board_differs() {
            e.board_copies.push(BoardCopy {
                reference: f.reference.to_owned(),
                hash: r.board_hash.clone(),
                differences: r.differences.clone(),
            });
        }
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
        let reviewed = Resolver {
            syms: &mut *syms,
            fps: &mut *fps,
        }
        .resolve(board, FootprintId::from_index(i));
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
        // an mx3: link with the part's fields filled in, as the schematic editor's Show Datasheet passes it
        let datasheet = part
            .field("Datasheet")
            .map(str::trim)
            .filter(|d| !d.is_empty() && *d != "~")
            .map(|d| {
                erc_parts::mpn::mx3_link_resolved(d, |n| {
                    let f = part
                        .fields
                        .iter()
                        .find(|f| f.name.eq_ignore_ascii_case(n))?;
                    part.field(&f.name)
                })
                .unwrap_or_else(|| d.to_owned())
            });
        let placed = Placed {
            reviewed,
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
            add(&mut entries, &placed, g);
        }
        for m in &mpns {
            let p = pairs::judge(lock, &placed.reviewed.hash, m);
            let g = Group {
                status: TodoStatus::of(p.status),
                cover: Some(p.cover()),
                mpn: Some(p.mpn),
                covered_by: p.covered_by,
                other: p.other,
            };
            add(&mut entries, &placed, g);
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
    let parts = lock
        .parts
        .iter()
        .filter(|p| entries.iter().any(|e| e.mpns.contains(&p.mpn)))
        .cloned()
        .collect();
    Ok(FpTodo {
        board: project.dir.join(&lay.file),
        lock: project.parts_lock_path(),
        entries,
        reviews,
        files,
        board_margins: reviewed::board_margins_note(&super::outline::Margins::of_setup(&lay.setup)),
        parts,
    })
}

impl FpTodo {
    /// The entries by footprint hash, in the order to review them: what one review of a footprint can cover
    pub fn by_footprint(&self) -> Vec<FootprintTodo> {
        group::by_footprint(&self.entries, &self.parts)
    }

    /// The entries that need a review, in the order to review them
    pub fn to_review(&self) -> impl Iterator<Item = &TodoEntry> {
        self.entries.iter().filter(|e| e.needs_review())
    }

    /// Nothing to review and nothing that can't be, no overridden footprint, no board copy that differs from its
    /// library footprint and no library outside what is allowed
    pub fn is_ok(&self) -> bool {
        self.entries
            .iter()
            .all(|e| e.is_ok() && !e.is_overridden() && e.board_copies.is_empty() && e.is_allowed())
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

    /// One line per footprint hash with a review to do, its families and MPNs under it; then the footprints that
    /// can't be reviewed under their own heading; then (with `all`) the reviewed ones, else their count and how to
    /// list them. Statuses in colour (the caller's stream drops it off a terminal).
    pub fn to_text_color(&self, all: bool) -> String {
        use std::fmt::Write as _;
        let dim = anstyle::Style::new().dimmed();
        let shown_entry = |e: &TodoEntry| {
            all || !e.is_ok() || e.is_overridden() || !e.board_copies.is_empty() || !e.is_allowed()
        };
        let groups: Vec<FootprintTodo> = self
            .by_footprint()
            .into_iter()
            .filter(|g| g.entries.iter().any(shown_entry))
            .collect();
        let fp_width = groups
            .iter()
            .map(|g| g.footprints().join(" ").len())
            .max()
            .unwrap_or(0);
        let what_width = groups
            .iter()
            .flat_map(|g| &g.entries)
            .filter(|e| shown_entry(e))
            .map(|e| what(e).len())
            .max()
            .unwrap_or(0);
        let all_groups = self.by_footprint();
        let blocked = groups
            .iter()
            .filter(|g| !g.needs_review() && !g.is_ok())
            .count();
        let done = all_groups.iter().filter(|g| g.is_ok()).count();
        let mut out = String::new();
        let mut heading_for = None;
        for g in &groups {
            let bucket = if g.needs_review() {
                0
            } else if g.is_ok() {
                2
            } else {
                1
            };
            if heading_for != Some(bucket) {
                heading_for = Some(bucket);
                match bucket {
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
            let st = style(g.status());
            let _ = write!(
                out,
                "{st}{:<13}{st:#} {:<fp_width$} {}",
                g.status().name(),
                g.footprints().join(" "),
                g.hash
            );
            let open = g.to_review().count();
            if open > 0 {
                let _ = write!(out, "  {dim}{open} of {} to review{dim:#}", g.entries.len());
            }
            out.push('\n');
            if !g.facts.differ.is_empty() && g.facts.mpns.len() > 1 {
                let keys: Vec<&str> = g.facts.differ.iter().map(|(k, _)| k.as_str()).collect();
                let _ = writeln!(
                    out,
                    "{:13}   {dim}the parts differ in{dim:#} {}",
                    "",
                    keys.join(", ")
                );
            }
            for e in g.entries.iter().filter(|e| shown_entry(e)) {
                self.entry_text(&mut out, e, what_width);
            }
        }
        let hidden = self.entries.iter().filter(|e| !shown_entry(e)).count();
        if hidden > 0 {
            let _ = writeln!(
                out,
                "{dim}{hidden} reviewed, not listed: --all lists them{dim:#}"
            );
        }
        if let Some(m) = &self.board_margins {
            let _ = writeln!(
                out,
                "{dim}board setup margins for pads that set none, on every footprint alike: {m}{dim:#}"
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
            "{}: {} footprints, {} to review; {} footprint pairs: {}",
            self.board.display(),
            all_groups.len(),
            all_groups.iter().filter(|g| g.needs_review()).count(),
            self.entries.len(),
            if counts.is_empty() {
                "none".to_string()
            } else {
                counts.join(", ")
            }
        );
        let mut copies: Vec<&str> = self
            .entries
            .iter()
            .flat_map(|e| &e.board_copies)
            .map(|c| c.reference.as_str())
            .collect();
        copies.sort_unstable();
        copies.dedup();
        if !copies.is_empty() {
            let _ = write!(
                out,
                "; {} board copies differ from their library footprint",
                copies.len()
            );
        }
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

    /// One entry under its footprint's line: its status, family or MPN, references, and what stands in its way
    fn entry_text(&self, out: &mut String, e: &TodoEntry, what_width: usize) {
        use anstyle::AnsiColor;
        use std::fmt::Write as _;
        let dim = anstyle::Style::new().dimmed();
        let pad = "";
        let st = style(e.status);
        let _ = write!(
            out,
            "{pad:13}   {st}{:<13}{st:#} {:<what_width$}",
            e.status.name(),
            what(e)
        );
        if matches!(e.cover, Some(Cover::Family { .. })) {
            let _ = write!(out, "  {dim}MPNs{dim:#} {}", e.mpns.join(" "));
        }
        let _ = write!(out, "  {dim}on{dim:#} {}", e.references.join(" "));
        let note = |out: &mut String, text: String| {
            let _ = write!(out, "\n{pad:13}{pad:17}{text}");
        };
        if !e.other.is_empty() {
            let others: Vec<String> = e
                .other
                .iter()
                .map(|o| format!("#{} {}", o.id, o.hash))
                .collect();
            note(
                out,
                format!("{dim}reviewed as{dim:#} {}", others.join(", ")),
            );
        }
        let warn = AnsiColor::Yellow.on_default();
        if let Some(why) = &e.placed_note {
            note(
                out,
                format!("{dim}reviewed as placed, no library footprint: {why}{dim:#}"),
            );
        }
        for c in &e.board_copies {
            note(
                out,
                format!(
                    "{warn}board copy {} ({}) differs from the library footprint, not part of the review:{warn:#}",
                    c.reference, c.hash
                ),
            );
            for l in c.describe() {
                note(out, format!("  {warn}{l}{warn:#}"));
            }
        }
        if e.status == TodoStatus::NotInLock {
            note(
                out,
                format!(
                    "{dim}not in {}: run `berc parts refresh`{dim:#}",
                    self.lock.display()
                ),
            );
        }
        if !e.overridden.is_empty() {
            let cyan = AnsiColor::Cyan.on_default();
            let placed: Vec<String> = e
                .overridden
                .iter()
                .map(|(r, fp)| format!("{r} (symbol sets {fp})"))
                .collect();
            note(
                out,
                format!(
                    "{cyan}placed as {}: {}: the review follows the board, not the symbol's own \
                     Footprint{cyan:#}",
                    e.footprint,
                    placed.join(", "),
                ),
            );
        }
        if let Some(lib) = &e.symbol_library_disallowed {
            note(
                out,
                format!("{warn}symbol library {lib} is not in the allowed libraries{warn:#}"),
            );
        }
        if let Some(lib) = &e.footprint_library_disallowed {
            note(
                out,
                format!("{warn}footprint library {lib} is not in the allowed libraries{warn:#}"),
            );
        }
        out.push('\n');
    }
}

/// A status in its colour
fn style(s: TodoStatus) -> anstyle::Style {
    use anstyle::AnsiColor;
    match s {
        TodoStatus::Covered => AnsiColor::Green.on_default(),
        TodoStatus::Changed => AnsiColor::Red.on_default().bold(),
        TodoStatus::OtherVersion | TodoStatus::Uncovered => AnsiColor::Yellow.on_default().bold(),
        TodoStatus::NotInLock | TodoStatus::UnknownMpn | TodoStatus::NoMpn => {
            AnsiColor::Cyan.on_default()
        }
    }
}

/// What an entry stands for: its family, its MPN, or (no MPN) its symbol
fn what(e: &TodoEntry) -> String {
    match &e.cover {
        Some(c @ Cover::Family { .. }) => format!("family {}", c.describe()),
        Some(Cover::Mpn { mpn }) => mpn.clone(),
        None => e.symbol.clone().unwrap_or_default(),
    }
}
