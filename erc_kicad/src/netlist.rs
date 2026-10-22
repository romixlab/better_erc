//! The KiCad netlist (`kicad-cli sch export netlist`, the s-expression "KiCad" format, version `E`) as written
//! by KiCad 8, 9 and 10, read into a [`Board`]. It is the second path into the model and the oracle of the
//! schematic importer: what KiCad itself computed as connectivity.
//!
//! A netlist carries parts, library parts and nets, but no drawing: no labels, no no-connect flags, no
//! positions. Pins are the nodes of the nets (KiCad writes every pin of a placed unit, the unconnected ones as
//! `unconnected-(...)` nets).

use crate::notes::Notes;
use crate::sexpr::{self, Sexp};
use erc_model::*;
use std::collections::HashMap;

/// Reads a KiCad netlist; `file` names it in provenance and notes
pub fn read(text: &str, file: &str) -> Result<(Board, Vec<ImportNote>), ImportError> {
    let parsed = sexpr::parse(text).map_err(|e| ImportError::Invalid {
        file: file.into(),
        message: e.to_string(),
    })?;
    let root = match parsed.as_slice() {
        [r] if r.head() == Some("export") => r,
        _ => {
            return Err(ImportError::Invalid {
                file: file.into(),
                message: "not a KiCad netlist: no `(export ...)`".into(),
            });
        }
    };
    let mut r = Reader {
        src: Provenance::Netlist { file: file.into() },
        notes: Notes::new(file),
        b: Board {
            name: file
                .rsplit('/')
                .next()
                .unwrap_or(file)
                .trim_end_matches(".net")
                .to_string(),
            ..Board::default()
        },
        sheets: HashMap::new(),
        symbols: HashMap::new(),
        parts: HashMap::new(),
        variants: HashMap::new(),
        unit_of_pin: HashMap::new(),
    };
    r.b.source = SourceInfo {
        tool: "kicad-netlist".into(),
        version: None,
        files: vec![file.into()],
    };
    r.root(root);
    let mut b = r.b;
    b.finish();
    Ok((b, r.notes.into_vec()))
}

struct Reader {
    src: Provenance,
    notes: Notes,
    b: Board,
    sheets: HashMap<String, SheetId>,
    symbols: HashMap<String, SymbolId>,
    parts: HashMap<String, PartId>,
    variants: HashMap<String, VariantId>,
    /// (designator, pin number) → unit, from KiCad 10's per-component unit lists
    unit_of_pin: HashMap<(String, String), u32>,
}

impl Reader {
    fn fact<T>(&self, v: T) -> Fact<T> {
        Fact::new(v, self.src.clone())
    }

    fn root(&mut self, root: &Sexp) {
        // Sheets first (parts refer to them), then library parts, variants, parts, nets
        let order = [
            "version",
            "design",
            "libparts",
            "variants",
            "components",
            "nets",
            "libraries",
            "groups",
        ];
        for item in root.args() {
            match item.head() {
                Some(h) if order.contains(&h) => {}
                _ => self.notes.unknown("export", item),
            }
        }
        for head in order {
            for item in root.children(head) {
                match head {
                    "version" => {}
                    "design" => self.design(item),
                    "libparts" => {
                        for p in item.args() {
                            if p.head() == Some("libpart") {
                                self.libpart(p);
                            } else {
                                self.notes.unknown("libparts", p);
                            }
                        }
                    }
                    "variants" => {
                        for v in item.args() {
                            if v.head() == Some("variant") {
                                let name = v.value("name").unwrap_or("").to_string();
                                let description = v.value("description").map(String::from);
                                self.variant(&name, description);
                            } else {
                                self.notes.unknown("variants", v);
                            }
                        }
                    }
                    "components" => {
                        for c in item.args() {
                            if c.head() == Some("comp") {
                                self.comp(c);
                            } else {
                                self.notes.unknown("components", c);
                            }
                        }
                    }
                    "nets" => {
                        for n in item.args() {
                            if n.head() == Some("net") {
                                self.net(n);
                            } else {
                                self.notes.unknown("nets", n);
                            }
                        }
                    }
                    // Groups of drawn items (KiCad 10): nothing the model keeps yet
                    "groups" if !item.args().is_empty() => {
                        self.notes.ignored("groups", "item groups are not kept")
                    }
                    // Library file paths: nothing the model keeps
                    _ => {}
                }
            }
        }
        if self.b.sheets.is_empty() {
            self.sheet("/", "/", None, None);
        }
        self.ties_to_numbers();
    }

