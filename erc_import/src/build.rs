//! A board filled from netlist files: parts, pins and nets looked up by name, so several files (EDIF and WireList,
//! pstxprt and pstxnet) can each add their facts to the same entities

use crate::notes::Notes;
use erc_model::*;
use std::collections::HashMap;

pub(crate) struct Builder {
    pub b: Board,
    parts: HashMap<String, PartId>,
    pins: HashMap<(PartId, String), PinId>,
    nets: HashMap<String, NetId>,
    symbols: HashMap<String, SymbolId>,
    sheets: HashMap<String, SheetId>,
    /// The file that first connected a pin, to name both when another file puts it on a different net
    pin_net_file: HashMap<PinId, String>,
}

impl Builder {
    /// A board with its root sheet
    pub fn new(name: &str, tool: &str) -> Builder {
        let mut b = Board {
            name: name.to_string(),
            source: SourceInfo {
                tool: tool.into(),
                version: None,
                files: Vec::new(),
                connectivity: None,
            },
            ..Board::default()
        };
        b.sheets.push(Sheet {
            name: String::new(),
            file: String::new(),
            parent: None,
            uuid_path: String::new(),
            page: None,
            rev: None,
        });
        Builder {
            b,
            parts: HashMap::new(),
            pins: HashMap::new(),
            nets: HashMap::new(),
            symbols: HashMap::new(),
            sheets: HashMap::new(),
            pin_net_file: HashMap::new(),
        }
    }

    pub fn root(&self) -> SheetId {
        SheetId(0)
    }

    /// The child sheet of the root with this name, created when new
    pub fn sheet(&mut self, name: &str, page: Option<String>) -> SheetId {
        if let Some(&s) = self.sheets.get(name) {
            return s;
        }
        let id = SheetId::from_index(self.b.sheets.len());
        self.b.sheets.push(Sheet {
            name: name.to_string(),
            file: String::new(),
            parent: Some(self.root()),
            uuid_path: String::new(),
            page,
            rev: None,
        });
        self.sheets.insert(name.to_string(), id);
        id
    }

    /// Adds a library symbol; a second one with the same name is not added and `None` returned
    pub fn add_symbol(&mut self, s: SymbolDef) -> Option<SymbolId> {
        if self.symbols.contains_key(&s.name) {
            return None;
        }
        let id = SymbolId::from_index(self.b.symbols.len());
        self.symbols.insert(s.name.clone(), id);
        self.b.symbols.push(s);
        Some(id)
    }

    pub fn symbol(&self, name: &str) -> Option<SymbolId> {
        self.symbols.get(name).copied()
    }

    pub fn find_part(&self, designator: &str) -> Option<PartId> {
        self.parts.get(designator).copied()
    }

    /// The part with this designator, created on the root sheet when new
    pub fn part(&mut self, designator: &str) -> PartId {
        if let Some(&p) = self.parts.get(designator) {
            return p;
        }
        let id = PartId::from_index(self.b.parts.len());
        self.b.parts.push(Part {
            designator: designator.to_string(),
            symbol: None,
            lib_id: String::new(),
            sheet: self.root(),
            units: Vec::new(),
            fields: Vec::new(),
            value: Facts::default(),
            flags: PartFlags::default(),
            variants: Vec::new(),
            pins: Vec::new(),
            ties: Vec::new(),
        });
        self.parts.insert(designator.to_string(), id);
        id
    }

    pub fn part_mut(&mut self, id: PartId) -> &mut Part {
        &mut self.b.parts[id.index()]
    }

    /// States a field of a part; a source restating it replaces its own fact
    pub fn field(&mut self, part: PartId, name: &str, fact: Fact<String>) {
        let fields = &mut self.b.parts[part.index()].fields;
        match fields.iter_mut().find(|f| f.name == name) {
            Some(f) => f.value.set(fact),
            None => fields.push(Field::new(name, fact)),
        }
    }

    /// States the part's value from the text of one of its fields
    pub fn value(&mut self, part: PartId, text: &str, source: &Provenance, from: &str) {
        let v = Value::parse(text).0;
        self.b.parts[part.index()]
            .value
            .set(Fact::new(v, source.clone()).with_note(format!("from the {from}")));
    }

