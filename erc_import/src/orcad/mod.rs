//! OrCAD Capture's Allegro netlist: three files written together by PSTWRITER, read with the pest grammars
//! `grammar/orcad_capture_*.pest` into one board.
//!
//! - `pstchip.dat` (`LIBRARY_PARTS`): the library parts (primitives) with their pins (name, number per section,
//!   use and load) and body properties (`VALUE`, `JEDEC_TYPE`, `PART_NUMBER`, ...): the symbols.
//! - `pstxprt.dat` (`EXPANDEDPARTLIST`): each part by designator with its primitive and its sections (units), each
//!   with its instance path and the page it is drawn on: the parts and their sheets.
//! - `pstxnet.dat` (`EXPANDEDNETLIST`): the nets with their nodes (designator, pin number, instance, pin name).
//!
//! Any of the three can be the main file; the others are found by name beside it (`pstxnet` ↔ `pstxprt` ↔
//! `pstchip` in the file name). A missing one is a note, and the import goes on with what is there.
//!
//! A multi-section part's pin numbers are a list with one entry per section (`(A3,0,0)`: pin A3 in section 1);
//! each non-zero entry is a pin of that unit, so a pin that exists in two sections (`(1,7)`) is two pins. Pages
//! become sheets under the root (`page3`), the section letters the symbol's unit names.

use crate::build::Builder;
use crate::notes::Notes;
use crate::{dir_of, file_name, text};
use erc_model::*;
use std::collections::HashMap;

mod chip {
    #[derive(pest_derive::Parser)]
    #[grammar = "grammar/orcad_capture_library_parts.pest"]
    pub struct P;
}

mod parts {
    #[derive(pest_derive::Parser)]
    #[grammar = "grammar/orcad_capture_part_list.pest"]
    pub struct P;
}

mod nets {
    #[derive(pest_derive::Parser)]
    #[grammar = "grammar/orcad_capture_netlist.pest"]
    pub struct P;
}

use pest::Parser;

/// The OrCAD Capture importer (pstxnet, pstxprt, pstchip)
pub struct OrcadImporter;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Kind {
    Net,
    Parts,
    Chip,
}

impl Kind {
    const ALL: [Kind; 3] = [Kind::Net, Kind::Parts, Kind::Chip];

    fn token(self) -> &'static str {
        match self {
            Kind::Net => "pstxnet",
            Kind::Parts => "pstxprt",
            Kind::Chip => "pstchip",
        }
    }

    fn file_type(self) -> &'static str {
        match self {
            Kind::Net => "EXPANDEDNETLIST",
            Kind::Parts => "EXPANDEDPARTLIST",
            Kind::Chip => "LIBRARY_PARTS",
        }
    }
}

/// The kind by the `FILE_TYPE` line
fn kind_by_contents(src: &Source, path: &str) -> Option<Kind> {
    let head = text::head(src, path, 64)?;
    let head = String::from_utf8_lossy(&head);
    let rest = head.strip_prefix("FILE_TYPE")?;
    let rest = rest.trim_start().strip_prefix('=')?.trim_start();
    Kind::ALL
        .into_iter()
        .find(|k| rest.starts_with(k.file_type()))
}

/// The kind by the file name, and where in the name its token is
fn kind_by_name(path: &str) -> Option<(Kind, usize)> {
    let lower = file_name(path).to_ascii_lowercase();
    Kind::ALL
        .into_iter()
        .find_map(|k| lower.find(k.token()).map(|i| (k, i)))
}

/// The three files: the main one and the others named like it
fn trio(src: &Source) -> HashMap<Kind, String> {
    let mut out = HashMap::new();
    let main_kind = kind_by_contents(src, &src.main)
        .or_else(|| kind_by_name(&src.main).map(|(k, _)| k))
        .unwrap_or(Kind::Net);
    out.insert(main_kind, src.main.clone());
    if let Some((k, i)) = kind_by_name(&src.main)
        && k == main_kind
    {
        let dir = dir_of(&src.main);
        let name = file_name(&src.main);
        let token_case = &name[i..i + k.token().len()];
        for other in Kind::ALL.into_iter().filter(|o| *o != k) {
            let t = if token_case.chars().all(|c| c.is_ascii_uppercase()) {
                other.token().to_ascii_uppercase()
            } else {
                other.token().to_string()
            };
            let candidate = format!("{dir}{}{t}{}", &name[..i], &name[i + k.token().len()..]);
            if src.files.read(&candidate).is_some() {
                out.insert(other, candidate);
            }
        }
    }
    out
}