    fn design(&mut self, d: &Sexp) {
        for item in d.args() {
            match item.head() {
                Some("source" | "date") => {}
                Some("tool") => {
                    // `Eeschema 10.0.6`
                    self.b.source.version = item
                        .arg(0)
                        .and_then(|t| t.rsplit(' ').next())
                        .map(String::from);
                }
                Some("sheet") => {
                    let name = item.value("name").unwrap_or("/");
                    let tstamps = item.value("tstamps").unwrap_or("/");
                    let file = item
                        .child("title_block")
                        .and_then(|t| t.value("source"))
                        .map(String::from);
                    let page = item.value("number").map(String::from);
                    self.sheet(name, tstamps, file, page);
                }
                Some("textvar") => {
                    let key = item.value("name").or(item.arg(0)).unwrap_or("").to_string();
                    let value = item
                        .arg(1)
                        .or(item.value("value"))
                        .unwrap_or("")
                        .to_string();
                    let f = self.fact(value);
                    self.b.settings.text_vars.push(Param::new(key, f));
                }
                _ => self.notes.unknown("design", item),
            }
        }
    }

    /// The sheet with this path (`/MCU/Power/`), created with its parents when new
    fn sheet(
        &mut self,
        path: &str,
        tstamps: &str,
        file: Option<String>,
        page: Option<String>,
    ) -> SheetId {
        let path = normalize_path(path);
        if let Some(&id) = self.sheets.get(&path) {
            let s = &mut self.b.sheets[id.index()];
            if let Some(f) = file {
                s.file = f;
            }
            if page.is_some() {
                s.page = page;
            }
            if tstamps != "/" || s.uuid_path.is_empty() {
                s.uuid_path = normalize_uuid_path(tstamps);
            }
            return id;
        }
        let (parent, name) = if path == "/" {
            (None, String::new())
        } else {
            let trimmed = path.trim_end_matches('/');
            let cut = trimmed.rfind('/').unwrap_or(0);
            let parent_path = &trimmed[..=cut];
            let parent_tstamps = parent_of_uuid_path(tstamps);
            let parent = self.sheet(parent_path, &parent_tstamps, None, None);
            (Some(parent), trimmed[cut + 1..].to_string())
        };
        let id = SheetId::from_index(self.b.sheets.len());
        self.b.sheets.push(Sheet {
            name,
            file: file.unwrap_or_default(),
            parent,
            uuid_path: normalize_uuid_path(tstamps),
            page,
        });
        self.sheets.insert(path, id);
        id
    }

    fn variant(&mut self, name: &str, description: Option<String>) -> VariantId {
        if let Some(&v) = self.variants.get(name) {
            if description.is_some() {
                self.b.variants[v.index()].description = description;
            }
            return v;
        }
        let id = VariantId::from_index(self.b.variants.len());
        self.b.variants.push(Variant {
            name: name.to_string(),
            description,
        });
        self.variants.insert(name.to_string(), id);
        id
    }

    fn libpart(&mut self, p: &Sexp) {
        let lib = p.value("lib").unwrap_or("");
        let part = p.value("part").unwrap_or("");
        let name = format!("{lib}:{part}");
        let mut s = SymbolDef {
            name: name.clone(),
            units: 1,
            ..SymbolDef::default()
        };
        for item in p.args() {
            match item.head() {
                Some("lib" | "part" | "docs") => {}
                Some("description") => s.description = item.arg(0).unwrap_or("").into(),
                Some("footprints") => {
                    s.footprint_filters = item
                        .children("fp")
                        .filter_map(|f| f.arg(0))
                        .map(String::from)
                        .collect();
                }
                Some("fields") => {
                    for f in item.children("field") {
                        if let Some(n) = f.value("name") {
                            let v = f.arg(1).unwrap_or("");
                            if !v.is_empty() {
                                s.fields.push((n.into(), v.into()));
                            }
                        }
                    }
                }
                Some("pins") => {
                    for pin in item.children("pin") {
                        let ty = pin.value("type").unwrap_or("unspecified");
                        let etype = EcadPinType::from_name(ty).unwrap_or_else(|| {
                            self.notes
                                .invalid(&format!("libpart {name}"), &format!("pin type `{ty}`"));
                            EcadPinType::Unspecified
                        });
                        s.pins.push(PinDef {
                            number: pin.value("num").unwrap_or("").into(),
                            name: pin.value("name").unwrap_or("").into(),
                            unit: 0,
                            body_style: 0,
                            etype,
                            style: String::new(),
                            hidden: false,
                            alternates: Vec::new(),
                        });
                    }
                }
                _ => self.notes.unknown(&format!("libpart {name}"), item),
            }
        }
        let id = SymbolId::from_index(self.b.symbols.len());
        self.symbols.insert(name, id);
        self.b.symbols.push(s);
    }

