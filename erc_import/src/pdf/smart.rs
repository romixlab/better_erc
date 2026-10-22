//! Altium Smart PDF: the schematic print Altium Designer writes with its netlist in the bookmarks.
//!
//! The bookmark tree holds one bookmark per sheet (`MCU.SchDoc(U_MCU)`, nested as the sheets are), and under each
//! sheet the sections `Components` (each part, or each unit of a part as `U1A`, with its pins `U1-C5`), `Nets` (each
//! net with its `Pins`, `NetLabels` and `Ports`; a bus `PA[0..15]` holds its member nets) and `Ports`. Net names are
//! the compiled names, the same on every sheet a net reaches, so the nets of the sheets join by name.
//!
//! The part properties (`Value`, `Footprint`, `Mfr. PN`, ...) are only in a property popup: a link annotation over
//! the symbol calls a document-level script `ShowCompProps_<guid>()` whose `app.popUpMenu("Value: 10K", ...)` lists
//! them, without the designator. Altium also writes invisible words over each symbol and pin, `CO<designator>` and
//! `PI<designator>0<pin>` (and `NL<label>` at each net label) for the bookmarks' search scripts; the word inside a
//! popup's rectangle names the part. Those facts are [`PdfConfidence::Position`]; the netlist itself is
//! [`PdfConfidence::Bookmarks`].

use super::doc::{Doc, Link, Outline, Word};
use super::js;
use crate::build::{Builder, net_kind};
use crate::notes::Notes;
use erc_model::*;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

/// How far outside a popup's rectangle a word may start and still count as inside (points): Altium puts the
/// `CO` word on the edge of the symbol's box
const SLACK: f64 = 1.5;

/// The page key of a sheet whose bookmark leads to no page
const ANY_PAGE: usize = usize::MAX;

/// A text as Altium spells it in its hidden words: every character but a letter or digit becomes `0` (`SD_CS` →
/// `SD0CS`, the pin `U1-G9` → `U10G9`)
fn ident(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '0' })
        .collect()
}

/// Whether an outline looks like a Smart PDF: some bookmark has `Components` or `Nets` under it
pub(crate) fn looks_like(outline: &Outline) -> bool {
    outline
        .items
        .iter()
        .any(|b| sections(outline, b).next().is_some())
}

fn sections<'o>(
    outline: &'o Outline,
    b: &'o super::doc::Bookmark,
) -> impl Iterator<Item = usize> + 'o {
    b.children
        .iter()
        .copied()
        .filter(|&c| matches!(outline.items[c].title.trim(), "Components" | "Nets"))
}

/// How well a popup's words name a component: words strictly inside, then words within the slack, then the
/// smaller mean distance from the popup's centre
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Score {
    strict: usize,
    near: usize,
    distance: u64,
}

impl Ord for Score {
    fn cmp(&self, o: &Score) -> std::cmp::Ordering {
        self.strict
            .cmp(&o.strict)
            .then(self.near.cmp(&o.near))
            .then(o.distance.cmp(&self.distance))
    }
}

impl PartialOrd for Score {
    fn partial_cmp(&self, o: &Score) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}

/// A net as one sheet lists it
struct SheetNet {
    sheet: SheetId,
    page: Option<usize>,
    /// The name on this sheet
    name: String,
    /// The bus it is a member of
    bus: Option<String>,
    /// Pin bookmarks (`U1-C5`)
    pins: Vec<String>,
    labels: Vec<String>,
    /// Port bookmarks: the title (the net's name, as Altium writes it) and the port's search word
    ports: Vec<(String, Option<String>)>,
}

/// The word of a bookmark's `_FindWord(page, "WORD")` script
fn find_word(scripts: &[String]) -> Option<String> {
    scripts
        .iter()
        .flat_map(|s| js::call_args(s, "_FindWord"))
        .find_map(|args| args.into_iter().nth(1))
}

/// Disjoint sets over `0..n`
struct UnionFind(Vec<usize>);

impl UnionFind {
    fn new(n: usize) -> UnionFind {
        UnionFind((0..n).collect())
    }

    fn find(&mut self, mut i: usize) -> usize {
        while self.0[i] != i {
            self.0[i] = self.0[self.0[i]];
            i = self.0[i];
        }
        i
    }

    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.0[a.max(b)] = a.min(b);
        }
    }
}

/// A sheet bookmark
struct SheetNode {
    item: usize,
    id: SheetId,
}

/// What [`read`] needs from the file besides the outline
pub(crate) struct Extras {
    pub links: Vec<Link>,
    pub scripts: Vec<(String, String)>,
    pub words: Vec<Word>,
    /// Media box per page
    pub pages: Vec<Option<[f64; 4]>>,
}