    /// The net with this name, created when new
    pub fn net(&mut self, name: &str, kind: NetKind) -> NetId {
        if let Some(&n) = self.nets.get(name) {
            return n;
        }
        let id = NetId::from_index(self.b.nets.len());
        self.b.nets.push(Net {
            name: name.to_string(),
            kind,
            class: Facts::default(),
            pins: Vec::new(),
            labels: Vec::new(),
        });
        self.nets.insert(name.to_string(), id);
        id
    }

    /// The pin of a part with this number, created (unconnected, no type) when new
    pub fn pin(&mut self, part: PartId, number: &str) -> PinId {
        let key = (part, number.to_string());
        if let Some(&p) = self.pins.get(&key) {
            return p;
        }
        let id = PinId::from_index(self.b.pins.len());
        self.b.pins.push(PinInst {
            part,
            number: number.to_string(),
            name: String::new(),
            unit: 0,
            etype: Facts::default(),
            hidden: false,
            net: None,
            no_connect: false,
            alt: None,
        });
        self.pins.insert(key, id);
        id
    }

    pub fn pin_mut(&mut self, id: PinId) -> &mut PinInst {
        &mut self.b.pins[id.index()]
    }

    /// Puts a pin on a net. A second file that puts it on another net is noted and the first net kept.
    pub fn connect(&mut self, pin: PinId, net: NetId, file: &str, notes: &mut Notes) {
        match self.b.pins[pin.index()].net {
            None => {
                self.b.pins[pin.index()].net = Some(net);
                self.pin_net_file.insert(pin, file.to_string());
            }
            Some(n) if n == net => {}
            Some(n) => {
                let first = self.pin_net_file.get(&pin).cloned().unwrap_or_default();
                let what = if first == file {
                    "also on".to_string()
                } else {
                    format!(
                        "{first} puts it on `{}`, this file on",
                        self.b.nets[n.index()].name
                    )
                };
                let msg = format!(
                    "pin {}: {what} `{}`; the first net is kept",
                    self.b.pin_name(pin),
                    self.b.nets[net.index()].name
                );
                notes.invalid(file, &format!("net {}", self.b.nets[net.index()].name), msg);
            }
        }
    }

    /// Makes sure every pin of the part's symbol on a placed unit (or common to all units) exists, with its unit
    /// and name, and states its type with `etype` (a pin a netlist leaves out is unconnected, not absent)
    pub fn symbol_pins(
        &mut self,
        part: PartId,
        etype: impl Fn(&PinDef) -> Option<Fact<EcadPinType>>,
    ) {
        let Some(sym) = self.b.parts[part.index()].symbol else {
            return;
        };
        let placed: Vec<u32> = self.b.parts[part.index()]
            .units
            .iter()
            .map(|u| u.unit)
            .collect();
        let defs: Vec<PinDef> = self.b.symbols[sym.index()]
            .pins
            .iter()
            .filter(|d| d.unit == 0 || placed.is_empty() || placed.contains(&d.unit))
            .cloned()
            .collect();
        for d in defs {
            let id = self.pin(part, &d.number);
            let p = &mut self.b.pins[id.index()];
            if p.name.is_empty() {
                p.name = d.name.clone();
            }
            if p.unit == 0 {
                p.unit = d.unit;
            }
            if let Some(f) = etype(&d) {
                p.etype.set(f);
            }
        }
    }

    pub fn finish(self) -> Board {
        let mut b = self.b;
        b.finish();
        b
    }
}

/// Altium names unnamed nets `NetR13_1` (designator and pin of one of its pins)
#[cfg_attr(not(any(feature = "altium", feature = "pdf")), allow(dead_code))]
pub(crate) fn net_kind(name: &str) -> NetKind {
    match name.strip_prefix("Net") {
        Some(rest) if rest.starts_with(|c: char| c.is_ascii_uppercase()) && rest.contains('_') => {
            NetKind::Auto
        }
        _ => NetKind::Named,
    }
}
