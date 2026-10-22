//! Altium's WireList netlist, read with the pest grammar `grammar/wirelist.pest`: the component list (part value,
//! designator, footprint) and the wire list (each net with its connections: designator, pin number, pin name, pin
//! type, part value). It is the Altium file that carries pin names and electrical types.

use crate::notes::Notes;
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar/wirelist.pest"]
struct WireListParser;

pub(crate) struct WireList {
    pub components: Vec<Component>,
    pub nets: Vec<WireNet>,
}

pub(crate) struct Component {
    /// The first column: the part's comment (value or part number)
    pub comment: String,
    pub designator: String,
    pub footprint: String,
}

pub(crate) struct WireNet {
    pub name: String,
    pub connections: Vec<Connection>,
}

pub(crate) struct Connection {
    pub designator: String,
    pub pin: String,
    pub pin_name: String,
    pub pin_type: String,
    pub part_value: String,
}

pub(crate) fn parse(text: &str, file: &str, notes: &mut Notes) -> Result<WireList, String> {
    let mut pairs = WireListParser::parse(Rule::file, text).map_err(|e| e.to_string())?;
    let mut out = WireList {
        components: Vec::new(),
        nets: Vec::new(),
    };
    let Some(f) = pairs.next() else {
        return Err("empty".into());
    };
    for section in f.into_inner() {
        match section.as_rule() {
            Rule::component_list => {
                for c in section.into_inner() {
                    match c.as_rule() {
                        Rule::component => {
                            let t = texts(c);
                            if let [comment, designator, footprint] = t.as_slice() {
                                out.components.push(Component {
                                    comment: comment.clone(),
                                    designator: designator.clone(),
                                    footprint: footprint.clone(),
                                });
                            }
                        }
                        Rule::bad_line => bad_line(notes, file, "component list", &c),
                        _ => {}
                    }
                }
            }
            Rule::wire_list => {
                for n in section.into_inner().filter(|n| n.as_rule() == Rule::net) {
                    let mut net = WireNet {
                        name: String::new(),
                        connections: Vec::new(),
                    };
                    for c in n.into_inner() {
                        match c.as_rule() {
                            Rule::net_name => net.name = c.as_str().trim_end().to_string(),
                            Rule::connection => {
                                let t = texts(c);
                                if let [designator, pin, pin_name, pin_type, part_value] =
                                    t.as_slice()
                                {
                                    net.connections.push(Connection {
                                        designator: designator.clone(),
                                        pin: pin.clone(),
                                        pin_name: pin_name.clone(),
                                        pin_type: pin_type.clone(),
                                        part_value: part_value.clone(),
                                    });
                                }
                            }
                            Rule::bad_line => {
                                let at = format!("net {}", net.name);
                                bad_line(notes, file, &at, &c);
                            }
                            _ => {}
                        }
                    }
                    out.nets.push(net);
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// The column texts of a line, trimmed
fn texts(p: Pair<Rule>) -> Vec<String> {
    p.into_inner()
        .map(|c| c.as_str().trim().to_string())
        .collect()
}

fn bad_line(notes: &mut Notes, file: &str, at: &str, p: &Pair<Rule>) {
    let (line, _) = p.line_col();
    notes.invalid(
        file,
        at,
        format!(
            "line {line} does not fit the columns, not read: `{}`",
            p.as_str().trim()
        ),
    );
}
