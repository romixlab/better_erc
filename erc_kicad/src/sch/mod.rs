//! The KiCad schematic hierarchy into a [`Board`]: the root sheet and its sub-sheets as sheet instances (a file
//! used twice is two instances, each with its own references), parts with their units, fields, flags and
//! KiCad 10 variants, pin instances, nets built by `connect` with KiCad's rules, labels, power symbols, sheet
//! pins and no-connect flags.

pub(crate) mod bus;
pub(crate) mod connect;
pub(crate) mod parse;

use crate::notes::Notes;
use crate::sexpr::{self, Sexp};
use connect::{Item, NameKind};
use erc_model::*;
use parse::{SchFile, SchLabelKind, VariantOverride};
use std::collections::HashMap;

/// A sheet instance: one use of a sheet file in the hierarchy
#[derive(Debug, Clone)]
pub(crate) struct Inst {
    /// Index into the files read
    pub file: usize,
    pub parent: Option<usize>,
    /// The sheet symbol in the parent's file
    pub sheet_in_parent: Option<usize>,
    /// Symbol instance path: `/root-uuid/sheet-uuid/...`
    pub key: String,
    /// The netlist's sheet path of UUIDs: `/` for the root, `/sheet-uuid/...`
    pub uuid_path: String,
    /// Human path with a trailing slash: `/`, `/MCU/`
    pub path: String,
    pub name: String,
    pub page: Option<String>,
    pub depth: usize,
    pub on_board: bool,
    pub in_bom: bool,
    pub dnp: bool,
}

/// A placed symbol resolved for one sheet instance
#[derive(Debug, Clone, Default)]
pub(crate) struct SymRes {
    pub reference: String,
    pub unit: u32,
    pub on_board: Option<bool>,
    pub variants: Vec<VariantOverride>,
}

/// A net name as KiCad escapes it: `/` becomes `{slash}`, line breaks are dropped
pub(crate) fn escape_netname(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '/' => out.push_str("{slash}"),
            '\n' | '\r' => {}
            c => out.push(c),
        }
    }
    out
}

/// KiCad's escaped text (`{slash}`, `{comma}`, ...) back to plain text; formatting braces (`~{RST}`) stay
pub(crate) fn unescape(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c != '{' {
            out.push(c);
            i += 1;
            continue;
        }
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let mut depth = 1;
        let mut token = String::new();
        let mut j = i + 1;
        let mut terminated = false;
        while j < chars.len() {
            match chars[j] {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                terminated = true;
                break;
            }
            token.push(chars[j]);
            j += 1;
        }
        if !terminated {
            out.push('{');
            out += &unescape(&token);
            break;
        }
        let plain = match token.as_str() {
            _ if matches!(prev, Some('$' | '~' | '^' | '_')) => None,
            "dblquote" => Some("\""),
            "quote" => Some("'"),
            "lt" => Some("<"),
            "gt" => Some(">"),
            "backslash" => Some("\\"),
            "slash" => Some("/"),
            "bar" => Some("|"),
            "comma" => Some(","),
            "colon" => Some(":"),
            "space" => Some(" "),
            "dollar" => Some("$"),
            "tab" => Some("\t"),
            "return" => Some("\n"),
            "brace" => Some("{"),
            _ => None,
        };
        match plain {
            Some(p) => out += p,
            None => {
                out.push('{');
                out += &unescape(&token);
                out.push('}');
            }
        }
        i = j + 1;
    }
    out
}

/// A label's text as KiCad names its net: unescaped, then `/` escaped again
pub(crate) fn label_netname(text: &str) -> String {
    escape_netname(&unescape(text))
}

/// A stable 64-bit FNV-1a hash of an s-expression, as hex
pub(crate) fn hash(s: &Sexp) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut feed = |b: &[u8]| {
        for &x in b {
            h ^= x as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    };
    let mut stack = vec![s];
    while let Some(x) = stack.pop() {
        match x {
            Sexp::List(items) => {
                feed(b"(");
                stack.extend(items.iter().rev());
            }
            Sexp::Atom(a) => {
                feed(a.as_bytes());
                feed(b" ");
            }
            Sexp::Str(t) => {
                feed(b"\"");
                feed(t.as_bytes());
                feed(b"\" ");
            }
        }
    }
    format!("{h:016x}")
}