impl Importer for OrcadImporter {
    fn name(&self) -> &'static str {
        "orcad"
    }

    fn sniff(&self, src: &Source) -> Confidence {
        if kind_by_contents(src, &src.main).is_some() {
            Confidence::Yes
        } else if kind_by_name(&src.main).is_some()
            && src.main.to_ascii_lowercase().ends_with(".dat")
            && src.files.read(&src.main).is_some()
        {
            Confidence::Maybe
        } else {
            Confidence::No
        }
    }

    fn import(&self, src: &Source, _opts: &ImportOptions) -> Result<Imported, ImportError> {
        let files = trio(src);
        let mut notes = Notes::default();
        let read = |k: Kind| -> Result<Option<(String, String)>, ImportError> {
            match files.get(&k) {
                Some(f) => Ok(Some((f.clone(), text::read(src, f)?))),
                None => Ok(None),
            }
        };
        let invalid = |file: &str, e: String| ImportError::Invalid {
            file: file.to_string(),
            message: e,
        };
        let chip = match read(Kind::Chip)? {
            Some((f, t)) => Some((parse_chip(&t).map_err(|e| invalid(&f, e))?, f)),
            None => None,
        };
        let parts = match read(Kind::Parts)? {
            Some((f, t)) => Some((parse_parts(&t).map_err(|e| invalid(&f, e))?, f)),
            None => None,
        };
        let nets = match read(Kind::Net)? {
            Some((f, t)) => Some((parse_nets(&t).map_err(|e| invalid(&f, e))?, f)),
            None => None,
        };
        for k in Kind::ALL {
            if !files.contains_key(&k) {
                let what = match k {
                    Kind::Net => "no nets",
                    Kind::Parts => "no part list: no sections, pages or primitive per part",
                    Kind::Chip => "no library parts: no pin types, values or footprints",
                };
                notes.missing(&src.main, "", format!("no {} beside it: {what}", k.token()));
            }
        }
        let name = parts
            .as_ref()
            .and_then(|(p, _)| p.directives.iter().find(|(k, _)| k == "ROOT_DRAWING"))
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| {
                let n = file_name(&src.main);
                n.rsplit_once('.').map_or(n, |(s, _)| s).to_string()
            });
        let mut b = Builder::new(&name, "orcad");
        let comments = [
            chip.as_ref().map(|c| &c.0.comment),
            parts.as_ref().map(|c| &c.0.comment),
            nets.as_ref().map(|c| &c.0.comment),
        ];
        b.b.source.version = comments
            .into_iter()
            .flatten()
            .find_map(|c| writer_version(c));
        let mut f = Fill {
            b: &mut b,
            notes: &mut notes,
            pin_types: HashMap::new(),
            instances: HashMap::new(),
        };
        if let Some((c, file)) = &chip {
            f.b.b.source.files.push(file.clone());
            f.chip(c, file);
        }
        if let Some((p, file)) = &parts {
            f.b.b.source.files.push(file.clone());
            f.parts(p, file, chip.as_ref());
        }
        if let Some((n, file)) = &nets {
            f.b.b.source.files.push(file.clone());
            f.nets(n, file);
        }
        if let Some((_, file)) = &chip {
            f.symbol_pins(file);
        }
        Ok(Imported {
            design: Design::single(b.finish()),
            notes: notes.into_vec(),
        })
    }
}

/// `Using PSTWRITER 17.4.0 d001Feb-26-2025 at 16:06:09` → `PSTWRITER 17.4.0`
fn writer_version(comment: &str) -> Option<String> {
    let rest = comment.trim().strip_prefix("Using ")?;
    let mut it = rest.split_whitespace();
    let tool = it.next()?;
    let version = it.next()?;
    Some(format!("{tool} {version}"))
}