    fn comp(&mut self, c: &Sexp) {
        let designator = c.value("ref").unwrap_or("").to_string();
        let ctx = format!("comp {designator}");
        if designator.is_empty() {
            self.notes
                .missing("components", "a component without a reference");
            return;
        }
        let mut fields: Vec<Field> = Vec::new();
        let add = |fields: &mut Vec<Field>, name: &str, value: &str, src: &Provenance| {
            if !value.is_empty() && !fields.iter().any(|f| f.name == name) {
                fields.push(Field::new(name, Fact::new(value.to_string(), src.clone())));
            }
        };
        let mut lib_id = String::new();
        let mut sheet = None;
        let mut uuids: Vec<String> = Vec::new();
        let mut flags = PartFlags::default();
        let mut variants: Vec<PartVariant> = Vec::new();
        let mut ties: Vec<Vec<String>> = Vec::new();
        let mut jumpers = false;
        // KiCad 10's unit list: the name of each unit, in unit order
        let mut unit_names: Vec<String> = Vec::new();
        for item in c.args() {
            match item.head() {
                Some("ref") => {}
                // KiCad writes an empty value as "~"
                Some("value") => {
                    let v = item.arg(0).unwrap_or("");
                    add(
                        &mut fields,
                        "Value",
                        if v == "~" { "" } else { v },
                        &self.src,
                    );
                }
                Some("footprint") => add(
                    &mut fields,
                    "Footprint",
                    item.arg(0).unwrap_or(""),
                    &self.src,
                ),
                Some("datasheet") => add(
                    &mut fields,
                    "Datasheet",
                    item.arg(0).unwrap_or(""),
                    &self.src,
                ),
                Some("description") => add(
                    &mut fields,
                    "Description",
                    item.arg(0).unwrap_or(""),
                    &self.src,
                ),
                Some("fields") => {
                    for f in item.children("field") {
                        if let Some(n) = f.value("name") {
                            add(&mut fields, n, f.arg(1).unwrap_or(""), &self.src);
                        }
                    }
                }
                Some("libsource") => {
                    lib_id = format!(
                        "{}:{}",
                        item.value("lib").unwrap_or(""),
                        item.value("part").unwrap_or("")
                    );
                }
                Some("property") => {
                    let name = item.value("name").unwrap_or("");
                    let value = item.value("value").unwrap_or("");
                    if !self.flag(&mut flags, name, value)
                        && !matches!(name, "Sheetname" | "Sheetfile")
                    {
                        add(&mut fields, name, value, &self.src);
                    }
                }
                Some("sheetpath") => {
                    let names = item.value("names").unwrap_or("/");
                    let tstamps = item.value("tstamps").unwrap_or("/");
                    sheet = Some(self.sheet(names, tstamps, None, None));
                }
                Some("tstamps") => {
                    uuids = item
                        .args()
                        .iter()
                        .filter_map(Sexp::text)
                        .map(String::from)
                        .collect();
                }
                Some("units") => {
                    for (i, u) in item.children("unit").enumerate() {
                        unit_names.push(u.value("name").unwrap_or("").to_string());
                        for pins in u.children("pins") {
                            for p in pins.children("pin") {
                                if let Some(n) = p.value("num") {
                                    self.unit_of_pin
                                        .insert((designator.clone(), n.to_string()), i as u32 + 1);
                                }
                            }
                        }
                    }
                }
                Some("variants") => {
                    for v in item.children("variant") {
                        let name = v.value("name").unwrap_or("");
                        let id = self.variant(name, None);
                        let mut pv = PartVariant {
                            variant: id,
                            flags: PartFlags::default(),
                            fields: Vec::new(),
                        };
                        for x in v.args() {
                            match x.head() {
                                Some("name") => {}
                                Some("property") => {
                                    let n = x.value("name").unwrap_or("");
                                    let val = x.value("value").unwrap_or("");
                                    if !self.flag(&mut pv.flags, n, val) {
                                        pv.fields.push(Field::new(n, self.fact(val.to_string())));
                                    }
                                }
                                Some("fields") => {
                                    for f in x.children("field") {
                                        if let Some(n) = f.value("name") {
                                            let val = f.arg(1).unwrap_or("").to_string();
                                            pv.fields.push(Field::new(n, self.fact(val)));
                                        }
                                    }
                                }
                                _ => self.notes.unknown(&format!("{ctx} variant {name}"), x),
                            }
                        }
                        variants.push(pv);
                    }
                }
                Some("component_classes") => {
                    let classes: Vec<&str> =
                        item.children("class").filter_map(|c| c.arg(0)).collect();
                    if !classes.is_empty() {
                        add(
                            &mut fields,
                            "Component Class",
                            &classes.join(","),
                            &self.src,
                        );
                    }
                }
                Some("duplicate_pin_numbers_are_jumpers") => {
                    jumpers = matches!(item.arg(0), Some("1" | "yes" | "true"));
                }
                Some("jumper_pin_groups") => {
                    for g in item.children("group") {
                        let pins: Vec<String> = g
                            .children("pin")
                            .filter_map(|p| p.arg(0))
                            .map(String::from)
                            .collect();
                        ties.push(pins);
                    }
                }
                _ => self.notes.unknown(&ctx, item),
            }
        }
        let sheet = sheet.unwrap_or_else(|| self.sheet("/", "/", None, None));
        let symbol = self.symbols.get(&lib_id).copied();
        if let Some(s) = symbol {
            let def = &mut self.b.symbols[s.index()];
            def.duplicate_pins_are_jumpers |= jumpers;
            def.units = def.units.max(unit_names.len() as u32);
            for (i, name) in unit_names.iter().enumerate() {
                let unit = i as u32 + 1;
                if !name.is_empty() && !def.unit_names.iter().any(|(u, _)| *u == unit) {
                    def.unit_names.push((unit, name.clone()));
                }
            }
        }
        let value = fields
            .iter()
            .find(|f| f.name == "Value")
            .and_then(|f| f.value.get())
            .map(|v| Value::parse(v).0)
            .unwrap_or(Value::Text(String::new()));
        let units = uuids
            .iter()
            .enumerate()
            .map(|(i, u)| UnitPlacement {
                unit: if uuids.len() > 1 { i as u32 + 1 } else { 0 },
                sheet,
                uuid: u.clone(),
                line: None,
                at: None,
            })
            .collect();
        if self.parts.contains_key(&designator) {
            self.notes.invalid(
                &ctx,
                "a second component with this reference; the first is kept",
            );
            return;
        }
        let id = PartId::from_index(self.b.parts.len());
        self.parts.insert(designator.clone(), id);
        let value = Facts::one(self.fact(value).with_note("from the Value field"));
        self.b.parts.push(Part {
            designator,
            symbol,
            lib_id,
            sheet,
            units,
            fields,
            value,
            flags,
            variants,
            pins: Vec::new(),
            ties,
        });
    }