/// `a/b/../c.kicad_sch` relative to the directory of `base`
fn join(base: &str, rel: &str) -> String {
    let rel = rel.replace('\\', "/");
    let mut parts: Vec<&str> = match base.rfind('/') {
        Some(i) if !rel.starts_with('/') => base[..i].split('/').collect(),
        _ => Vec::new(),
    };
    for p in rel.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    parts.join("/")
}

/// Pin numbers of a stacked pin (`[1,2,5-7]`), or the number itself
pub(crate) fn stacked_numbers(number: &str) -> Vec<String> {
    let Some(inner) = number.strip_prefix('[').and_then(|n| n.strip_suffix(']')) else {
        return vec![number.to_string()];
    };
    let mut out = Vec::new();
    for part in inner.split(',') {
        let part = part.trim();
        if let Some((a, b)) = part.split_once('-')
            && let (Ok(a), Ok(b)) = (a.parse::<u64>(), b.parse::<u64>())
            && a <= b
            && b - a < 10_000
        {
            out.extend((a..=b).map(|n| n.to_string()));
        } else if !part.is_empty() {
            out.push(part.to_string());
        }
    }
    if out.is_empty() {
        vec![number.to_string()]
    } else {
        out
    }
}

/// The KiCad major version that writes a schematic format version
pub fn kicad_of_format(format: u32) -> u32 {
    if format <= 20231120 {
        8
    } else if format <= 20250114 {
        9
    } else {
        10
    }
}

/// Reads each sheet file once
struct Loader<'a> {
    files: &'a dyn Files,
    sch: Vec<SchFile>,
    names: Vec<String>,
    index: HashMap<String, usize>,
    notes: Vec<ImportNote>,
}

impl Loader<'_> {
    fn load(&mut self, path: &str) -> Result<usize, ImportError> {
        if let Some(&i) = self.index.get(path) {
            return Ok(i);
        }
        let bytes = self
            .files
            .read(path)
            .ok_or_else(|| ImportError::Missing(path.to_string()))?;
        let text = String::from_utf8(bytes).map_err(|_| ImportError::Invalid {
            file: path.into(),
            message: "not UTF-8".into(),
        })?;
        let parsed = sexpr::parse(&text).map_err(|e| ImportError::Invalid {
            file: path.into(),
            message: e.to_string(),
        })?;
        let root = match parsed.as_slice() {
            [r] if r.head() == Some("kicad_sch") => r,
            _ => {
                return Err(ImportError::Invalid {
                    file: path.into(),
                    message: "not a KiCad schematic: no `(kicad_sch ...)`".into(),
                });
            }
        };
        let mut notes = Notes::new(path);
        let f = parse::read(root, &mut notes);
        self.notes.extend(notes.into_vec());
        self.sch.push(f);
        self.names.push(path.to_string());
        self.index.insert(path.to_string(), self.sch.len() - 1);
        Ok(self.sch.len() - 1)
    }
}