impl Extras {
    pub fn read(doc: &Doc, file: &str, notes: &mut Notes) -> Extras {
        let (links, cut) = doc.links();
        if let Some(c) = cut {
            notes.add(file, "links", NoteKind::Invalid, c);
        }
        let (scripts, cut) = doc.document_scripts();
        if let Some(c) = cut {
            notes.add(file, "scripts", NoteKind::Invalid, c);
        }
        let (words, gaps) = doc.words();
        if let Some(c) = gaps.truncated {
            notes.add(file, "page text", NoteKind::Invalid, c);
        }
        let pages = (0..doc.page_count()).map(|p| doc.media_box(p)).collect();
        Extras {
            links,
            scripts,
            words,
            pages,
        }
    }
}

/// Builds the board from the outline and the extras
pub(crate) fn read(
    file: &str,
    outline: &Outline,
    extras: &Extras,
    creator: Option<String>,
    notes: &mut Notes,
) -> Board {
    let name = super::file_stem(file);
    let mut b = Builder::new(name, "altium-pdf");
    b.b.source.version = creator;
    b.b.source.files.push(file.to_string());
    let netlist = Provenance::Pdf {
        file: file.to_string(),
        confidence: PdfConfidence::Bookmarks,
    };
    b.b.source.connectivity = Some(netlist);
    if let Some(c) = &outline.truncated {
        notes.add(file, "bookmarks", NoteKind::Invalid, c.clone());
    }
    if outline.loops > 0 {
        notes.add(
            file,
            "bookmarks",
            NoteKind::Invalid,
            format!(
                "{} bookmark(s) reached twice (a loop in the tree), not followed",
                outline.loops
            ),
        );
    }

    let sheets = sheets(outline, &mut b, file, notes);
    let parents: HashSet<SheetId> = b.b.sheets.iter().filter_map(|s| s.parent).collect();
    let mut r = Reader {
        file,
        outline,
        b,
        notes,
        units: HashMap::new(),
        placed_on: HashMap::new(),
        hidden: HashMap::new(),
        parents,
        sheet_nets: Vec::new(),
        port_names: HashMap::new(),
    };
    // Components of every sheet first: the pins of the nets are split by the designators they name
    for s in &sheets {
        r.components(s);
    }
    for s in &sheets {
        r.nets(s);
    }
    r.join_nets(extras);
    r.properties(extras);
    r.b.finish()
}

/// Every sheet bookmark, created as a sheet of the board under its parent sheet bookmark (or the root)
fn sheets(outline: &Outline, b: &mut Builder, file: &str, notes: &mut Notes) -> Vec<SheetNode> {
    let mut out = Vec::new();
    // Schematic sheets are named by their `.SchDoc`; a file without such names (an old Altium) has its sheets
    // known by their sections only
    let named = outline.items.iter().any(|i| sheet_file(&i.title).is_some());
    // (bookmark, sheet of the nearest sheet bookmark above it)
    let mut todo: Vec<(usize, SheetId)> = outline
        .roots
        .iter()
        .rev()
        .map(|&r| (r, b.root()))
        .collect();
    let mut names: HashSet<(SheetId, String)> = HashSet::new();
    while let Some((i, parent)) = todo.pop() {
        let item = &outline.items[i];
        let has_sections = sections(outline, item).next().is_some();
        let is_sheet = sheet_file(&item.title).is_some() || (!named && has_sections);
        if has_sections && !is_sheet {
            notes.ignored(
                file,
                &format!("bookmark {}", item.title.trim()),
                "a component list outside the schematic sheets (an assembly drawing of the board): not read",
            );
        }
        let mut here = parent;
        if is_sheet {
            let (file, mut name) = match sheet_file(&item.title) {
                Some((f, n)) => (f, n),
                None => (String::new(), item.title.trim().to_string()),
            };
            // Two sheets of one name under one parent would share a path: the second gets its bookmark number
            if !names.insert((parent, name.clone())) {
                name = format!("{name}#{i}");
                names.insert((parent, name.clone()));
            }
            let id = SheetId::from_index(b.b.sheets.len());
            b.b.sheets.push(Sheet {
                name,
                file,
                parent: Some(parent),
                uuid_path: String::new(),
                page: item.action.page.map(|p| (p + 1).to_string()),
            });
            out.push(SheetNode { item: i, id });
            here = id;
        }
        for &c in item.children.iter().rev() {
            if !is_section(&outline.items[c].title) {
                todo.push((c, here));
            }
        }
    }
    out
}

fn is_section(title: &str) -> bool {
    matches!(title.trim(), "Components" | "Nets" | "Ports")
}

