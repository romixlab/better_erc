//! Altium Designer netlists: the EDIF netlist ("EDIF for PCB", `.edf`) and the WireList (`.net`), read together
//! into one board. EDIF gives the library symbols with their ports, the sheets, the parts with all their
//! properties, and the nets; the WireList gives what EDIF lacks: each pin's name and electrical type, and the
//! footprint. Either file can be the main file; the other is found beside it (same name first, then any file of
//! that kind in the same directory). One alone imports too, with a note on what is missing.
//!
//! Both files state facts about the same parts and pins, each with its own [`Provenance::Netlist`]: a pin's type
//! from EDIF (its port direction, `INOUT` for nearly everything) ranks just below the WireList's, so the finer
//! WireList type is the one a check sees and the EDIF one stays as a second source. A pin the two files put on
//! different nets is reported.

mod edif;
mod wirelist;

use crate::build::Builder;
use crate::notes::Notes;
use crate::{dir_of, file_name, text};
use erc_model::*;

/// The Altium importer (EDIF plus WireList)
pub struct AltiumImporter;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Edif,
    WireList,
}

fn kind_of(src: &Source, path: &str) -> Option<Kind> {
    let head = text::head(src, path, 16)?;
    if head.len() >= 5 && head[..5].eq_ignore_ascii_case(b"(edif") {
        Some(Kind::Edif)
    } else if head.starts_with(b"Wire List") {
        Some(Kind::WireList)
    } else {
        None
    }
}

/// The other file of the pair: the same name with another extension first, then the only file of that kind in the
/// directory
fn companion(src: &Source, kind: Kind) -> Option<String> {
    let want = match kind {
        Kind::Edif => Kind::WireList,
        Kind::WireList => Kind::Edif,
    };
    let dir = dir_of(&src.main);
    let stem = |p: &str| {
        let n = file_name(p);
        n.rsplit_once('.').map_or(n, |(s, _)| s).to_ascii_lowercase()
    };
    let main_stem = stem(&src.main);
    let siblings: Vec<String> = src
        .files
        .list()
        .into_iter()
        .filter(|p| *p != src.main && dir_of(p) == dir)
        .collect();
    if let Some(p) = siblings
        .iter()
        .find(|p| stem(p) == main_stem && kind_of(src, p) == Some(want))
    {
        return Some(p.clone());
    }
    let exts: &[&str] = match want {
        Kind::Edif => &["edf", "edif", "edn"],
        Kind::WireList => &["net", "txt", "wir"],
    };
    let mut found = siblings.iter().filter(|p| {
        file_name(p)
            .rsplit_once('.')
            .is_some_and(|(_, e)| exts.contains(&e.to_ascii_lowercase().as_str()))
            && kind_of(src, p) == Some(want)
    });
    match (found.next(), found.next()) {
        (Some(p), None) => Some(p.clone()),
        _ => None,
    }
}

impl Importer for AltiumImporter {
    fn name(&self) -> &'static str {
        "altium"
    }

    fn sniff(&self, src: &Source) -> Confidence {
        if kind_of(src, &src.main).is_some() {
            return Confidence::Yes;
        }
        let ext = src.main.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
        match ext.as_deref() {
            Some("edf" | "edif") if src.files.read(&src.main).is_some() => Confidence::Maybe,
            _ => Confidence::No,
        }
    }

    fn import(&self, src: &Source, _opts: &ImportOptions) -> Result<Imported, ImportError> {
        let kind = kind_of(src, &src.main).unwrap_or(Kind::Edif);
        let other = companion(src, kind);
        let (edif_file, wire_file) = match kind {
            Kind::Edif => (Some(src.main.clone()), other),
            Kind::WireList => (other, Some(src.main.clone())),
        };
        let mut notes = Notes::default();
        let invalid = |file: &str, message: String| ImportError::Invalid {
            file: file.to_string(),
            message,
        };
        let edif = match &edif_file {
            Some(f) => Some(edif::parse(&text::read(src, f)?, f, &mut notes).map_err(|m| invalid(f, m))?),
            None => None,
        };
        let wire = match &wire_file {
            Some(f) => {
                Some(wirelist::parse(&text::read(src, f)?, f, &mut notes).map_err(|m| invalid(f, m))?)
            }
            None => None,
        };
        let name = match &edif {
            Some(e) if !e.name.is_empty() => e.name.clone(),
            _ => {
                let n = file_name(&src.main);
                n.rsplit_once('.').map_or(n, |(s, _)| s).to_string()
            }
        };
        let mut b = Builder::new(&name, "altium");
        if let (Some(e), Some(f)) = (&edif, &edif_file) {
            b.b.source.version = e.program.clone();
            b.b.source.files.push(f.clone());
            fill_edif(&mut b, e, f, &mut notes);
        }
        if let (Some(w), Some(f)) = (&wire, &wire_file) {
            b.b.source.files.push(f.clone());
            fill_wirelist(&mut b, w, f, &mut notes);
        }
        match (&edif_file, &wire_file) {
            (Some(f), None) => notes.missing(
                f,
                "",
                "no WireList beside it: pin names are missing and pin types are the EDIF port directions only",
            ),
            (None, Some(f)) => notes.missing(
                f,
                "",
                "no EDIF netlist beside it: no library symbols, sheets or part properties beyond the comment and footprint",
            ),
            _ => {}
        }
        Ok(Imported {
            design: Design::single(b.finish()),
            notes: notes.into_vec(),
        })
    }
}

