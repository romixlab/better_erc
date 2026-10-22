//! Altium's EDIF netlist ("EDIF for PCB"), read with the pest grammar `grammar/edif.pest`.
//!
//! Library cells with ports and no contents are the component symbols (their ports the pins, with a direction);
//! cells with contents are the sheets, holding the instances (parts, with their properties) and the nets (port
//! references to instances). Names can be renamed (`(rename CONN_2_3__6_ "CONN-2-3 (6)")`): the display name is
//! what the model keeps, the identifier is what references use.

use crate::notes::Notes;
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar/edif.pest"]
struct EdifParser;

/// Deeper nesting than any EDIF writer produces is refused before parsing, so the recursive grammar cannot run out
/// of stack on a broken or hostile file
const MAX_DEPTH: usize = 64;

pub(crate) struct Edif {
    pub name: String,
    /// `Altium Designer - EDIF For PCB 1.0.0`
    pub program: Option<String>,
    pub symbols: Vec<SymbolCell>,
    pub sheets: Vec<SheetCell>,
}

pub(crate) struct SymbolCell {
    pub id: String,
    pub name: String,
    pub ports: Vec<Port>,
}

pub(crate) struct Port {
    pub id: String,
    pub name: String,
    pub direction: Option<String>,
}

pub(crate) struct SheetCell {
    pub name: String,
    pub instances: Vec<Instance>,
    pub nets: Vec<EdifNet>,
}

pub(crate) struct Instance {
    pub designator: String,
    /// The symbol cell's identifier
    pub cell: Option<String>,
    pub properties: Vec<(String, String)>,
}

pub(crate) struct EdifNet {
    pub name: String,
    /// Port identifier and instance identifier
    pub refs: Vec<(String, Option<String>)>,
    pub properties: Vec<(String, String)>,
}

/// Parses an EDIF file; an error is the parser's message with line and column
pub(crate) fn parse(text: &str, file: &str, notes: &mut Notes) -> Result<Edif, String> {
    check_depth(text)?;
    let mut pairs = EdifParser::parse(Rule::file, text).map_err(|e| e.to_string())?;
    let mut edif = Edif {
        name: String::new(),
        program: None,
        symbols: Vec::new(),
        sheets: Vec::new(),
    };
    let Some(file_pair) = pairs.next() else {
        return Err("empty".into());
    };
    let mut r = Reader { file, notes };
    for p in file_pair.into_inner() {
        match p.as_rule() {
            Rule::k_edif | Rule::boilerplate | Rule::EOI | Rule::design => {}
            Rule::name => edif.name = name_of(p).1,
            Rule::status => edif.program = program(p),
            Rule::library => r.library(p, &mut edif),
            Rule::other => r.other("edif", &p),
            _ => {}
        }
    }
    Ok(edif)
}