/// `MCU.SchDoc(U_MCU)` → (`MCU.SchDoc`, `U_MCU`); `Board(A).SchDoc("Board(A)")` → the quoted name; a bare
/// `Power.SchDoc` → its stem
fn sheet_file(title: &str) -> Option<(String, String)> {
    let t = title.trim();
    let at = t.to_ascii_lowercase().find(".schdoc")?;
    let end = at + ".schdoc".len();
    let file = t[..end].to_string();
    let rest = t[end..].trim();
    let name = match rest.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        Some(inner) => inner.trim().trim_matches('"').to_string(),
        None => t[..at].to_string(),
    };
    let name = if name.is_empty() {
        t[..at].to_string()
    } else {
        name
    };
    Some((file, name))
}

struct Reader<'a> {
    file: &'a str,
    outline: &'a Outline,
    b: Builder,
    notes: &'a mut Notes,
    /// Component bookmark title (`U1A`, `R5`) → part and unit
    units: HashMap<String, (PartId, u32)>,
    /// Component bookmark title → the sheets it is listed on
    placed_on: HashMap<String, Vec<SheetId>>,
    /// (page, hidden word) → the component bookmark title it belongs to: `CO` + the title, `PI` + each pin
    /// bookmark, in Altium's identifier spelling ([`ident`]); [`ANY_PAGE`] for a sheet without a page
    hidden: HashMap<(usize, String), String>,
    /// Sheets with sheets under them (an overview sheet may hold nothing else)
    parents: HashSet<SheetId>,
    /// Every sheet's nets, as the sheet names them
    sheet_nets: Vec<SheetNet>,
    /// A port's search word (`POSAI40CK1`) → the port's name (`SAI4_CK1`), from the sheets' Ports sections
    port_names: HashMap<String, String>,
}