/// EDIF port directions are coarse (Altium writes `INOUT` for most pins): one below the design rank, so the
/// WireList's pin type wins where both are there
const EDIF_DIRECTION_RANK: Rank = Rank(Rank::DESIGN.0 - 1);

fn direction_type(d: Option<&str>) -> Option<EcadPinType> {
    Some(match d?.to_ascii_uppercase().as_str() {
        "INOUT" => EcadPinType::Bidirectional,
        "INPUT" => EcadPinType::Input,
        "OUTPUT" => EcadPinType::Output,
        _ => return None,
    })
}

fn fill_edif(b: &mut Builder, e: &edif::Edif, file: &str, notes: &mut Notes) {
    let src = Provenance::Netlist { file: file.into() };
    for cell in &e.symbols {
        let mut s = SymbolDef {
            name: cell.name.clone(),
            units: 1,
            ..SymbolDef::default()
        };
        for p in &cell.ports {
            let etype = direction_type(p.direction.as_deref()).unwrap_or_else(|| {
                if let Some(d) = &p.direction {
                    notes.invalid(
                        file,
                        &format!("cell {} port {}", cell.name, p.name),
                        format!("direction `{d}`"),
                    );
                }
                EcadPinType::Unspecified
            });
            s.pins.push(PinDef {
                number: p.name.clone(),
                name: String::new(),
                unit: 0,
                body_style: 0,
                etype,
                style: String::new(),
                hidden: false,
                alternates: Vec::new(),
            });
        }
        if b.add_symbol(s).is_none() {
            notes.invalid(
                file,
                &format!("cell {}", cell.name),
                "a second symbol cell with this name; the first is kept",
            );
        }
    }
    let cell_of = |id: &str| e.symbols.iter().find(|c| c.id == id);
    for sheet in &e.sheets {
        let sid = b.sheet(&sheet.name, None);
        for inst in &sheet.instances {
            let ctx = format!("instance {}", inst.designator);
            if inst.designator.is_empty() {
                notes.missing(file, &format!("sheet {}", sheet.name), "an instance without a name");
                continue;
            }
            if b.find_part(&inst.designator).is_some() {
                notes.invalid(file, &ctx, "a second instance with this designator; the first is kept");
                continue;
            }
            let part = b.part(&inst.designator);
            let cell = inst.cell.as_deref().and_then(cell_of);
            let symbol = cell.and_then(|c| b.symbol(&c.name));
            let p = b.part_mut(part);
            p.sheet = sid;
            p.lib_id = cell.map_or_else(|| inst.cell.clone().unwrap_or_default(), |c| c.name.clone());
            p.symbol = symbol;
            if cell.is_none() {
                notes.missing(
                    file,
                    &ctx,
                    format!(
                        "symbol cell `{}` is not in the libraries",
                        inst.cell.as_deref().unwrap_or("")
                    ),
                );
            }
            for (name, value) in &inst.properties {
                // Altium writes every property, empty ones too: an empty one says nothing
                if !value.is_empty() {
                    b.field(part, name, Fact::new(value.clone(), src.clone()));
                }
            }
            let value_from = ["Value", "Comment"].into_iter().find_map(|n| {
                inst.properties
                    .iter()
                    .find(|(k, v)| k == n && !v.is_empty())
                    .map(|(k, v)| (k.clone(), v.clone()))
            });
            if let Some((k, v)) = value_from {
                b.value(part, &v, &src, &format!("{k} property"));
            }
            b.symbol_pins(part, |d| {
                Some(
                    Fact::new(d.etype, src.clone())
                        .with_rank(EDIF_DIRECTION_RANK)
                        .with_note("EDIF port direction"),
                )
            });
        }
    }
    for sheet in &e.sheets {
        for n in &sheet.nets {
            let ctx = format!("net {}", n.name);
            let net = b.net(&n.name, net_kind(&n.name));
            for (port, inst) in &n.refs {
                let Some(inst) = inst else {
                    // A port of the sheet itself: Altium's flattened netlist has none
                    notes.ignored(file, &ctx, "a port reference without an instance is not kept");
                    continue;
                };
                // References use identifiers; the instance's designator and the port's number are display names
                let designator = sheet
                    .instances
                    .iter()
                    .find(|i| i.designator == *inst || ident(&i.designator) == *inst)
                    .map_or_else(|| inst.clone(), |i| i.designator.clone());
                let cell = sheet
                    .instances
                    .iter()
                    .find(|i| i.designator == designator)
                    .and_then(|i| i.cell.as_deref())
                    .and_then(cell_of);
                let number = cell
                    .and_then(|c| c.ports.iter().find(|p| p.id == *port))
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| port.strip_prefix('&').unwrap_or(port).to_string());
                let part = match b.find_part(&designator) {
                    Some(p) => p,
                    None => {
                        notes.invalid(
                            file,
                            &ctx,
                            format!("{designator}.{number}: no such instance, added without fields"),
                        );
                        b.part(&designator)
                    }
                };
                let pin = b.pin(part, &number);
                b.connect(pin, net, file, notes);
            }
            for (k, _) in &n.properties {
                notes.ignored(file, &ctx, format!("net property `{k}` is not kept"));
            }
        }
    }
}

