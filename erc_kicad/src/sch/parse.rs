//! One `.kicad_sch` file read into plain structs: library symbols with their pins, placed symbols with their
//! instances, wires, buses, junctions, labels, no-connect flags, bus entries and sheets. Nothing is connected
//! here; [`super::connect`] does that per sheet instance.

use crate::notes::Notes;
use crate::sexpr::{Lines, Sexp};
use erc_model::{EcadPinType, PinAlternate, PowerScope, Pt};
use std::collections::HashMap;

/// A schematic file
#[derive(Debug, Default)]
pub struct SchFile {
    pub version: Option<String>,
    pub generator_version: Option<String>,
    pub uuid: String,
    /// The revision in the title block
    pub rev: Option<String>,
    pub lib_symbols: HashMap<String, LibSymbol>,
    pub symbols: Vec<SchSymbol>,
    pub wires: Vec<(Pt, Pt)>,
    pub buses: Vec<(Pt, Pt)>,
    pub junctions: Vec<Pt>,
    pub no_connects: Vec<Pt>,
    /// Wire-to-bus entries: the two ends
    pub bus_entries: Vec<(Pt, Pt)>,
    pub labels: Vec<SchLabel>,
    pub sheets: Vec<SchSheet>,
    /// Bus aliases (KiCad 8 keeps them in the schematic): name and members
    pub bus_aliases: Vec<(String, Vec<String>)>,
}

/// A library symbol as embedded in `lib_symbols`
#[derive(Debug, Clone, Default)]
pub struct LibSymbol {
    pub name: String,
    pub extends: Option<String>,
    pub power: Option<PowerScope>,
    pub description: String,
    pub keywords: String,
    pub footprint_filters: Vec<String>,
    pub fields: Vec<(String, String)>,
    pub pins: Vec<LibPin>,
    pub units: u32,
    pub duplicate_pins_are_jumpers: bool,
    pub jumper_pin_groups: Vec<Vec<String>>,
    /// Hash of the library copy as written, for LIB-17
    pub hash: String,
}

/// A pin of a library symbol
#[derive(Debug, Clone)]
pub struct LibPin {
    pub number: String,
    pub name: String,
    pub unit: u32,
    pub body_style: u32,
    pub etype: EcadPinType,
    pub style: String,
    pub hidden: bool,
    /// Connection point in library coordinates (y up), nanometres
    pub at: (i64, i64),
    pub alternates: Vec<PinAlternate>,
}

/// A placed symbol
#[derive(Debug, Clone, Default)]
pub struct SchSymbol {
    pub lib_id: String,
    /// Key into `lib_symbols` when it differs from `lib_id` (a modified copy)
    pub lib_name: Option<String>,
    pub at: Pt,
    pub rotation: i32,
    pub mirror: Option<char>,
    pub unit: u32,
    pub body_style: u32,
    pub uuid: String,
    /// The line of `(symbol` in the file
    pub line: Option<u32>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub dnp: Option<bool>,
    pub exclude_from_sim: Option<bool>,
    pub in_pos_files: Option<bool>,
    /// Fields in file order (`Reference`, `Value`, `Footprint`, ...)
    pub properties: Vec<(String, String)>,
    /// Alternate pin functions chosen, by pin number
    pub alternates: HashMap<String, String>,
    pub instances: Vec<SymbolInstance>,
}

impl SchSymbol {
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// The key of its library symbol
    pub fn lib_key(&self) -> &str {
        self.lib_name.as_deref().unwrap_or(&self.lib_id)
    }
}

/// One instance of a placed symbol (per sheet instance path)
#[derive(Debug, Clone, Default)]
pub struct SymbolInstance {
    pub project: String,
    /// `/root-uuid/sheet-uuid/...`
    pub path: String,
    pub reference: String,
    pub unit: Option<u32>,
    pub variants: Vec<VariantOverride>,
}

/// A KiCad 10 variant block of a symbol instance
#[derive(Debug, Clone, Default)]
pub struct VariantOverride {
    pub name: String,
    pub flags: Vec<(String, bool)>,
    pub fields: Vec<(String, String)>,
}