impl Reader<'_> {
    fn sheet_at(&self, s: &SheetNode) -> String {
        format!("sheet {}", self.b.b.sheet_path(s.id))
    }

    fn section(&self, s: &SheetNode, name: &str) -> Vec<usize> {
        self.outline.items[s.item]
            .children
            .iter()
            .copied()
            .filter(|&c| self.outline.items[c].title.trim() == name)
            .collect()
    }

    fn title(&self, i: usize) -> &str {
        self.outline.items[i].title.trim()
    }

    fn components(&mut self, s: &SheetNode) {
        for sec in self.section(s, "Components") {
            for &c in &self.outline.items[sec].children {
                let title = self.title(c).to_string();
                let pins: Vec<String> = self.outline.items[c]
                    .children
                    .iter()
                    .map(|&p| self.title(p).to_string())
                    .collect();
                let (designator, unit) = designator_of(&title, &pins);
                let placed = self.placed_on.entry(title.clone()).or_default();
                placed.push(s.id);
                if placed.len() == 2 {
                    let at = self.sheet_at(s);
                    self.notes.invalid(
                        self.file,
                        &at,
                        format!(
                            "component {title} is listed on two sheets: a logical print of a repeated sheet joins \
                             its channels; print the Smart PDF with physical designators"
                        ),
                    );
                }
                let part = match self.b.find_part(&designator) {
                    Some(p) => p,
                    None => {
                        let p = self.b.part(&designator);
                        self.b.part_mut(p).sheet = s.id;
                        p
                    }
                };
                let unit_no = unit_number(unit);
                if !self.b.b.parts[part.index()]
                    .units
                    .iter()
                    .any(|u| u.unit == unit_no && u.sheet == s.id)
                {
                    self.b.part_mut(part).units.push(UnitPlacement {
                        unit: unit_no,
                        sheet: s.id,
                        uuid: String::new(),
                        line: None,
                        at: None,
                    });
                }
                self.units.insert(title.clone(), (part, unit_no));
                let page = self.outline.items[s.item].action.page.unwrap_or(ANY_PAGE);
                self.hidden
                    .insert((page, format!("CO{}", ident(&title))), title.clone());
                for p in &pins {
                    self.hidden
                        .insert((page, format!("PI{}", ident(p))), title.clone());
                }
                for p in pins {
                    match p
                        .strip_prefix(designator.as_str())
                        .and_then(|r| r.strip_prefix('-'))
                    {
                        Some(number) if !number.is_empty() => {
                            let pin = self.b.pin(part, number);
                            if self.b.b.pins[pin.index()].unit == 0 {
                                self.b.pin_mut(pin).unit = unit_no;
                            }
                        }
                        _ => self.notes.invalid(
                            self.file,
                            &format!("component {title}"),
                            format!("pin bookmark `{p}` does not name a pin of {designator}"),
                        ),
                    }
                }
            }
        }
    }

    /// The part and pin number a pin bookmark (`U1-C5`) names: the longest designator it starts with
    fn split_pin(&mut self, title: &str, at: &str) -> Option<(PartId, String)> {
        let mut best: Option<(usize, PartId)> = None;
        let mut search = title;
        while let Some(i) = search.rfind('-') {
            let des = &title[..i];
            if let Some(p) = self.b.find_part(des) {
                best = Some((i, p));
                break;
            }
            search = &title[..i];
        }
        match best {
            Some((i, p)) if i + 1 < title.len() => Some((p, title[i + 1..].to_string())),
            _ => {
                let (des, number) = title.split_once('-')?;
                if des.is_empty() || number.is_empty() {
                    return None;
                }
                self.notes.missing(
                    self.file,
                    at,
                    format!("pin {title}: no component {des} in the Components bookmarks"),
                );
                Some((self.b.part(des), number.to_string()))
            }
        }
    }

    /// The nets a sheet lists, as they are named on that sheet; joined across sheets by [`Reader::join_nets`]
    fn nets(&mut self, s: &SheetNode) {
        for sec in self.section(s, "Ports") {
            for &p in &self.outline.items[sec].children {
                if let Some(w) = find_word(&self.outline.items[p].action.scripts) {
                    self.port_names
                        .entry(w)
                        .or_insert_with(|| self.outline.items[p].title.trim().to_string());
                }
            }
        }
        for sec in self.section(s, "Nets") {
            // (net bookmark, the bus it is a member of)
            let mut todo: Vec<(usize, Option<String>)> = self.outline.items[sec]
                .children
                .iter()
                .rev()
                .map(|&n| (n, None))
                .collect();
            while let Some((n, bus)) = todo.pop() {
                let mut net = SheetNet {
                    sheet: s.id,
                    page: self.outline.items[s.item].action.page,
                    name: self.title(n).to_string(),
                    bus: bus.clone(),
                    pins: Vec::new(),
                    labels: Vec::new(),
                    ports: Vec::new(),
                };
                let mut members = Vec::new();
                for &k in &self.outline.items[n].children {
                    let leaves = self.outline.items[k].children.iter();
                    match self.title(k) {
                        "Pins" => net.pins.extend(leaves.map(|&l| self.title(l).to_string())),
                        "NetLabels" => net
                            .labels
                            .extend(leaves.map(|&l| self.title(l).to_string())),
                        "Ports" => net.ports.extend(leaves.map(|&l| {
                            let item = &self.outline.items[l];
                            (
                                item.title.trim().to_string(),
                                find_word(&item.action.scripts),
                            )
                        })),
                        _ => members.push(k),
                    }
                }
                for &m in members.iter().rev() {
                    todo.push((m, Some(net.name.clone())));
                }
                // A bus holds its member nets and nothing of its own
                if !(net.pins.is_empty()
                    && net.labels.is_empty()
                    && net.ports.is_empty()
                    && !members.is_empty())
                {
                    self.sheet_nets.push(net);
                }
            }
        }
        if self.section(s, "Components").is_empty()
            && self.section(s, "Nets").is_empty()
            && !self.parents.contains(&s.id)
        {
            let at = self.sheet_at(s);
            self.notes.missing(
                self.file,
                &at,
                "no Components or Nets bookmarks under this sheet",
            );
        }
    }

    /// Joins the sheets' nets into the board's nets, the way Altium connects sheets as far as the bookmarks show
    /// it. A sheet names a net after its label, power port or port on that sheet, else after a pin of it
    /// (`NetR87_1`), so one net has a name per sheet. In a flat design (no sheet below another) net labels, power
    /// ports and ports are global: nets of one name join, and so do nets with ports of one name. In a hierarchy net
    /// labels are local to their sheet: nets of one name join only when no label names them (power ports are
    /// global), and a port joins the nets with a port of the same name on the sibling sheets and the parent's net
    /// of that name, as the sheet entries on the parent connect them. The parent's wires between sheet entries are
    /// not in the bookmarks: same-named entries are taken as wired together, which a note says.
    fn join_nets(&mut self, extras: &Extras) {
        let n = self.sheet_nets.len();
        let mut uf = UnionFind::new(n);
        let parent_of = |s: SheetId| self.b.b.sheets[s.index()].parent;
        let hierarchy = self
            .b
            .b
            .sheets
            .iter()
            .any(|s| s.parent.is_some_and(|p| self.b.b.sheets[p.index()].parent.is_some()));

        let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, net) in self.sheet_nets.iter().enumerate() {
            by_name.entry(net.name.as_str()).or_default().push(i);
        }
        for list in by_name.values() {
            let global: Vec<usize> = list
                .iter()
                .copied()
                .filter(|&i| !hierarchy || self.sheet_nets[i].labels.is_empty())
                .collect();
            for w in global.windows(2) {
                uf.union(w[0], w[1]);
            }
        }

        // Ports by their search word, among siblings (a hierarchy) or everywhere (flat)
        let mut by_port: HashMap<(Option<SheetId>, &str), Vec<usize>> = HashMap::new();
        for (i, net) in self.sheet_nets.iter().enumerate() {
            for (_, w) in &net.ports {
                if let Some(w) = w {
                    let scope = if hierarchy { parent_of(net.sheet) } else { None };
                    let list = by_port.entry((scope, w.as_str())).or_default();
                    if !list.contains(&i) {
                        list.push(i);
                    }
                }
            }
        }
        let mut through_ports: HashSet<usize> = HashSet::new();
        for ((scope, word), list) in &by_port {
            for w in list.windows(2) {
                uf.union(w[0], w[1]);
            }
            let mut joined = list.len() > 1;
            // The parent's own net of the port's name
            if let Some(parent) = scope {
                let spelled = word.strip_prefix("PO").unwrap_or(word);
                for (j, net) in self.sheet_nets.iter().enumerate() {
                    if net.sheet == *parent && ident(&net.name) == spelled {
                        uf.union(list[0], j);
                        joined = true;
                    }
                }
            }
            if joined {
                through_ports.extend(list.iter().copied());
            }
        }

        // One board net per group, named by a label or power port before a port's name before a pin's
        let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for i in 0..n {
            groups.entry(uf.find(i)).or_default().push(i);
        }
        // A net with a port that joined nothing: the port leads to no other sheet's net
        let lonely: Vec<usize> = groups
            .values()
            .filter(|g| g.len() == 1 && !self.sheet_nets[g[0]].ports.is_empty())
            .map(|g| g[0])
            .collect();
        let depth = |s: SheetId| {
            let mut d = 0;
            let mut cur = parent_of(s);
            while let Some(p) = cur {
                d += 1;
                cur = parent_of(p);
            }
            d
        };
        let mut named: Vec<(Vec<usize>, String, NetKind)> = Vec::new();
        for (_, members) in groups {
            let mut by_rank: Vec<(u8, usize, &str)> = Vec::new();
            for &m in &members {
                let net = &self.sheet_nets[m];
                let rank = if net_kind(&net.name) == NetKind::Named { 0 } else { 2 };
                by_rank.push((rank, depth(net.sheet), &net.name));
                for (_, w) in &net.ports {
                    if let Some(real) = w.as_ref().and_then(|w| self.port_names.get(w)) {
                        by_rank.push((1, depth(net.sheet), real));
                    }
                }
            }
            by_rank.sort_by(|a, b| {
                a.0.cmp(&b.0)
                    .then(a.1.cmp(&b.1))
                    .then_with(|| natural_cmp(a.2, b.2))
            });
            let (rank, _, name) = by_rank[0];
            let kind = match rank {
                2 => net_kind(name),
                _ => NetKind::Named,
            };
            named.push((members, name.to_string(), kind));
        }
        // A name two nets would share (local labels of one text on two sheets) gets its sheet's path
        let mut count: HashMap<String, usize> = HashMap::new();
        for (_, name, _) in &named {
            *count.entry(name.clone()).or_default() += 1;
        }
        let mut labels_at: HashMap<SheetId, LabelWords> = HashMap::new();
        let mut joined_groups = 0;
        for (members, name, kind) in named {
            let first = self.sheet_nets[members[0]].sheet;
            let name = if count[&name] > 1 {
                format!("{}{name}", self.b.b.sheet_path(first))
            } else {
                name
            };
            if members.iter().any(|m| through_ports.contains(m)) && members.len() > 1 {
                joined_groups += 1;
            }
            let id = self.b.net(&name, kind);
            for m in members {
                let net = std::mem::take(&mut self.sheet_nets[m].pins);
                let sn = &self.sheet_nets[m];
                let (sheet, page) = (sn.sheet, sn.page);
                let at = match &sn.bus {
                    Some(bus) => format!("sheet {} net {} of bus {bus}", self.b.b.sheet_path(sheet), sn.name),
                    None => format!("sheet {} net {}", self.b.b.sheet_path(sheet), sn.name),
                };
                for p in net {
                    if let Some((part, number)) = self.split_pin(&p, &at) {
                        let pin = self.b.pin(part, &number);
                        self.b.connect(pin, id, self.file, self.notes);
                    }
                }
                let words = labels_at
                    .entry(sheet)
                    .or_insert_with(|| LabelWords::new(extras, page));
                let sn = &self.sheet_nets[m];
                let list: Vec<(LabelKind, String)> = sn
                    .labels
                    .iter()
                    .map(|t| (LabelKind::Local, t.clone()))
                    .chain(sn.ports.iter().map(|(t, w)| {
                        let real = w.as_ref().and_then(|w| self.port_names.get(w));
                        (LabelKind::Hierarchical, real.unwrap_or(t).clone())
                    }))
                    .collect();
                for (kind, text) in list {
                    let at = words.take(kind, &text);
                    self.b.b.labels.push(Label {
                        kind,
                        text,
                        sheet,
                        at,
                        shape: None,
                        child: None,
                        fields: Vec::new(),
                        net: Some(id),
                        dangling: false,
                    });
                }
            }
        }

        let unplaced: usize = labels_at.values().map(|w| w.unplaced).sum();
        let all = self
            .b
            .b
            .labels
            .iter()
            .filter(|l| l.kind == LabelKind::Local)
            .count();
        if unplaced > 0 {
            self.notes.missing(
                self.file,
                "net labels",
                format!(
                    "{unplaced} of {all} net labels have no position: no search word (`NL<label>`) with a position \
                     on their sheet's page"
                ),
            );
        }
        if hierarchy && joined_groups > 0 {
            self.notes.missing(
                self.file,
                "nets",
                format!(
                    "{joined_groups} net(s) join sheets through ports of one name: the wires between sheet entries \
                     on the parent sheets are not in the bookmarks, so same-named entries are taken as wired together"
                ),
            );
        }
        // Ports that lead to no other sheet's net, one note per sheet
        let mut per_sheet: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for i in lonely {
            let net = &self.sheet_nets[i];
            let names: Vec<String> = net
                .ports
                .iter()
                .map(|(t, w)| {
                    w.as_ref()
                        .and_then(|w| self.port_names.get(w))
                        .unwrap_or(t)
                        .clone()
                })
                .collect();
            per_sheet
                .entry(self.b.b.sheet_path(net.sheet))
                .or_default()
                .extend(names);
        }
        for (sheet, mut ports) in per_sheet {
            ports.sort_by(|a, b| natural_cmp(a, b));
            ports.dedup();
            self.notes.missing(
                self.file,
                &format!("sheet {sheet}"),
                format!(
                    "{} port(s) meet no port or net of their name on another sheet: {}",
                    ports.len(),
                    ports.join(", ")
                ),
            );
        }
    }

    /// Fields of the parts from the property popups. Each popup is tied to one component bookmark by the hidden
    /// words inside its rectangle, one popup per component: popups of parts drawn close together overlap, so the
    /// pairs are taken best first over the whole page ([`Score`]) rather than popup by popup.
    fn properties(&mut self, extras: &Extras) {
        let mut popups: HashMap<&str, Vec<(String, String)>> = HashMap::new();
        for (_, script) in &extras.scripts {
            for f in js::defined_functions(script) {
                if let Some(args) = js::call_args(script, "app.popUpMenu").into_iter().next() {
                    popups.insert(f, properties_of(&args));
                }
            }
        }
        let mut by_page: HashMap<usize, Vec<&Word>> = HashMap::new();
        for w in extras.words.iter().filter(|w| w.invisible) {
            by_page.entry(w.page).or_default().push(w);
        }
        let sheet_pages: HashSet<usize> = self.hidden.keys().map(|(p, _)| *p).collect();
        let mut found: Vec<(usize, &Link, Vec<(String, String)>)> = Vec::new();
        let mut elsewhere = 0;
        for link in &extras.links {
            let mut props = None;
            for script in &link.action.scripts {
                // A popup defined in the link's own script, or one it calls
                if let Some(args) = js::call_args(script, "app.popUpMenu").into_iter().next() {
                    props = Some(properties_of(&args));
                    break;
                }
                if let Some(p) = js::called_functions(script)
                    .into_iter()
                    .find_map(|f| popups.get(f))
                {
                    props = Some(p.clone());
                    break;
                }
            }
            let Some(props) = props else { continue };
            if !sheet_pages.contains(&link.page) && !sheet_pages.contains(&ANY_PAGE) {
                elsewhere += 1;
                continue;
            }
            found.push((found.len(), link, props));
        }
        let total = found.len() + elsewhere;

        // Every (popup, component) pair the words support, best first
        let mut pairs: Vec<(Score, usize, String)> = Vec::new();
        for (i, link, _) in &found {
            let words = by_page.get(&link.page).map_or(&[][..], |v| &v[..]);
            for (title, score) in self.candidates(link.page, &link.rect, words) {
                pairs.push((score, *i, title));
            }
        }
        pairs.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        let mut popup_done = vec![false; found.len()];
        let mut taken: HashSet<String> = HashSet::new();
        let mut contested = 0;
        for (_, i, title) in pairs {
            if popup_done[i] || taken.contains(&title) {
                continue;
            }
            let Some(&(part, _)) = self.units.get(&title) else {
                continue;
            };
            popup_done[i] = true;
            self.set_properties(part, &found[i].2);
            taken.insert(title);
        }
        let mut unbound = 0;
        for (i, link, _) in &found {
            if !popup_done[*i] {
                let words = by_page.get(&link.page).map_or(&[][..], |v| &v[..]);
                if self.candidates(link.page, &link.rect, words).is_empty() {
                    unbound += 1;
                } else {
                    contested += 1;
                }
            }
        }

        // Hidden words that all sit at one point carry no position (llPDFLib writes them so)
        let placed: HashSet<(i64, i64)> = extras
            .words
            .iter()
            .filter(|w| w.invisible && self.hidden.contains_key(&(w.page, w.text.clone())))
            .map(|w| (w.x.round() as i64, w.y.round() as i64))
            .collect();
        if unbound > 0 && taken.is_empty() && placed.len() <= 1 {
            self.notes.missing(
                self.file,
                "property popups",
                format!(
                    "none of the {total} property popups can be tied to a part: this file's search words \
                     (`CO<designator>`, `PI<pin>`) carry no position, so the properties (values, footprints, \
                     part numbers) are not read"
                ),
            );
        } else if unbound > 0 {
            self.notes.missing(
                self.file,
                "property popups",
                format!(
                    "{unbound} of {total} property popups have no component or pin word (`CO<designator>`, \
                     `PI<pin>`) inside them: their properties are not read"
                ),
            );
        }
        if contested > 0 {
            self.notes.invalid(
                self.file,
                "property popups",
                format!(
                    "{contested} of {total} property popups cover only components another popup covers better: \
                     their properties are not read"
                ),
            );
        }
        if elsewhere > 0 {
            self.notes.ignored(
                self.file,
                "property popups",
                format!(
                    "{elsewhere} of {total} property popups are on pages no sheet bookmark leads to (another \
                     document bound into the file): not read"
                ),
            );
        }
    }

    /// The component bookmark titles the hidden words inside a popup's rectangle name (`CO` of a component, `PI`
    /// of one of its pins), for components listed on this page, each with its [`Score`]
    fn candidates(&self, page: usize, rect: &[f64; 4], words: &[&Word]) -> Vec<(String, Score)> {
        let (cx, cy) = ((rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0);
        let mut out: Vec<(String, Score)> = Vec::new();
        for w in words {
            let strict = w.x >= rect[0] && w.x <= rect[2] && w.y >= rect[1] && w.y <= rect[3];
            let near = w.x >= rect[0] - SLACK
                && w.x <= rect[2] + SLACK
                && w.y >= rect[1] - SLACK
                && w.y <= rect[3] + SLACK;
            if !near {
                continue;
            }
            let Some(title) = self
                .hidden
                .get(&(page, w.text.clone()))
                .or_else(|| self.hidden.get(&(ANY_PAGE, w.text.clone())))
            else {
                continue;
            };
            let i = match out.iter().position(|(t, _)| t == title) {
                Some(i) => i,
                None => {
                    out.push((title.clone(), Score::default()));
                    out.len() - 1
                }
            };
            let s = &mut out[i].1;
            s.strict += usize::from(strict);
            s.near += 1;
            // Hundredths of a point, so the score orders exactly
            s.distance += ((w.x - cx).hypot(w.y - cy) * 100.0) as u64;
        }
        for (_, s) in &mut out {
            s.distance /= s.near as u64;
        }
        out
    }

    fn set_properties(&mut self, part: PartId, props: &[(String, String)]) {
        let src = Provenance::Pdf {
            file: self.file.to_string(),
            confidence: PdfConfidence::Position,
        };
        for (k, v) in props {
            if v.is_empty() {
                continue;
            }
            self.b
                .field(part, k, Fact::new(v.clone(), src.clone()).with_note("property popup"));
        }
        let get = |k: &str| props.iter().find(|(n, v)| n == k && !v.is_empty());
        if let Some((k, v)) = get("Value").or_else(|| get("Comment")) {
            self.b.value(part, v, &src, &format!("{k} property"));
        }
        if let Some((_, v)) = get("Library Reference") {
            self.b.part_mut(part).lib_id = v.clone();
        }
    }
}