/// Reads the schematic hierarchy whose root sheet is `root` (a path among `files`). `project_aliases` are the
/// bus aliases of the project file (KiCad 9 and 10 keep them there; KiCad 8 in the sheets, which are read too).
pub fn read(
    files: &dyn Files,
    root: &str,
    opts: &ImportOptions,
    project_aliases: &[(String, Vec<String>)],
) -> Result<(Board, Vec<ImportNote>), ImportError> {
    let mut ld = Loader {
        files,
        sch: Vec::new(),
        names: Vec::new(),
        index: HashMap::new(),
        notes: Vec::new(),
    };
    let mut notes = Notes::new(root);
    let root_file = ld.load(root)?;
    let mut insts = vec![Inst {
        file: root_file,
        parent: None,
        sheet_in_parent: None,
        key: String::new(),
        uuid_path: "/".into(),
        path: "/".into(),
        name: String::new(),
        page: None,
        depth: 0,
        on_board: true,
        in_bom: true,
        dnp: false,
    }];
    insts[0].key = format!("/{}", ld.sch[root_file].uuid);
    // Breadth first, so instances come in hierarchy order
    let mut next = 0;
    while next < insts.len() {
        let parent = insts[next].clone();
        let sheets = ld.sch[parent.file].sheets.clone();
        for (si, sh) in sheets.iter().enumerate() {
            let at = format!("sheet {} in {}", sh.name, ld.names[parent.file]);
            if sh.file.is_empty() {
                continue;
            }
            let path = join(&ld.names[parent.file], &sh.file);
            // A sheet that contains itself, directly or further down, would never end
            let mut cur = Some(next);
            let mut cycle = false;
            while let Some(c) = cur {
                if ld.names[insts[c].file] == path {
                    cycle = true;
                }
                cur = insts[c].parent;
            }
            if cycle || parent.depth >= 64 {
                notes.invalid(&at, "the sheet contains itself; not read again");
                continue;
            }
            let file = match ld.load(&path) {
                Ok(f) => f,
                Err(e) => {
                    notes.missing(&at, &e.to_string());
                    continue;
                }
            };
            let page = sh
                .instances
                .iter()
                .find(|(p, _)| *p == parent.key)
                .and_then(|(_, page)| page.clone());
            let uuid_path = if parent.uuid_path == "/" {
                format!("/{}", sh.uuid)
            } else {
                format!("{}/{}", parent.uuid_path, sh.uuid)
            };
            insts.push(Inst {
                file,
                parent: Some(next),
                sheet_in_parent: Some(si),
                key: format!("{}/{}", parent.key, sh.uuid),
                uuid_path,
                path: format!("{}{}/", parent.path, escape_netname(&sh.name)),
                name: sh.name.clone(),
                page,
                depth: parent.depth + 1,
                on_board: parent.on_board && sh.on_board.unwrap_or(true),
                in_bom: parent.in_bom && sh.in_bom.unwrap_or(true),
                dnp: parent.dnp || sh.dnp.unwrap_or(false),
            });
        }
        next += 1;
    }

    // Each placed symbol's reference and unit in each instance
    let project = root
        .rsplit('/')
        .next()
        .unwrap_or(root)
        .trim_end_matches(".kicad_sch")
        .to_string();
    let Loader {
        sch,
        names,
        notes: mut all_notes,
        ..
    } = ld;
    let mut syms: Vec<Vec<SymRes>> = Vec::new();
    for inst in &insts {
        let f = &sch[inst.file];
        let mut list = Vec::new();
        for s in &f.symbols {
            let found = s
                .instances
                .iter()
                .filter(|i| i.path == inst.key)
                .max_by_key(|i| i.project == project);
            let res = match found {
                Some(i) => SymRes {
                    reference: i.reference.clone(),
                    unit: i.unit.unwrap_or(s.unit),
                    on_board: s.on_board,
                    variants: i.variants.clone(),
                },
                None => {
                    let reference = s.property("Reference").unwrap_or("?").to_string();
                    if !reference.starts_with('#') {
                        notes.missing(
                            &format!("symbol {reference} in {}", names[inst.file]),
                            &format!(
                                "no instance for sheet path {}; the Reference field is used",
                                inst.key
                            ),
                        );
                    }
                    SymRes {
                        reference,
                        unit: s.unit,
                        on_board: s.on_board,
                        variants: Vec::new(),
                    }
                }
            };
            list.push(res);
        }
        syms.push(list);
    }

    let kicad = opts.kicad_version.unwrap_or_else(|| {
        sch.iter()
            .filter_map(|f| f.version.as_deref()?.parse::<u32>().ok())
            .max()
            .map_or(10, kicad_of_format)
    });
    let mut aliases: HashMap<String, Vec<String>> = HashMap::new();
    for (name, members) in sch
        .iter()
        .flat_map(|f| &f.bus_aliases)
        .chain(project_aliases)
    {
        aliases.insert(name.clone(), members.clone());
    }
    let conn = connect::build(&connect::Input {
        files: &sch,
        insts: &insts,
        syms: &syms,
        kicad,
        aliases: &aliases,
    });
    let board = assemble(&sch, &names, &insts, &syms, &conn, &project, &mut notes);
    all_notes.extend(notes.into_vec());
    Ok((board, all_notes))
}