/// Kind of label
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchLabelKind {
    Local,
    Global,
    Hierarchical,
    Directive,
}

/// A label
#[derive(Debug, Clone)]
pub struct SchLabel {
    pub kind: SchLabelKind,
    pub text: String,
    pub at: Pt,
    pub shape: Option<String>,
    pub fields: Vec<(String, String)>,
}

/// A sheet symbol
#[derive(Debug, Clone, Default)]
pub struct SchSheet {
    pub uuid: String,
    pub name: String,
    pub file: String,
    pub pins: Vec<SheetPin>,
    /// `(path, page)` per instance of the parent
    pub instances: Vec<(String, Option<String>)>,
    pub on_board: Option<bool>,
    pub in_bom: Option<bool>,
    pub dnp: Option<bool>,
}

/// A pin of a sheet symbol
#[derive(Debug, Clone)]
pub struct SheetPin {
    pub name: String,
    pub shape: String,
    pub at: Pt,
}

fn pt(item: Option<&Sexp>) -> Option<Pt> {
    let at = item?;
    Some(Pt::from_mm(at.num(0)?, at.num(1)?))
}

fn yes_no(item: &Sexp, head: &str) -> Option<bool> {
    item.child(head)
        .map(|c| !matches!(c.arg(0), Some("no" | "false")))
}

/// `(xy a b) (xy c d)` of `(pts ...)`: the two ends of a line
fn line(item: &Sexp) -> Option<(Pt, Pt)> {
    let pts: Vec<Pt> = item
        .child("pts")?
        .children("xy")
        .filter_map(|xy| Some(Pt::from_mm(xy.num(0)?, xy.num(1)?)))
        .collect();
    match pts.as_slice() {
        [a, b] => Some((*a, *b)),
        _ => None,
    }
}