/// `Key: Value` entries of a popup; an entry without a colon is a key with no value
fn properties_of(args: &[String]) -> Vec<(String, String)> {
    args.iter()
        .map(|a| match a.split_once(':') {
            Some((k, v)) => (k.trim().to_string(), v.trim().to_string()),
            None => (a.trim().to_string(), String::new()),
        })
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

/// The designator and unit suffix of a component bookmark: the prefix every pin bookmark starts with (`U1A` with
/// pins `U1-C5` → `U1`, `A`); the whole title when the pins do not tell
fn designator_of<'t>(title: &'t str, pins: &[String]) -> (String, &'t str) {
    let mut end = title.len();
    loop {
        let des = &title[..end];
        if !pins.is_empty()
            && pins.iter().all(|p| {
                p.strip_prefix(des)
                    .is_some_and(|r| r.starts_with('-') && r.len() > 1)
            })
        {
            return (des.to_string(), &title[end..]);
        }
        // Only letters are a unit suffix
        match title[..end].chars().last() {
            Some(c) if c.is_ascii_alphabetic() && end > 1 => end -= c.len_utf8(),
            _ => return (title.to_string(), ""),
        }
    }
}

/// `A` → 1, `B` → 2, ..., `AA` → 27; no suffix → 1
fn unit_number(suffix: &str) -> u32 {
    let mut n: u32 = 0;
    for c in suffix.chars() {
        let d = (c.to_ascii_uppercase() as u32).saturating_sub('A' as u32) + 1;
        n = n.saturating_mul(26).saturating_add(d);
    }
    n.max(1)
}