// ---------------------------------------------------------------------------------------------------------------
// The three parsers: text into plain structs

struct ChipFile {
    comment: String,
    primitives: Vec<Primitive>,
}

struct Primitive {
    name: String,
    pins: Vec<(String, Vec<(String, String)>)>,
    body: Vec<(String, String)>,
}

struct PartsFile {
    comment: String,
    directives: Vec<(String, String)>,
    parts: Vec<PartEntry>,
}

struct PartEntry {
    designator: String,
    primitive: String,
    sections: Vec<Section>,
}

struct Section {
    number: String,
    instance: String,
    p_path: String,
    section: String,
}

struct NetsFile {
    comment: String,
    nets: Vec<NetEntry>,
}

struct NetEntry {
    name: String,
    nodes: Vec<Node>,
}

struct Node {
    designator: String,
    pin: String,
    instance: String,
    pin_name: String,
}

/// The text inside a quoted value pair (`'x'` → `x`)
macro_rules! inner {
    ($p:expr) => {
        $p.into_inner()
            .next()
            .map(|s| s.as_str().to_string())
            .unwrap_or_default()
    };
}

fn parse_chip(text: &str) -> Result<ChipFile, String> {
    use chip::Rule;
    let file = chip::P::parse(Rule::file, text)
        .map_err(|e| e.to_string())?
        .next()
        .ok_or("empty")?;
    let mut out = ChipFile {
        comment: String::new(),
        primitives: Vec::new(),
    };
    let param = |p: pest::iterators::Pair<Rule>| {
        let mut it = p.into_inner();
        let k = it
            .next()
            .map(|s| s.as_str().to_string())
            .unwrap_or_default();
        let v = it.next().map(|v| inner!(v)).unwrap_or_default();
        (k, v)
    };
    for p in file.into_inner() {
        match p.as_rule() {
            Rule::exporter_comment => out.comment = inner!(p),
            Rule::primitive => {
                let mut prim = Primitive {
                    name: String::new(),
                    pins: Vec::new(),
                    body: Vec::new(),
                };
                for c in p.into_inner() {
                    match c.as_rule() {
                        Rule::primitive_name => prim.name = inner!(c),
                        Rule::pins => {
                            for pin in c.into_inner() {
                                let mut it = pin.into_inner();
                                let name = it.next().map(|n| inner!(n)).unwrap_or_default();
                                prim.pins.push((name, it.map(param).collect()));
                            }
                        }
                        Rule::body => prim.body = c.into_inner().map(param).collect(),
                        _ => {}
                    }
                }
                out.primitives.push(prim);
            }
            _ => {}
        }
    }
    Ok(out)
}