fn check_depth(text: &str) -> Result<(), String> {
    let (mut depth, mut max, mut in_string) = (0usize, 0usize, false);
    for c in text.bytes() {
        match c {
            b'"' => in_string = !in_string,
            b'(' if !in_string => {
                depth += 1;
                max = max.max(depth);
            }
            b')' if !in_string => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    if max > MAX_DEPTH {
        return Err(format!("nested {max} levels deep, more than EDIF uses"));
    }
    Ok(())
}

/// (identifier, display name) of a `name` pair
fn name_of(p: Pair<Rule>) -> (String, String) {
    let Some(inner) = p.into_inner().next() else {
        return Default::default();
    };
    match inner.as_rule() {
        Rule::rename => {
            let mut it = inner.into_inner().skip(1);
            let id = it.next().map(|s| s.as_str().to_string()).unwrap_or_default();
            let display = it
                .next()
                .and_then(|s| s.into_inner().next())
                .map(|s| s.as_str().to_string())
                .unwrap_or_default();
            let display = if display.is_empty() {
                id.clone()
            } else {
                display
            };
            (id, display)
        }
        _ => (inner.as_str().to_string(), inner.as_str().to_string()),
    }
}

/// The first bare token of an s-expression (`(written ...)` → `written`)
fn head(p: &Pair<Rule>) -> String {
    p.clone()
        .into_inner()
        .find(|c| c.as_rule() == Rule::atom)
        .map_or_else(|| "()".to_string(), |a| a.as_str().to_string())
}

/// `(status (written ... (program "Altium Designer - EDIF For PCB" (version "1.0.0")) ...))`
fn program(p: Pair<Rule>) -> Option<String> {
    for d in p.into_inner().flat_map(|c| c.into_inner()) {
        if d.as_rule() == Rule::other && head(&d).eq_ignore_ascii_case("program") {
            let mut out: Vec<String> = Vec::new();
            for c in d.into_inner() {
                match c.as_rule() {
                    Rule::string => out.extend(c.into_inner().map(|s| s.as_str().to_string())),
                    Rule::other if head(&c).eq_ignore_ascii_case("version") => out.extend(
                        c.into_inner()
                            .filter(|s| s.as_rule() == Rule::string)
                            .flat_map(|s| s.into_inner())
                            .map(|s| s.as_str().to_string()),
                    ),
                    _ => {}
                }
            }
            return (!out.is_empty()).then(|| out.join(" "));
        }
    }
    None
}

/// The text of a typed value: `(String "x")` → `x`, `(Integer 3)` → `3`, `(Boolean (True))` → `True`
fn typed_text(p: Pair<Rule>) -> String {
    let mut out: Vec<String> = Vec::new();
    for c in p.into_inner().skip(1) {
        match c.as_rule() {
            Rule::string => out.extend(c.into_inner().map(|s| s.as_str().to_string())),
            Rule::atom => out.push(c.as_str().to_string()),
            Rule::other => out.push(head(&c)),
            _ => {}
        }
    }
    out.join(" ")
}

struct Reader<'a> {
    file: &'a str,
    notes: &'a mut Notes,
}

impl Reader<'_> {
    fn other(&mut self, at: &str, p: &Pair<Rule>) {
        let (line, _) = p.line_col();
        self.notes.unknown(
            self.file,
            at,
            format!("({} ...) at line {line} not read", head(p)),
        );
    }

    fn library(&mut self, p: Pair<Rule>, edif: &mut Edif) {
        let mut lib = String::new();
        for c in p.into_inner() {
            match c.as_rule() {
                Rule::k_library | Rule::boilerplate | Rule::technology => {}
                Rule::name => lib = name_of(c).1,
                Rule::cell => self.cell(c, &lib, edif),
                Rule::other => self.other(&format!("library {lib}"), &c),
                _ => {}
            }
        }
    }

    fn cell(&mut self, p: Pair<Rule>, lib: &str, edif: &mut Edif) {
        let (mut id, mut name) = (String::new(), String::new());
        let mut ports = Vec::new();
        let mut contents = None;
        for c in p.into_inner() {
            match c.as_rule() {
                Rule::k_cell | Rule::cell_type => {}
                Rule::name => (id, name) = name_of(c),
                Rule::view => {
                    for v in c.into_inner() {
                        match v.as_rule() {
                            Rule::k_view | Rule::name | Rule::view_type => {}
                            Rule::interface => {
                                for port in v.into_inner() {
                                    match port.as_rule() {
                                        Rule::port => ports.push(self.port(port, &name)),
                                        Rule::other => self.other(&format!("cell {name}"), &port),
                                        _ => {}
                                    }
                                }
                            }
                            Rule::contents => contents = Some(v),
                            Rule::other => self.other(&format!("cell {name}"), &v),
                            _ => {}
                        }
                    }
                }
                Rule::other => self.other(&format!("library {lib}"), &c),
                _ => {}
            }
        }
        match contents {
            Some(c) => {
                let sheet = self.contents(c, &name);
                edif.sheets.push(sheet);
            }
            None => edif.symbols.push(SymbolCell { id, name, ports }),
        }
    }

    fn port(&mut self, p: Pair<Rule>, cell: &str) -> Port {
        let mut port = Port {
            id: String::new(),
            name: String::new(),
            direction: None,
        };
        for c in p.into_inner() {
            match c.as_rule() {
                Rule::name => (port.id, port.name) = name_of(c),
                Rule::direction => {
                    port.direction = c
                        .into_inner()
                        .find(|d| d.as_rule() == Rule::symbol)
                        .map(|d| d.as_str().to_string());
                }
                Rule::other => self.other(&format!("cell {cell} port {}", port.name), &c),
                _ => {}
            }
        }
        port
    }

    fn contents(&mut self, p: Pair<Rule>, sheet: &str) -> SheetCell {
        let mut out = SheetCell {
            name: sheet.to_string(),
            instances: Vec::new(),
            nets: Vec::new(),
        };
        for c in p.into_inner() {
            match c.as_rule() {
                Rule::instance => out.instances.push(self.instance(c, sheet)),
                Rule::net => out.nets.push(self.net(c, sheet)),
                Rule::other => self.other(&format!("sheet {sheet}"), &c),
                _ => {}
            }
        }
        out
    }

    fn property(&mut self, p: Pair<Rule>, at: &str) -> (String, String) {
        let (mut name, mut value) = (String::new(), String::new());
        for c in p.into_inner() {
            match c.as_rule() {
                Rule::name => name = name_of(c).1,
                Rule::typed => value = typed_text(c),
                Rule::other => self.other(at, &c),
                _ => {}
            }
        }
        (name, value)
    }

    fn instance(&mut self, p: Pair<Rule>, sheet: &str) -> Instance {
        let mut inst = Instance {
            designator: String::new(),
            cell: None,
            properties: Vec::new(),
        };
        for c in p.into_inner() {
            match c.as_rule() {
                Rule::name => inst.designator = name_of(c).1,
                Rule::view_ref => {
                    inst.cell = c
                        .into_inner()
                        .find(|r| r.as_rule() == Rule::cell_ref)
                        .and_then(|r| r.into_inner().find(|n| n.as_rule() == Rule::name))
                        .map(|n| name_of(n).0);
                }
                Rule::property => {
                    let at = format!("instance {}", inst.designator);
                    let prop = self.property(c, &at);
                    inst.properties.push(prop);
                }
                Rule::other => self.other(&format!("sheet {sheet}"), &c),
                _ => {}
            }
        }
        inst
    }

    fn net(&mut self, p: Pair<Rule>, sheet: &str) -> EdifNet {
        let mut net = EdifNet {
            name: String::new(),
            refs: Vec::new(),
            properties: Vec::new(),
        };
        for c in p.into_inner() {
            match c.as_rule() {
                Rule::name => net.name = name_of(c).1,
                Rule::joined => {
                    for r in c.into_inner() {
                        match r.as_rule() {
                            Rule::port_ref => {
                                let mut it = r.into_inner().skip(1);
                                let port = it.next().map(|s| s.as_str().to_string());
                                let inst = it
                                    .next()
                                    .and_then(|i| i.into_inner().nth(1))
                                    .map(|s| s.as_str().to_string());
                                if let Some(port) = port {
                                    net.refs.push((port, inst));
                                }
                            }
                            Rule::other => self.other(&format!("net {}", net.name), &r),
                            _ => {}
                        }
                    }
                }
                Rule::property => {
                    let at = format!("net {}", net.name);
                    let prop = self.property(c, &at);
                    net.properties.push(prop);
                }
                Rule::other => self.other(&format!("sheet {sheet}"), &c),
                _ => {}
            }
        }
        net
    }
}