/// The `NL<text>` words of a sheet's page, handed out in order to the labels of that text
struct LabelWords {
    words: HashMap<String, VecDeque<Pt>>,
    unplaced: usize,
}

impl LabelWords {
    fn new(extras: &Extras, page: Option<usize>) -> LabelWords {
        let mut words: HashMap<String, VecDeque<Pt>> = HashMap::new();
        let mut positions: HashSet<(i64, i64)> = HashSet::new();
        if let Some(page) = page
            && let Some(Some(mb)) = extras.pages.get(page)
        {
            for w in extras
                .words
                .iter()
                .filter(|w| w.page == page && w.invisible)
            {
                positions.insert((w.x.round() as i64, w.y.round() as i64));
                if let Some(t) = w.text.strip_prefix("NL") {
                    // Keyed by the spelling of the word; a label's text is looked up through `ident`
                    words
                        .entry(t.to_string())
                        .or_default()
                        .push_back(page_point(mb, w.x, w.y));
                }
            }
        }
        // Search words that all sit at one point carry no position (llPDFLib writes them so)
        if positions.len() <= 1 {
            words.clear();
        }
        LabelWords { words, unplaced: 0 }
    }

    /// The position of the next label with this text; ports have no word of their own
    fn take(&mut self, kind: LabelKind, text: &str) -> Option<Pt> {
        if kind == LabelKind::Local
            && let Some(p) = self.words.get_mut(&ident(text)).and_then(VecDeque::pop_front)
        {
            return Some(p);
        }
        if kind == LabelKind::Local {
            self.unplaced += 1;
        }
        None
    }
}