    /// A part known only by its reference (a node of a component the netlist does not list)
    fn bare_part(&mut self, designator: &str) -> PartId {
        let sheet = self.sheet("/", "/", None, None);
        let id = PartId::from_index(self.b.parts.len());
        self.parts.insert(designator.to_string(), id);
        self.b.parts.push(Part {
            designator: designator.to_string(),
            symbol: None,
            lib_id: String::new(),
            sheet,
            units: Vec::new(),
            fields: Vec::new(),
            value: Facts::default(),
            flags: PartFlags::default(),
            variants: Vec::new(),
            pins: Vec::new(),
            ties: Vec::new(),
        });
        id
    }

    /// Jumper pin groups name pins by name (KiCad 10); the model keeps pin numbers
    fn ties_to_numbers(&mut self) {
        for (i, part) in self.b.parts.iter_mut().enumerate() {
            let pins: Vec<&PinInst> = self.b.pins.iter().filter(|p| p.part.index() == i).collect();
            for group in &mut part.ties {
                for entry in group.iter_mut() {
                    if pins.iter().any(|p| p.number == *entry) {
                        continue;
                    }
                    if let Some(p) = pins.iter().find(|p| p.name == *entry) {
                        *entry = p.number.clone();
                    }
                }
            }
        }
    }

    /// A component property that is one of KiCad's flags; `false` when it is not one
    fn flag(&self, flags: &mut PartFlags, name: &str, value: &str) -> bool {
        // A flag property is set by its presence; KiCad 10 variants write "1" / "0"
        let on = !matches!(value, "0" | "no" | "false");
        let (slot, v) = match name {
            "dnp" => (&mut flags.dnp, on),
            "exclude_from_bom" => (&mut flags.in_bom, !on),
            "exclude_from_board" => (&mut flags.on_board, !on),
            "exclude_from_sim" => (&mut flags.exclude_from_sim, on),
            "exclude_from_pos_files" => (&mut flags.in_pos_files, !on),
            _ => return false,
        };
        slot.set(self.fact(v));
        true
    }