fn parse_parts(text: &str) -> Result<PartsFile, String> {
    use parts::Rule;
    let file = parts::P::parse(Rule::file, text)
        .map_err(|e| e.to_string())?
        .next()
        .ok_or("empty")?;
    let mut out = PartsFile {
        comment: String::new(),
        directives: Vec::new(),
        parts: Vec::new(),
    };
    for p in file.into_inner() {
        match p.as_rule() {
            Rule::exporter_comment => out.comment = inner!(p),
            Rule::directives => {
                for d in p.into_inner() {
                    let mut it = d.into_inner();
                    let k = it
                        .next()
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_default();
                    let v = it.next().map(|v| inner!(v)).unwrap_or_default();
                    out.directives.push((k, v));
                }
            }
            Rule::parts => {
                for part in p.into_inner() {
                    let mut e = PartEntry {
                        designator: String::new(),
                        primitive: String::new(),
                        sections: Vec::new(),
                    };
                    for c in part.into_inner() {
                        match c.as_rule() {
                            Rule::designator => e.designator = c.as_str().to_string(),
                            Rule::primitive_name => e.primitive = inner!(c),
                            Rule::sections => {
                                for s in c.into_inner() {
                                    let mut sec = Section {
                                        number: String::new(),
                                        instance: String::new(),
                                        p_path: String::new(),
                                        section: String::new(),
                                    };
                                    let mut values = 0;
                                    for x in s.into_inner() {
                                        match x.as_rule() {
                                            Rule::section_number => sec.number = x.as_str().into(),
                                            Rule::instance_name => sec.instance = inner!(x),
                                            Rule::p_path => sec.p_path = inner!(x),
                                            // PRIM_FILE first, SECTION second
                                            Rule::param_value => {
                                                values += 1;
                                                if values == 2 {
                                                    sec.section = inner!(x);
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    e.sections.push(sec);
                                }
                            }
                            _ => {}
                        }
                    }
                    out.parts.push(e);
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

fn parse_nets(text: &str) -> Result<NetsFile, String> {
    use nets::Rule;
    let file = nets::P::parse(Rule::file, text)
        .map_err(|e| e.to_string())?
        .next()
        .ok_or("empty")?;
    let mut out = NetsFile {
        comment: String::new(),
        nets: Vec::new(),
    };
    for p in file.into_inner() {
        match p.as_rule() {
            Rule::exporter_comment => out.comment = inner!(p),
            Rule::net => {
                let mut net = NetEntry {
                    name: String::new(),
                    nodes: Vec::new(),
                };
                for c in p.into_inner() {
                    match c.as_rule() {
                        Rule::net_name => net.name = inner!(c),
                        Rule::node => {
                            let mut n = Node {
                                designator: String::new(),
                                pin: String::new(),
                                instance: String::new(),
                                pin_name: String::new(),
                            };
                            for x in c.into_inner() {
                                match x.as_rule() {
                                    Rule::designator => n.designator = x.as_str().into(),
                                    Rule::pin_id => n.pin = x.as_str().into(),
                                    Rule::instance_name => n.instance = inner!(x),
                                    Rule::pin_name => n.pin_name = inner!(x),
                                    _ => {}
                                }
                            }
                            net.nodes.push(n);
                        }
                        _ => {}
                    }
                }
                out.nets.push(net);
            }
            _ => {}
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------------------------
// Into the board

/// The pin parameters the importer reads; others are noted once
const PIN_PARAMS: &[&str] = &[
    "PIN_NUMBER",
    "PINUSE",
    "INPUT_LOAD",
    "OUTPUT_LOAD",
    "BIDIRECTIONAL",
    "OUTPUT_TYPE",
];

/// The electrical type of a pstchip pin, and the parameters that said so
fn pin_type(params: &[(String, String)]) -> (EcadPinType, String) {
    let get = |k: &str| params.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    if get("BIDIRECTIONAL") == Some("TRUE") {
        return (EcadPinType::Bidirectional, "BIDIRECTIONAL=TRUE".into());
    }
    let pinuse = get("PINUSE");
    let by_use = match pinuse {
        Some("IN") => Some(EcadPinType::Input),
        Some("OUT") => Some(EcadPinType::Output),
        Some("BI") => Some(EcadPinType::Bidirectional),
        Some("TRI") => Some(EcadPinType::TriState),
        Some("OCA" | "OCL") => Some(EcadPinType::OpenCollector),
        Some("OEA" | "OEL") => Some(EcadPinType::OpenEmitter),
        Some("POWER" | "GROUND") => Some(EcadPinType::PowerIn),
        Some("NC") => Some(EcadPinType::NoConnect),
        _ => None,
    };
    if let (Some(t), Some(u)) = (by_use, pinuse) {
        return (t, format!("PINUSE={u}"));
    }
    if get("OUTPUT_LOAD").is_some() {
        return match get("OUTPUT_TYPE") {
            Some(t @ "(OC,AND)") => (EcadPinType::OpenCollector, format!("OUTPUT_TYPE={t}")),
            Some(t @ "(OE,OR)") => (EcadPinType::OpenEmitter, format!("OUTPUT_TYPE={t}")),
            _ => (EcadPinType::Output, "OUTPUT_LOAD".into()),
        };
    }
    if get("INPUT_LOAD").is_some() {
        return (EcadPinType::Input, "INPUT_LOAD".into());
    }
    // Capture's passive pins come out as PINUSE=UNSPEC
    (
        EcadPinType::Passive,
        format!("PINUSE={}", pinuse.unwrap_or("none")),
    )
}

/// `@pcba.schematic1(sch_1):page3_ins11643525@...` → `page3`
fn page_of(p_path: &str) -> Option<String> {
    let after = p_path.split_once(':')?.1;
    let page = after.split('_').next()?;
    let digits = page.strip_prefix("page")?;
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())).then(|| page.to_string())
}

struct Fill<'a> {
    b: &'a mut Builder,
    notes: &'a mut Notes,
    /// (primitive, pin number, unit) → type and why
    pin_types: HashMap<(String, String, u32), (EcadPinType, String)>,
    /// Section instance path → (part, unit)
    instances: HashMap<String, (PartId, u32)>,
}

impl Fill<'_> {
    fn chip(&mut self, c: &ChipFile, file: &str) {
        for prim in &c.primitives {
            let at = format!("primitive {}", prim.name);
            let mut s = SymbolDef {
                name: prim.name.clone(),
                units: 1,
                fields: prim.body.clone(),
                ..SymbolDef::default()
            };
            for (name, params) in &prim.pins {
                for (k, _) in params {
                    if !PIN_PARAMS.contains(&k.as_str()) {
                        self.notes
                            .ignored(file, &at, format!("pin property `{k}` is not kept"));
                    }
                }
                let Some((_, number)) = params.iter().find(|(k, _)| k == "PIN_NUMBER") else {
                    self.notes
                        .missing(file, &at, format!("pin `{name}` has no PIN_NUMBER"));
                    continue;
                };
                let (etype, why) = pin_type(params);
                let list = number.trim_start_matches('(').trim_end_matches(')');
                let entries: Vec<&str> = list.split(',').map(str::trim).collect();
                let multi = entries.len() > 1;
                s.units = s.units.max(entries.len() as u32);
                for (i, n) in entries.iter().enumerate() {
                    if n.is_empty() || (multi && *n == "0") {
                        continue;
                    }
                    let unit = if multi { i as u32 + 1 } else { 0 };
                    s.pins.push(PinDef {
                        number: n.to_string(),
                        name: name.clone(),
                        unit,
                        body_style: 0,
                        etype,
                        style: String::new(),
                        hidden: false,
                        alternates: Vec::new(),
                    });
                    self.pin_types.insert(
                        (prim.name.clone(), n.to_string(), unit),
                        (etype, why.clone()),
                    );
                }
            }
            if self.b.add_symbol(s).is_none() {
                self.notes.invalid(
                    file,
                    &at,
                    "a second primitive with this name; the first is kept",
                );
            }
        }
    }

    fn parts(&mut self, p: &PartsFile, file: &str, chip: Option<&(ChipFile, String)>) {
        for (k, _) in &p.directives {
            if k != "ROOT_DRAWING" {
                self.notes
                    .ignored(file, "DIRECTIVES", format!("directive `{k}` is not kept"));
            }
        }
        if p.parts.iter().any(|e| !e.sections.is_empty()) {
            self.notes.ignored(
                file,
                "SECTION_NUMBER",
                "C_PATH and PRIM_FILE are not kept (the P_PATH gives the page)",
            );
        }
        for e in &p.parts {
            let at = format!("part {}", e.designator);
            if self.b.find_part(&e.designator).is_some() {
                self.notes.invalid(
                    file,
                    &at,
                    "a second part with this designator; the first is kept",
                );
                continue;
            }
            let part = self.b.part(&e.designator);
            let mut symbol = self.b.symbol(&e.primitive);
            if symbol.is_none() {
                if chip.is_some() {
                    self.notes.missing(
                        file,
                        &at,
                        format!(
                            "primitive `{}` is not in pstchip: its symbol has no pins",
                            e.primitive
                        ),
                    );
                }
                // What the part list says of the primitive: its name and sections
                symbol = self.b.add_symbol(SymbolDef {
                    name: e.primitive.clone(),
                    units: e.sections.len().max(1) as u32,
                    ..SymbolDef::default()
                });
            }
            let mut units = Vec::new();
            for s in &e.sections {
                let unit: u32 = match s.number.parse() {
                    Ok(u) => u,
                    Err(_) => {
                        self.notes
                            .invalid(file, &at, format!("section number `{}`", s.number));
                        continue;
                    }
                };
                let sheet = match page_of(&s.p_path) {
                    Some(page) => {
                        let number = page.trim_start_matches("page").to_string();
                        self.b.sheet(&page, Some(number))
                    }
                    None => {
                        self.notes.invalid(
                            file,
                            &at,
                            format!("no page in P_PATH `{}`; placed on the root", s.p_path),
                        );
                        self.b.root()
                    }
                };
                if let Some(sym) = symbol
                    && !s.section.is_empty()
                {
                    let names = &mut self.b.b.symbols[sym.index()].unit_names;
                    if !names.iter().any(|(u, _)| *u == unit) {
                        names.push((unit, s.section.clone()));
                    }
                }
                self.instances.insert(s.instance.clone(), (part, unit));
                units.push(UnitPlacement {
                    unit,
                    sheet,
                    uuid: s.instance.clone(),
                    at: None,
                });
            }
            let p = self.b.part_mut(part);
            p.lib_id = e.primitive.clone();
            p.symbol = symbol;
            if let Some(u) = units.first() {
                p.sheet = u.sheet;
            }
            p.units = units;
            if let Some((c, chip_file)) = chip
                && let Some(prim) = c.primitives.iter().find(|x| x.name == e.primitive)
            {
                let src = Provenance::Netlist {
                    file: chip_file.clone(),
                };
                for (k, v) in &prim.body {
                    let (name, note) = match k.as_str() {
                        "VALUE" => ("Value", Some("pstchip VALUE")),
                        "JEDEC_TYPE" => ("Footprint", Some("pstchip JEDEC_TYPE")),
                        _ => (k.as_str(), None),
                    };
                    let mut fact = Fact::new(v.clone(), src.clone());
                    if let Some(n) = note {
                        fact = fact.with_note(n);
                    }
                    self.b.field(part, name, fact);
                }
                if let Some((_, v)) = prim.body.iter().find(|(k, _)| k == "VALUE") {
                    self.b.value(part, v, &src, "VALUE property");
                }
            }
        }
    }

    fn nets(&mut self, n: &NetsFile, file: &str) {
        if !n.nets.is_empty() {
            self.notes.ignored(
                file,
                "NET_NAME",
                "full net names and C_SIGNAL are not kept (the short name is the net)",
            );
        }
        let mut seen = std::collections::HashSet::new();
        for net in &n.nets {
            let at = format!("net {}", net.name);
            if !seen.insert(net.name.clone()) {
                self.notes.invalid(
                    file,
                    &at,
                    "a second net with this name; its nodes join the first",
                );
            }
            let auto = net
                .name
                .strip_prefix('N')
                .is_some_and(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()));
            let id = self
                .b
                .net(&net.name, if auto { NetKind::Auto } else { NetKind::Named });
            for node in &net.nodes {
                let part = match self.b.find_part(&node.designator) {
                    Some(p) => p,
                    None => {
                        self.notes.invalid(
                            file,
                            &at,
                            format!(
                                "node {}.{}: not in the part list, added without fields",
                                node.designator, node.pin
                            ),
                        );
                        self.b.part(&node.designator)
                    }
                };
                let pin = self.b.pin(part, &node.pin);
                let unit = self.instances.get(&node.instance).map(|(_, u)| *u);
                let p = self.b.pin_mut(pin);
                if p.name.is_empty() {
                    p.name = node.pin_name.clone();
                }
                if let Some(u) = unit {
                    p.unit = u;
                }
                self.b.connect(pin, id, file, self.notes);
            }
        }
    }

    /// Every pin of every part's primitive, with its type from pstchip
    fn symbol_pins(&mut self, chip_file: &str) {
        let src = Provenance::Netlist {
            file: chip_file.to_string(),
        };
        for i in 0..self.b.b.parts.len() {
            let part = PartId::from_index(i);
            let Some(sym) = self.b.b.parts[i].symbol else {
                continue;
            };
            let prim = self.b.b.symbols[sym.index()].name.clone();
            let types = &self.pin_types;
            self.b.symbol_pins(part, |d| {
                types
                    .get(&(prim.clone(), d.number.clone(), d.unit))
                    .map(|(t, why)| Fact::new(*t, src.clone()).with_note(why.clone()))
            });
        }
    }
}