/// A point of the page (points, y up, from the media box corner) as a schematic point (nm, y down from the top
/// left corner)
fn page_point(mb: &[f64; 4], x: f64, y: f64) -> Pt {
    let mm = 25.4 / 72.0;
    Pt::from_mm((x - mb[0]) * mm, (mb[3] - y) * mm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_titles() {
        assert_eq!(
            sheet_file("MCU.SchDoc(U_MCU)"),
            Some(("MCU.SchDoc".into(), "U_MCU".into()))
        );
        assert_eq!(
            sheet_file("NandFlash Board(A).SchDoc(\"NandFlash Board(A)\")"),
            Some(("NandFlash Board(A).SchDoc".into(), "NandFlash Board(A)".into()))
        );
        assert_eq!(
            sheet_file("JGL1905_Xyloni_FPGA_V1.06.SchDoc"),
            Some((
                "JGL1905_Xyloni_FPGA_V1.06.SchDoc".into(),
                "JGL1905_Xyloni_FPGA_V1.06".into()
            ))
        );
        assert_eq!(sheet_file("Components"), None);
    }

    #[test]
    fn designators_and_units() {
        let pins = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(designator_of("U1A", &pins(&["U1-C5", "U1-D7"])), ("U1".into(), "A"));
        assert_eq!(designator_of("R75", &pins(&["R75-1", "R75-2"])), ("R75".into(), ""));
        assert_eq!(
            designator_of("ETH2_JP1", &pins(&["ETH2_JP1-1"])),
            ("ETH2_JP1".into(), "")
        );
        assert_eq!(designator_of("TP1", &[]), ("TP1".into(), ""));
        assert_eq!(designator_of("Q1", &pins(&["X-1"])), ("Q1".into(), ""));
        assert_eq!(unit_number(""), 1);
        assert_eq!(unit_number("B"), 2);
        assert_eq!(unit_number("AA"), 27);
    }

    #[test]
    fn popup_properties() {
        let args: Vec<String> = ["Value: 10K", "Ibis Model: ", "Note", ": x", "URL: http://a:b"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            properties_of(&args),
            [
                ("Value".to_string(), "10K".to_string()),
                ("Ibis Model".into(), "".into()),
                ("Note".into(), "".into()),
                ("URL".into(), "http://a:b".into())
            ]
        );
    }
}