    fn net(&mut self, n: &Sexp) {
        let name = n.value("name").unwrap_or("").to_string();
        let ctx = format!("net {name}");
        let kind = if name.starts_with("unconnected-(") {
            NetKind::Unconnected
        } else if name.starts_with("Net-(") || name.contains("/Net-(") {
            NetKind::Auto
        } else {
            NetKind::Named
        };
        let class = match n.value("class") {
            Some(c) if !c.is_empty() => Facts::one(self.fact(c.to_string())),
            _ => Facts::default(),
        };
        let id = NetId::from_index(self.b.nets.len());
        self.b.nets.push(Net {
            name: name.clone(),
            kind,
            class,
            pins: Vec::new(),
            labels: Vec::new(),
        });
        for item in n.args() {
            match item.head() {
                Some("code" | "name" | "class") => {}
                Some("node") => {
                    let r = item.value("ref").unwrap_or("");
                    let num = item.value("pin").unwrap_or("").to_string();
                    let part = match self.parts.get(r) {
                        Some(&p) => p,
                        None => {
                            // Keep the connection: a part with only this reference
                            self.notes.invalid(
                                &ctx,
                                &format!("node {r}.{num}: no such component, added without fields"),
                            );
                            self.bare_part(r)
                        }
                    };
                    // `+no_connect` marks a pin KiCad found unconnected; the type is the part before it
                    let ty = item.value("pintype").unwrap_or("unspecified");
                    let ty = ty.split('+').next().unwrap_or(ty);
                    let etype = EcadPinType::from_name(ty).unwrap_or_else(|| {
                        self.notes.invalid(&ctx, &format!("pin type `{ty}`"));
                        EcadPinType::Unspecified
                    });
                    let p = &self.b.parts[part.index()];
                    let lib_name = p.symbol.and_then(|s| {
                        self.b.symbols[s.index()]
                            .pins
                            .iter()
                            .find(|d| d.number == num)
                            .map(|d| d.name.clone())
                    });
                    let name = lib_name
                        .or_else(|| item.value("pinfunction").map(String::from))
                        .unwrap_or_default();
                    let unit = self
                        .unit_of_pin
                        .get(&(r.to_string(), num.clone()))
                        .copied()
                        .unwrap_or(0);
                    self.b.pins.push(PinInst {
                        part,
                        number: num,
                        name,
                        unit,
                        etype: Facts::one(self.fact(etype)),
                        hidden: false,
                        net: Some(id),
                        no_connect: false,
                        alt: None,
                    });
                }
                _ => self.notes.unknown(&ctx, item),
            }
        }
    }
}

/// `/MCU/` with exactly one leading and one trailing slash
fn normalize_path(p: &str) -> String {
    let t = p.trim_matches('/');
    if t.is_empty() {
        "/".into()
    } else {
        format!("/{t}/")
    }
}

/// `/uuid1/uuid2` without the trailing slash; the root is `/`
pub(crate) fn normalize_uuid_path(p: &str) -> String {
    let t = p.trim_matches('/');
    if t.is_empty() {
        "/".into()
    } else {
        format!("/{t}")
    }
}

fn parent_of_uuid_path(p: &str) -> String {
    let t = p.trim_matches('/');
    match t.rfind('/') {
        Some(i) => format!("/{}", &t[..i]),
        None => "/".into(),
    }
}

/// The KiCad netlist as an [`Importer`]
pub struct NetlistImporter;

impl Importer for NetlistImporter {
    fn name(&self) -> &'static str {
        "kicad-netlist"
    }

    fn sniff(&self, src: &Source) -> Confidence {
        match src.files.read(&src.main) {
            Some(b) if b.trim_ascii_start().starts_with(b"(export") => Confidence::Yes,
            Some(_) if src.main.ends_with(".net") => Confidence::Maybe,
            _ => Confidence::No,
        }
    }

    fn import(&self, src: &Source, _opts: &ImportOptions) -> Result<Imported, ImportError> {
        let text = src.text(&src.main)?;
        let (board, notes) = read(&text, &src.main)?;
        Ok(Imported {
            design: Design::single(board),
            notes,
        })
    }
}