/// The board from the files, instances and connectivity
fn assemble(
    sch: &[SchFile],
    names: &[String],
    insts: &[Inst],
    syms: &[Vec<SymRes>],
    conn: &connect::Connectivity,
    project: &str,
    notes: &mut Notes,
) -> Board {
    let root = &sch[insts[0].file];
    let mut b = Board {
        name: project.to_string(),
        source: SourceInfo {
            tool: "kicad".into(),
            version: root.generator_version.clone().or(root.version.clone()),
            files: names.to_vec(),
        },
        ..Board::default()
    };
    for inst in insts {
        b.sheets.push(Sheet {
            name: inst.name.clone(),
            file: names[inst.file].clone(),
            parent: inst.parent.map(SheetId::from_index),
            uuid_path: inst.uuid_path.clone(),
            page: inst.page.clone(),
        });
    }
    // Library symbols, the first copy of each name
    let mut symbol_ids: HashMap<String, SymbolId> = HashMap::new();
    for f in sch {
        let mut keys: Vec<&String> = f.lib_symbols.keys().collect();
        keys.sort();
        for k in keys {
            if symbol_ids.contains_key(k) {
                continue;
            }
            let l = &f.lib_symbols[k];
            let mut pins = l.pins.clone();
            if pins.is_empty()
                && let Some(parent) = l.extends.as_ref().and_then(|e| {
                    let lib = k.split_once(':').map_or("", |(lib, _)| lib);
                    f.lib_symbols
                        .get(&format!("{lib}:{e}"))
                        .or_else(|| f.lib_symbols.get(e))
                })
            {
                pins = parent.pins.clone();
            }
            symbol_ids.insert(k.clone(), SymbolId::from_index(b.symbols.len()));
            b.symbols.push(SymbolDef {
                name: k.clone(),
                description: l.description.clone(),
                keywords: l.keywords.clone(),
                footprint_filters: l.footprint_filters.clone(),
                power: l.power,
                units: l.units,
                pins: pins
                    .iter()
                    .map(|p| PinDef {
                        number: p.number.clone(),
                        name: p.name.clone(),
                        unit: p.unit,
                        body_style: p.body_style,
                        etype: p.etype,
                        style: p.style.clone(),
                        hidden: p.hidden,
                        alternates: p.alternates.clone(),
                    })
                    .collect(),
                fields: l.fields.clone(),
                duplicate_pins_are_jumpers: l.duplicate_pins_are_jumpers,
                jumper_pin_groups: l.jumper_pin_groups.clone(),
                hash: Some(l.hash.clone()),
            });
        }
    }
    for (name, kind) in &conn.nets {
        b.nets.push(Net {
            name: name.clone(),
            kind: match kind {
                NameKind::Label => NetKind::Named,
                NameKind::Power => NetKind::Power,
                NameKind::Pin if name.starts_with("unconnected-(") => NetKind::Unconnected,
                NameKind::Pin => NetKind::Auto,
            },
            class: Facts::default(),
            pins: Vec::new(),
            labels: Vec::new(),
        });
    }

    let mut variants: HashMap<String, VariantId> = HashMap::new();
    let mut parts: HashMap<String, PartId> = HashMap::new();
    // (part, pin number) → pin
    let mut pins: HashMap<(PartId, String), PinId> = HashMap::new();
    for (ii, inst) in insts.iter().enumerate() {
        let f = &sch[inst.file];
        let file = &names[inst.file];
        let src = Provenance::Schematic { file: file.clone() };
        let fact = |v: String| Fact::new(v, src.clone());
        let sheet = SheetId::from_index(ii);
        let nets = &conn.insts[ii];
        for (si, s) in f.symbols.iter().enumerate() {
            let res = &syms[ii][si];
            let lib = f.lib_symbols.get(s.lib_key());
            if lib.is_none() {
                notes.missing(
                    &format!("symbol {} in {file}", res.reference),
                    &format!("no library symbol `{}` in lib_symbols", s.lib_key()),
                );
            }
            let unit_pins = connect::unit_pins(f, si, res.unit);
            // A power symbol is not a part: its pin names a net
            if lib.is_some_and(|l| l.power.is_some()) {
                let net = unit_pins
                    .first()
                    .and_then(|(p, _)| nets.item_net.get(&Item::Pin(si, *p)))
                    .map(|&n| NetId::from_index(n));
                let dangling = unit_pins
                    .first()
                    .is_some_and(|(p, _)| nets.alone.contains(&Item::Pin(si, *p)));
                b.labels.push(Label {
                    kind: LabelKind::Power,
                    text: s.property("Value").unwrap_or("").to_string(),
                    sheet,
                    at: s.at,
                    shape: None,
                    child: None,
                    fields: Vec::new(),
                    net,
                    dangling,
                });
                continue;
            }
            let placement = UnitPlacement {
                unit: res.unit,
                sheet,
                uuid: s.uuid.clone(),
                at: Some(Placement {
                    at: s.at,
                    rotation: s.rotation as f64,
                    mirror: s.mirror,
                }),
            };
            let unannotated = res.reference.ends_with('?');
            let existing = if unannotated {
                None
            } else {
                parts.get(&res.reference).copied()
            };
            let part = match existing {
                Some(p) => {
                    let part = &mut b.parts[p.index()];
                    if part.lib_id != s.lib_id {
                        notes.invalid(
                            &format!("symbol {} in {file}", res.reference),
                            &format!(
                                "reference also used by a `{}`; joined as one part",
                                part.lib_id
                            ),
                        );
                    }
                    part.units.push(placement);
                    p
                }
                None => {
                    let id = PartId::from_index(b.parts.len());
                    if !unannotated {
                        parts.insert(res.reference.clone(), id);
                    }
                    let mut fields = Vec::new();
                    for (k, v) in &s.properties {
                        if k == "Reference" || v.is_empty() {
                            continue;
                        }
                        if !fields.iter().any(|f: &Field| f.name == *k) {
                            fields.push(Field::new(k.clone(), fact(v.clone())));
                        }
                    }
                    let value = s.property("Value").unwrap_or("");
                    let mut flags = PartFlags::default();
                    let flag =
                        |v: Option<bool>, inherited: Option<bool>, what: &str| -> Facts<bool> {
                            match (v, inherited) {
                                (_, Some(i)) => Facts::one(
                                    Fact::new(i, src.clone())
                                        .with_note(format!("{what} by its sheet")),
                                ),
                                (Some(v), None) => Facts::one(Fact::new(v, src.clone())),
                                (None, None) => Facts::default(),
                            }
                        };
                    flags.dnp = flag(s.dnp, inst.dnp.then_some(true), "DNP");
                    flags.in_bom = flag(
                        s.in_bom,
                        (!inst.in_bom).then_some(false),
                        "excluded from the BOM",
                    );
                    flags.on_board = flag(
                        s.on_board,
                        (!inst.on_board).then_some(false),
                        "excluded from the board",
                    );
                    flags.exclude_from_sim = flag(s.exclude_from_sim, None, "");
                    flags.in_pos_files = flag(s.in_pos_files, None, "");
                    let mut pvs = Vec::new();
                    for v in &res.variants {
                        let vid = *variants.entry(v.name.clone()).or_insert_with(|| {
                            b.variants.push(Variant {
                                name: v.name.clone(),
                                description: None,
                            });
                            VariantId::from_index(b.variants.len() - 1)
                        });
                        let mut pv = PartVariant {
                            variant: vid,
                            flags: PartFlags::default(),
                            fields: Vec::new(),
                        };
                        for (k, on) in &v.flags {
                            let (slot, val) = match k.as_str() {
                                "exclude_from_bom" => (&mut pv.flags.in_bom, !on),
                                "exclude_from_board" => (&mut pv.flags.on_board, !on),
                                k => match pv.flags.by_name_mut(k) {
                                    Some(slot) => (slot, *on),
                                    None => continue,
                                },
                            };
                            slot.set(Fact::new(val, src.clone()));
                        }
                        for (k, val) in &v.fields {
                            pv.fields.push(Field::new(k.clone(), fact(val.clone())));
                        }
                        pvs.push(pv);
                    }
                    b.parts.push(Part {
                        designator: res.reference.clone(),
                        symbol: symbol_ids.get(s.lib_key()).copied(),
                        lib_id: s.lib_id.clone(),
                        sheet,
                        units: vec![placement],
                        fields,
                        value: Facts::one(
                            Fact::new(Value::parse(value).0, src.clone())
                                .with_note("from the Value field"),
                        ),
                        flags,
                        variants: pvs,
                        pins: Vec::new(),
                        ties: lib.map(|l| l.jumper_pin_groups.clone()).unwrap_or_default(),
                    });
                    id
                }
            };
            let Some(lib) = lib else { continue };
            for (pi, _) in &unit_pins {
                let lp = &lib.pins[*pi];
                let item = Item::Pin(si, *pi);
                let net = nets.item_net.get(&item).map(|&n| NetId::from_index(n));
                let alt = s.alternates.get(&lp.number).cloned();
                let etype = alt
                    .as_ref()
                    .and_then(|a| lp.alternates.iter().find(|x| x.name == *a))
                    .map_or(lp.etype, |a| a.etype);
                for number in stacked_numbers(&lp.number) {
                    if let Some(&existing) = pins.get(&(part, number.clone())) {
                        // A pin shared by units, or repeated: one pin, a named net wins over an auto one
                        let pin = &mut b.pins[existing.index()];
                        let auto = |n: Option<NetId>, b: &Board| {
                            n.is_none_or(|n| {
                                matches!(
                                    b.nets[n.index()].kind,
                                    NetKind::Auto | NetKind::Unconnected
                                )
                            })
                        };
                        if net.is_some() && auto(pin.net, &b) && !auto(net, &b) {
                            b.pins[existing.index()].net = net;
                        }
                        continue;
                    }
                    let id = PinId::from_index(b.pins.len());
                    pins.insert((part, number.clone()), id);
                    b.pins.push(PinInst {
                        part,
                        number,
                        name: lp.name.clone(),
                        unit: if lp.unit == 0 { res.unit } else { lp.unit },
                        etype: Facts::one(Fact::new(etype, src.clone())),
                        hidden: lp.hidden,
                        net,
                        no_connect: nets.no_connect.contains(&item),
                        alt: alt.clone(),
                    });
                }
            }
        }
        for (li, l) in f.labels.iter().enumerate() {
            let item = Item::Label(li);
            let kind = match l.kind {
                SchLabelKind::Local => LabelKind::Local,
                SchLabelKind::Global => LabelKind::Global,
                SchLabelKind::Hierarchical => LabelKind::Hierarchical,
                SchLabelKind::Directive => LabelKind::Directive,
            };
            let net = nets.item_net.get(&item).map(|&n| NetId::from_index(n));
            let text = if l.kind == SchLabelKind::Directive && l.text.is_empty() {
                l.fields
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                l.text.clone()
            };
            b.labels.push(Label {
                kind,
                text,
                sheet,
                at: l.at,
                shape: l.shape.clone(),
                child: None,
                fields: l.fields.clone(),
                net,
                dangling: nets.alone.contains(&item),
            });
        }
        for (shi, sh) in f.sheets.iter().enumerate() {
            let child = insts
                .iter()
                .position(|c| c.parent == Some(ii) && c.sheet_in_parent == Some(shi))
                .map(SheetId::from_index);
            for (pi, p) in sh.pins.iter().enumerate() {
                let item = Item::SheetPin(shi, pi);
                b.labels.push(Label {
                    kind: LabelKind::SheetPin,
                    text: p.name.clone(),
                    sheet,
                    at: p.at,
                    shape: Some(p.shape.clone()),
                    child,
                    fields: Vec::new(),
                    net: nets.item_net.get(&item).map(|&n| NetId::from_index(n)),
                    dangling: nets.alone.contains(&item),
                });
            }
        }
        for (ni, at) in f.no_connects.iter().enumerate() {
            let pin = nets
                .nc_pins
                .get(&ni)
                .and_then(|list| match list.as_slice() {
                    [Item::Pin(si, pi)] => {
                        let res = &syms[ii][*si];
                        let part = parts.get(&res.reference)?;
                        let lib = f.lib_symbols.get(f.symbols[*si].lib_key())?;
                        pins.get(&(*part, lib.pins[*pi].number.clone())).copied()
                    }
                    _ => None,
                });
            b.no_connects.push(NoConnect {
                sheet,
                at: *at,
                pin,
            });
        }
    }
    b.finish();
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_stacks() {
        assert_eq!(join("a/b.kicad_sch", "c.kicad_sch"), "a/c.kicad_sch");
        assert_eq!(join("b.kicad_sch", "sub/../c.kicad_sch"), "c.kicad_sch");
        assert_eq!(join("a/b.kicad_sch", "../c.kicad_sch"), "c.kicad_sch");
        assert_eq!(stacked_numbers("[1,3-5]"), vec!["1", "3", "4", "5"]);
        assert_eq!(stacked_numbers("A1"), vec!["A1"]);
        assert_eq!(escape_netname("A/B\n"), "A{slash}B");
        assert_eq!(unescape("~{SYNC}{slash}~{RESET}"), "~{SYNC}/~{RESET}");
        assert_eq!(unescape("A{comma}B{x}"), "A,B{x}");
        assert_eq!(
            label_netname("~{SYNC}{slash}~{RESET}"),
            "~{SYNC}{slash}~{RESET}"
        );
    }
}