/// The identifier EDIF would make of a display name (`R13` stays `R13`)
fn ident(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Altium names unnamed nets `NetR13_1` (designator and pin of one of its pins)
fn net_kind(name: &str) -> NetKind {
    match name.strip_prefix("Net") {
        Some(rest) if rest.starts_with(|c: char| c.is_ascii_uppercase()) && rest.contains('_') => {
            NetKind::Auto
        }
        _ => NetKind::Named,
    }
}

fn wire_type(t: &str) -> (EcadPinType, Option<&'static str>) {
    match t {
        "PASSIVE" => (EcadPinType::Passive, None),
        "OUTPUT" => (EcadPinType::Output, None),
        "INPUT" => (EcadPinType::Input, None),
        "I/O" => (EcadPinType::Bidirectional, None),
        "OPEN COLLECTOR" => (EcadPinType::OpenCollector, None),
        "OPEN EMITTER" => (EcadPinType::OpenEmitter, None),
        "HIZ" | "HI-Z" => (EcadPinType::TriState, None),
        "POWER" => (
            EcadPinType::PowerIn,
            Some("Altium POWER: the WireList does not say input or output"),
        ),
        _ => (EcadPinType::Unspecified, Some("unknown WireList pin type")),
    }
}

fn fill_wirelist(b: &mut Builder, w: &wirelist::WireList, file: &str, notes: &mut Notes) {
    let src = Provenance::Netlist { file: file.into() };
    let listed: std::collections::HashSet<&str> =
        w.components.iter().map(|c| c.designator.as_str()).collect();
    let mut unlisted = std::collections::HashSet::new();
    for c in &w.components {
        let part = b.part(&c.designator);
        b.field(part, "Comment", Fact::new(c.comment.clone(), src.clone()));
        b.field(part, "Footprint", Fact::new(c.footprint.clone(), src.clone()));
        if b.b.parts[part.index()].field("Value").is_none() {
            b.value(part, &c.comment, &src, "component list");
        }
    }
    for n in &w.nets {
        let ctx = format!("net {}", n.name);
        let net = b.net(&n.name, net_kind(&n.name));
        for c in &n.connections {
            if !listed.contains(c.designator.as_str()) && unlisted.insert(c.designator.as_str()) {
                notes.invalid(
                    file,
                    &ctx,
                    format!("{}.{}: not in the component list", c.designator, c.pin),
                );
            }
            let part = b.part(&c.designator);
            if !listed.contains(c.designator.as_str()) {
                // The part value column is all the WireList says about an unlisted part
                b.field(part, "Comment", Fact::new(c.part_value.clone(), src.clone()));
            } else if let Some(comp) = w.components.iter().find(|x| x.designator == c.designator)
                && comp.comment != c.part_value
            {
                notes.invalid(
                    file,
                    &ctx,
                    format!(
                        "{}.{}: part value `{}` differs from the component list's `{}`",
                        c.designator, c.pin, c.part_value, comp.comment
                    ),
                );
            }
            let pin = b.pin(part, &c.pin);
            let (etype, note) = wire_type(&c.pin_type);
            let mut fact = Fact::new(etype, src.clone());
            if let Some(nt) = note {
                fact = fact.with_note(format!("{nt} ({})", c.pin_type));
            }
            let p = b.pin_mut(pin);
            p.name = c.pin_name.clone();
            p.etype.set(fact);
            b.connect(pin, net, file, notes);
        }
    }
}