/// Reads one schematic file
pub fn read(root: &Sexp, lines: &Lines, notes: &mut Notes) -> SchFile {
    let mut f = SchFile::default();
    for item in root.args() {
        let Some(head) = item.head() else {
            notes.unknown("kicad_sch", item);
            continue;
        };
        match head {
            "version" => f.version = item.arg(0).map(String::from),
            "generator_version" => f.generator_version = item.arg(0).map(String::from),
            "uuid" => f.uuid = item.arg(0).unwrap_or("").to_string(),
            "lib_symbols" => {
                for s in item.args() {
                    if s.head() == Some("symbol") {
                        let ls = lib_symbol(s, notes);
                        f.lib_symbols.insert(ls.name.clone(), ls);
                    } else {
                        notes.unknown("lib_symbols", s);
                    }
                }
            }
            "symbol" => f.symbols.push(SchSymbol {
                line: lines.line_of(item),
                ..symbol(item, notes)
            }),
            "wire" => match line(item) {
                Some(l) => f.wires.push(l),
                None => notes.invalid("wire", "not two points"),
            },
            "bus" => match line(item) {
                Some(l) => f.buses.push(l),
                None => notes.invalid("bus", "not two points"),
            },
            "junction" => f.junctions.extend(pt(item.child("at"))),
            "no_connect" => f.no_connects.extend(pt(item.child("at"))),
            "bus_entry" => {
                let at = pt(item.child("at"));
                let size = pt(item.child("size"));
                if let (Some(a), Some(s)) = (at, size) {
                    f.bus_entries.push((a, Pt::new(a.x + s.x, a.y + s.y)));
                } else {
                    notes.invalid("bus_entry", "no position or size");
                }
            }
            "label" | "global_label" | "hierarchical_label" | "directive_label"
            | "netclass_flag" => {
                let kind = match head {
                    "label" => SchLabelKind::Local,
                    "global_label" => SchLabelKind::Global,
                    "hierarchical_label" => SchLabelKind::Hierarchical,
                    _ => SchLabelKind::Directive,
                };
                let Some(at) = pt(item.child("at")) else {
                    notes.invalid(head, "a label without a position");
                    continue;
                };
                let fields = item
                    .children("property")
                    .filter_map(|p| Some((p.arg(0)?.to_string(), p.arg(1)?.to_string())))
                    .collect();
                f.labels.push(SchLabel {
                    kind,
                    text: item.arg(0).unwrap_or("").to_string(),
                    at,
                    shape: item.value("shape").map(String::from),
                    fields,
                });
            }
            "sheet" => f.sheets.push(sheet(item, notes)),
            "bus_alias" => {
                let name = item.arg(0).unwrap_or("").to_string();
                let members = item
                    .child("members")
                    .map(|m| {
                        m.args()
                            .iter()
                            .filter_map(Sexp::text)
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                f.bus_aliases.push((name, members));
            }
            // Drawing, text and bookkeeping that carry no connectivity
            "title_block" => {
                f.rev = item
                    .value("rev")
                    .filter(|r| !r.is_empty())
                    .map(String::from)
            }
            "generator" | "paper" | "text" | "text_box" | "polyline" | "rectangle" | "circle"
            | "arc" | "bezier" | "image" | "table" | "sheet_instances" | "symbol_instances"
            | "embedded_fonts" | "embedded_files" | "group" | "rule_area" => {}
            _ => notes.unknown("kicad_sch", item),
        }
    }
    let format = f.version.as_deref().and_then(|v| v.parse::<u32>().ok());
    if format.is_none_or(|v| v < 20250318) {
        tilde_is_empty(&mut f);
    }
    f
}

/// Before format 20250318 (KiCad 10) a field value, pin name or pin number of `~` means empty; since then `~`
/// is the text itself
fn tilde_is_empty(f: &mut SchFile) {
    let clear = |s: &mut String| {
        if s == "~" {
            s.clear();
        }
    };
    for l in f.lib_symbols.values_mut() {
        l.fields.iter_mut().for_each(|(_, v)| clear(v));
        for p in &mut l.pins {
            clear(&mut p.name);
            clear(&mut p.number);
        }
    }
    for s in &mut f.symbols {
        s.properties.iter_mut().for_each(|(_, v)| clear(v));
        for i in &mut s.instances {
            for v in &mut i.variants {
                v.fields.iter_mut().for_each(|(_, v)| clear(v));
            }
        }
    }
}

/// `NAME_U_B` of a library sub-symbol: unit and body style
fn unit_body(name: &str) -> (u32, u32) {
    let mut it = name.rsplitn(3, '_');
    let b = it.next().and_then(|s| s.parse().ok());
    let u = it.next().and_then(|s| s.parse().ok());
    match (u, b) {
        (Some(u), Some(b)) => (u, b),
        _ => (0, 0),
    }
}

fn lib_symbol(s: &Sexp, notes: &mut Notes) -> LibSymbol {
    let name = s.arg(0).unwrap_or("").to_string();
    let ctx = format!("lib_symbols {name}");
    let mut ls = LibSymbol {
        name: name.clone(),
        units: 1,
        hash: crate::sch::hash(s),
        ..LibSymbol::default()
    };
    let mut max_unit = 0;
    for item in s.args() {
        match item.head() {
            Some("extends") => ls.extends = item.arg(0).map(String::from),
            Some("power") => {
                ls.power = Some(match item.arg(0) {
                    Some("local") => PowerScope::Local,
                    _ => PowerScope::Global,
                })
            }
            Some("property") => {
                let k = item.arg(0).unwrap_or("");
                let v = item.arg(1).unwrap_or("");
                match k {
                    "ki_description" | "Description" => ls.description = v.into(),
                    "ki_keywords" => ls.keywords = v.into(),
                    "ki_fp_filters" => {
                        ls.footprint_filters = v.split_whitespace().map(String::from).collect()
                    }
                    _ => {}
                }
                if !v.is_empty() && !k.starts_with("ki_") {
                    ls.fields.push((k.into(), v.into()));
                }
            }
            Some("duplicate_pin_numbers_are_jumpers") => {
                ls.duplicate_pins_are_jumpers = matches!(item.arg(0), Some("yes"))
            }
            Some("jumper_pin_groups") => {
                for g in item.args() {
                    let pins: Vec<String> = g
                        .items()
                        .iter()
                        .filter_map(Sexp::text)
                        .map(String::from)
                        .collect();
                    if !pins.is_empty() {
                        ls.jumper_pin_groups.push(pins);
                    }
                }
            }
            Some("symbol") => {
                let sub = item.arg(0).unwrap_or("");
                let (unit, body) = unit_body(sub);
                max_unit = max_unit.max(unit);
                for p in item.args() {
                    match p.head() {
                        Some("pin") => ls.pins.push(lib_pin(p, unit, body, &ctx, notes)),
                        Some(
                            "rectangle" | "polyline" | "circle" | "arc" | "bezier" | "text"
                            | "text_box" | "unit_name" | "image",
                        ) => {}
                        _ => {
                            if p.head().is_some() {
                                notes.unknown(&ctx, p);
                            }
                        }
                    }
                }
            }
            Some(
                "pin_numbers" | "pin_names" | "exclude_from_sim" | "in_bom" | "on_board"
                | "in_pos_files" | "embedded_fonts" | "embedded_files" | "body_styles"
                | "unit_name",
            ) => {}
            _ => {
                if item.head().is_some() {
                    notes.unknown(&ctx, item);
                }
            }
        }
    }
    ls.units = max_unit.max(1);
    ls
}

fn pin_type(name: &str, ctx: &str, notes: &mut Notes) -> EcadPinType {
    EcadPinType::from_name(name).unwrap_or_else(|| {
        notes.invalid(ctx, &format!("pin type `{name}`"));
        EcadPinType::Unspecified
    })
}

fn lib_pin(p: &Sexp, unit: u32, body: u32, ctx: &str, notes: &mut Notes) -> LibPin {
    let etype = pin_type(p.arg(0).unwrap_or("unspecified"), ctx, notes);
    let style = p.arg(1).unwrap_or("line").to_string();
    let at = p.child("at");
    let at = (
        at.and_then(|a| a.num(0))
            .map_or(0, erc_model::layout::mm_to_nm),
        at.and_then(|a| a.num(1))
            .map_or(0, erc_model::layout::mm_to_nm),
    );
    // KiCad 8 writes a bare `hide`, KiCad 9 and 10 `(hide yes)`
    let hidden =
        p.args().iter().any(|a| matches!(a, Sexp::Atom("hide"))) || p.flag("hide").unwrap_or(false);
    let alternates = p
        .children("alternate")
        .map(|a| PinAlternate {
            name: a.arg(0).unwrap_or("").to_string(),
            etype: pin_type(a.arg(1).unwrap_or("unspecified"), ctx, notes),
            style: a.arg(2).unwrap_or("line").to_string(),
        })
        .collect();
    LibPin {
        number: p.value("number").unwrap_or("").to_string(),
        name: p.value("name").unwrap_or("").to_string(),
        unit,
        body_style: body,
        etype,
        style,
        hidden,
        at,
        alternates,
    }
}

fn symbol(s: &Sexp, notes: &mut Notes) -> SchSymbol {
    let mut sym = SchSymbol {
        lib_id: s.value("lib_id").unwrap_or("").to_string(),
        lib_name: s.value("lib_name").map(String::from),
        unit: s.value("unit").and_then(|u| u.parse().ok()).unwrap_or(1),
        body_style: s
            .value("body_style")
            .or(s.value("convert"))
            .and_then(|u| u.parse().ok())
            .unwrap_or(1),
        uuid: s.value("uuid").unwrap_or("").to_string(),
        in_bom: yes_no(s, "in_bom"),
        on_board: yes_no(s, "on_board"),
        dnp: yes_no(s, "dnp"),
        exclude_from_sim: yes_no(s, "exclude_from_sim"),
        in_pos_files: yes_no(s, "in_pos_files"),
        ..SchSymbol::default()
    };
    if let Some(at) = s.child("at") {
        sym.at = Pt::from_mm(at.num(0).unwrap_or(0.0), at.num(1).unwrap_or(0.0));
        sym.rotation = at.num(2).unwrap_or(0.0).round() as i32;
    }
    sym.mirror = s.value("mirror").and_then(|m| m.chars().next());
    let ctx = format!("symbol {}", sym.uuid);
    for item in s.args() {
        match item.head() {
            Some("property") => sym.properties.push((
                item.arg(0).unwrap_or("").to_string(),
                item.arg(1).unwrap_or("").to_string(),
            )),
            Some("pin") => {
                if let (Some(n), Some(alt)) = (item.arg(0), item.value("alternate")) {
                    sym.alternates.insert(n.to_string(), alt.to_string());
                }
            }
            Some("instances") => {
                for proj in item.children("project") {
                    let project = proj.arg(0).unwrap_or("").to_string();
                    for path in proj.children("path") {
                        let mut inst = SymbolInstance {
                            project: project.clone(),
                            path: path.arg(0).unwrap_or("").to_string(),
                            reference: path.value("reference").unwrap_or("").to_string(),
                            unit: path.value("unit").and_then(|u| u.parse().ok()),
                            variants: Vec::new(),
                        };
                        for v in path.children("variant") {
                            inst.variants.push(variant(v, &ctx, notes));
                        }
                        sym.instances.push(inst);
                    }
                }
            }
            Some(
                "lib_id" | "lib_name" | "at" | "mirror" | "unit" | "convert" | "body_style"
                | "uuid" | "in_bom" | "on_board" | "dnp" | "exclude_from_sim" | "in_pos_files"
                | "fields_autoplaced" | "default_instance",
            ) => {}
            _ => {
                if item.head().is_some() {
                    notes.unknown(&ctx, item);
                }
            }
        }
    }
    sym
}

fn variant(v: &Sexp, ctx: &str, notes: &mut Notes) -> VariantOverride {
    let mut o = VariantOverride {
        name: v.value("name").unwrap_or("").to_string(),
        ..VariantOverride::default()
    };
    for x in v.args() {
        match x.head() {
            Some("name") => {}
            Some(
                h @ ("dnp" | "in_bom" | "on_board" | "exclude_from_sim" | "in_pos_files"
                | "exclude_from_bom" | "exclude_from_board"),
            ) => {
                let on = !matches!(x.arg(0), Some("no" | "false" | "0"));
                o.flags.push((h.to_string(), on));
            }
            Some("field") => {
                let n = x.value("name").unwrap_or("").to_string();
                let val = x.value("value").unwrap_or("").to_string();
                o.fields.push((n, val));
            }
            _ => notes.unknown(&format!("{ctx} variant {}", o.name), x),
        }
    }
    o
}

fn sheet(s: &Sexp, notes: &mut Notes) -> SchSheet {
    let mut sh = SchSheet {
        uuid: s.value("uuid").unwrap_or("").to_string(),
        on_board: yes_no(s, "on_board"),
        in_bom: yes_no(s, "in_bom"),
        dnp: yes_no(s, "dnp"),
        ..SchSheet::default()
    };
    for item in s.args() {
        match item.head() {
            Some("property") => match item.arg(0) {
                // KiCad 7 and later; "Sheet name" / "Sheet file" before
                Some("Sheetname" | "Sheet name") => sh.name = item.arg(1).unwrap_or("").into(),
                Some("Sheetfile" | "Sheet file") => sh.file = item.arg(1).unwrap_or("").into(),
                _ => {}
            },
            Some("pin") => {
                if let Some(at) = pt(item.child("at")) {
                    sh.pins.push(SheetPin {
                        name: item.arg(0).unwrap_or("").to_string(),
                        shape: item.arg(1).unwrap_or("").to_string(),
                        at,
                    });
                }
            }
            Some("instances") => {
                for proj in item.children("project") {
                    for path in proj.children("path") {
                        sh.instances.push((
                            path.arg(0).unwrap_or("").to_string(),
                            path.value("page").map(String::from),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    if sh.file.is_empty() {
        notes.missing(&format!("sheet {}", sh.name), "no sheet file");
    }
    sh
}
