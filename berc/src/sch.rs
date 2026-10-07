//! `.kicad_sch` to sorted text: title block, sheets, symbols with their fields and variant overrides, power
//! symbols and no-connects counted, labels, text, tables, drawing counts and library symbol hashes.

use crate::common::{
    Counted, Options, Out, SortKey, at_of, compact, hash, natural_cmp, q, quote, sort_natural,
};
use crate::sexpr::Sexp;
use std::collections::{BTreeMap, HashMap};

/// Symbol flags and how they print when they differ from the KiCad default
const FLAGS: &[(&str, bool, &str)] = &[
    ("dnp", false, "dnp"),
    ("in_bom", true, "no_bom"),
    ("on_board", true, "no_board"),
    ("in_pos_files", true, "no_pos"),
    ("exclude_from_sim", false, "no_sim"),
];

/// Properties shown on the symbol line itself, or never shown
const MAIN_FIELDS: &[&str] = &["Reference", "Value", "Footprint"];

pub fn render(root: &Sexp, opts: Options) -> String {
    let items = root.args();
    let libs: HashMap<&str, &Sexp> = root
        .child("lib_symbols")
        .map(|l| {
            l.children("symbol")
                .filter_map(|s| Some((s.arg(0)?, s)))
                .collect()
        })
        .unwrap_or_default();

    let mut out = Out::default();
    out.line(header("kicad_sch", root));

    let mut paper = None;
    let mut title = Out::default();
    let mut symbols: Vec<Symbol> = Vec::new();
    let mut power = Counted::default();
    let mut no_connects: Vec<(f64, f64, String)> = Vec::new();
    let mut sheets = Vec::new();
    let mut sheet_pins = Vec::new();
    let mut labels = Counted::default();
    let mut texts = Counted::default();
    let mut tables = Vec::new();
    let mut drawing = Counted::default();
    let mut library = Vec::new();
    let mut other = Counted::default();

    for item in items {
        let Some(head) = item.head() else {
            other.add(compact(item, opts));
            continue;
        };
        match head {
            "version" | "generator" | "generator_version" | "uuid" | "sheet_instances"
            | "symbol_instances" => {}
            "paper" => paper = Some(format!("paper {}", args_text(item))),
            "title_block" => title_block(item, &mut title),
            "lib_symbols" => {
                for s in item.children("symbol") {
                    let name = s.arg(0).unwrap_or("?");
                    let hash = hash(&compact(
                        s,
                        Options {
                            with_geometry: true,
                        },
                    ));
                    library.push(format!("lib {}  {hash}", q(name)));
                }
            }
            "symbol" => {
                let sym = Symbol::read(item, &libs, opts);
                if sym.power {
                    let mut line = format!("power {}", q(&sym.value));
                    if opts.with_geometry {
                        line += &format!(" {}", at_of(item));
                    }
                    power.add(line);
                } else {
                    symbols.push(sym);
                }
            }
            "no_connect" => {
                let at = item.child("at");
                let x = at.and_then(|a| a.num(0)).unwrap_or(f64::NAN);
                let y = at.and_then(|a| a.num(1)).unwrap_or(f64::NAN);
                no_connects.push((x, y, at_of(item)));
            }
            "sheet" => {
                sheets.push(sheet(item, opts));
                sheet_pins.extend(sheet_pin_positions(item));
            }
            "label" | "global_label" | "hierarchical_label" | "directive_label"
            | "netclass_flag" => {
                labels.add(label(item, head, opts));
            }
            "text" | "text_box" => {
                let mut line = format!("{head} {}", quote(item.arg(0).unwrap_or("")));
                if opts.with_geometry {
                    line += &format!(" {}", at_of(item));
                }
                texts.add(line);
            }
            "table" => tables.push(table(item)),
            "wire" | "bus" | "polyline" if opts.with_geometry => {
                drawing.add(format!("{head} {}", points(item)))
            }
            "junction" | "bus_entry" if opts.with_geometry => {
                drawing.add(format!("{head} {}", at_of(item)))
            }
            "wire" | "bus" | "junction" | "bus_entry" | "polyline" | "rectangle" | "circle"
            | "arc" | "bezier" => {
                drawing.add(head.to_string());
            }
            "image" => drawing.add(format!("image {}", hash(item.value("data").unwrap_or("")))),
            "embedded_files" => {
                for f in item.children("file") {
                    let name = f.value("name").unwrap_or("?");
                    let kind = f.value("type").unwrap_or("?");
                    let data = hash(f.value("data").unwrap_or(""));
                    other.add(format!("embedded_file {} {kind} {data}", q(name)));
                }
            }
            "group" => {
                let members = item.child("members").map_or(0, |m| m.args().len());
                other.add(format!(
                    "group {} members={members}",
                    q(item.arg(0).unwrap_or(""))
                ));
            }
            _ => other.add(compact(item, opts)),
        }
    }

    // Fixed order, whatever the file order
    if let Some(p) = paper {
        out.line(p);
    }
    out.append(title);
    let (nc_lines, nc_count) = no_connect_lines(&no_connects, &symbols, &sheet_pins, &libs, opts);
    let symbol_count = symbols.len();
    let symbol_blocks = symbol_blocks(symbols, opts);

    sort_natural(&mut sheets);
    tables.sort();
    sort_natural(&mut library);
    out.section("sheets", sheets.len(), &sheets);
    out.section("symbols", symbol_count, &symbol_blocks);
    out.section("power symbols", power.total(), &power.lines());
    out.section("no_connect", nc_count, &nc_lines);
    out.section("labels", labels.total(), &labels.lines());
    out.section("text", texts.total(), &texts.lines());
    out.section("tables", tables.len(), &tables);
    out.section("drawing", drawing.total(), &drawing.lines());
    out.section("library symbols", library.len(), &library);
    out.section("other", other.total(), &other.lines());
    out.finish()
}

/// `kicad_sch version 20260306, eeschema 10.0`
pub fn header(kind: &str, root: &Sexp) -> String {
    let mut line = format!("{kind} version {}", root.value("version").unwrap_or("?"));
    if let Some(g) = root.value("generator") {
        line += &format!(", {g}");
        if let Some(v) = root.value("generator_version") {
            line += &format!(" {v}");
        }
    }
    line
}

/// Title block fields, one per line: `title "..."`, `comment 1 "..."`
pub fn title_block(item: &Sexp, out: &mut Out) {
    for f in item.args() {
        let Some(head) = f.head() else { continue };
        let values: Vec<String> = f.args().iter().filter_map(Sexp::text).map(q).collect();
        out.line(format!("{head} {}", values.join(" ")));
    }
}

fn args_text(item: &Sexp) -> String {
    item.args()
        .iter()
        .filter_map(Sexp::text)
        .map(q)
        .collect::<Vec<_>>()
        .join(" ")
}

/// `(pts (xy 1 2) (xy 3 4))` as `(1,2)-(3,4)`, endpoints in a stable order for two-point items
fn points(item: &Sexp) -> String {
    let mut pts: Vec<String> = item
        .child("pts")
        .map(|p| {
            p.children("xy")
                .map(|xy| {
                    format!(
                        "({},{})",
                        xy.arg(0).unwrap_or("?"),
                        xy.arg(1).unwrap_or("?")
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    if pts.len() == 2 {
        pts.sort();
    }
    pts.join("-")
}

fn flags_of(item: &Sexp) -> Vec<&'static str> {
    FLAGS
        .iter()
        .filter(|(name, default, _)| item.flag(name).is_some_and(|v| v != *default))
        .map(|(_, _, shown)| *shown)
        .collect()
}

/// Non-empty properties other than the main ones, sorted: `Name=value`
fn fields_of(item: &Sexp, skip: &[&str]) -> Vec<(String, String)> {
    let mut fields: Vec<(String, String)> = item
        .children("property")
        .filter_map(|p| Some((p.arg(0)?, p.arg(1).unwrap_or(""))))
        .filter(|(name, value)| {
            !skip.contains(name) && !value.is_empty() && *value != "~" && !name.starts_with("ki_")
        })
        .map(|(n, v)| (n.to_string(), v.to_string()))
        .collect();
    fields.sort_by(|a, b| natural_cmp(&a.0, &b.0).then_with(|| a.cmp(b)));
    fields
}

fn field_line(name: &str, value: &str) -> String {
    format!("{}={}", q(name), q(value))
}

struct Symbol {
    reference: String,
    unit: u32,
    body_style: u32,
    lib_id: String,
    /// Key into lib_symbols: `lib_name` when the symbol was edited locally, else `lib_id`
    lib_key: String,
    value: String,
    footprint: String,
    power: bool,
    flags: Vec<&'static str>,
    fields: Vec<(String, String)>,
    /// Pin alternates, other instances, variant overrides: lines under the symbol
    extra: Vec<String>,
    at: (f64, f64, f64),
    mirror: Option<String>,
    geometry: String,
}

impl Symbol {
    fn read(item: &Sexp, libs: &HashMap<&str, &Sexp>, opts: Options) -> Symbol {
        let prop = |name: &str| {
            item.children("property")
                .find(|p| p.arg(0) == Some(name))
                .and_then(|p| p.arg(1))
                .unwrap_or("")
                .to_string()
        };
        let lib_id = item.value("lib_id").unwrap_or("").to_string();
        let lib_name = item.value("lib_name").unwrap_or(&lib_id).to_string();
        let mut reference = prop("Reference");
        let value = prop("Value");
        let lib_power = libs
            .get(lib_name.as_str())
            .is_some_and(|l| l.child("power").is_some());
        let power = lib_power
            || lib_id.starts_with("power:")
            || reference.starts_with("#PWR")
            || reference.starts_with("#FLG");
        let unit = item.value("unit").and_then(|u| u.parse().ok()).unwrap_or(1);
        let body_style = item
            .value("body_style")
            .or(item.value("convert"))
            .and_then(|u| u.parse().ok())
            .unwrap_or(1);
        let flags = flags_of(item);
        let fields = fields_of(item, MAIN_FIELDS);

        let mut extra = Vec::new();
        if body_style != 1 {
            extra.push(format!("body_style {body_style}"));
        }
        for pin in item.children("pin") {
            if let Some(alt) = pin.value("alternate") {
                extra.push(format!(
                    "pin {} alt={}",
                    q(pin.arg(0).unwrap_or("?")),
                    q(alt)
                ));
            }
        }

        // Instances: the reference per sheet path, and the variant overrides
        let mut base: BTreeMap<String, String> = fields.iter().cloned().collect();
        base.insert("Value".into(), value.clone());
        base.insert("Footprint".into(), prop("Footprint"));
        let paths: Vec<&Sexp> = item
            .children("instances")
            .flat_map(|i| i.children("project"))
            .flat_map(|p| p.children("path"))
            .collect();
        let instance_refs: Vec<&str> = paths.iter().filter_map(|p| p.value("reference")).collect();
        if (reference.is_empty() || reference.ends_with('?')) && !instance_refs.is_empty() {
            reference = instance_refs[0].to_string();
        }
        let mut other_refs: Vec<&str> = instance_refs
            .iter()
            .copied()
            .filter(|r| *r != reference)
            .collect();
        other_refs.sort_by(|a, b| natural_cmp(a, b));
        other_refs.dedup();
        for r in other_refs {
            extra.push(format!("instance {}", q(r)));
        }
        for path in &paths {
            let r = path.value("reference").unwrap_or(&reference);
            let who = if r == reference {
                String::new()
            } else {
                format!(" ({})", q(r))
            };
            for variant in path.children("variant") {
                let name = q(variant.value("name").unwrap_or("?"));
                for line in variant_overrides(variant, item, &base, opts) {
                    extra.push(format!("variant {name}{who} {line}"));
                }
            }
        }

        let at = item.child("at");
        let at3 = (
            at.and_then(|a| a.num(0)).unwrap_or(f64::NAN),
            at.and_then(|a| a.num(1)).unwrap_or(f64::NAN),
            at.and_then(|a| a.num(2)).unwrap_or(0.0),
        );
        let mirror = item.value("mirror").map(str::to_string);
        let mut geometry = at_of(item);
        if let Some(m) = &mirror {
            geometry += &format!(" mirror={m}");
        }
        Symbol {
            reference,
            unit,
            body_style,
            footprint: prop("Footprint"),
            lib_id,
            lib_key: lib_name,
            value,
            power,
            flags,
            fields,
            extra,
            at: at3,
            mirror,
            geometry,
        }
    }

    fn lib_name(&self) -> &str {
        &self.lib_key
    }
}

/// What a variant changes, compared with the symbol itself: `dnp=yes`, `Value=255`
fn variant_overrides(
    variant: &Sexp,
    symbol: &Sexp,
    base: &BTreeMap<String, String>,
    opts: Options,
) -> Vec<String> {
    let mut lines = Vec::new();
    for c in variant.args() {
        match c.head() {
            Some("name") => {}
            Some("field") => {
                let name = c.value("name").unwrap_or("?");
                let value = c.value("value").unwrap_or("");
                if base.get(name).map_or("", String::as_str) != value {
                    lines.push(field_line(name, value));
                }
            }
            Some(flag) if FLAGS.iter().any(|f| f.0 == flag) => {
                let default = FLAGS.iter().find(|f| f.0 == flag).is_some_and(|f| f.1);
                let base_value = symbol.flag(flag).unwrap_or(default);
                let value = !matches!(c.arg(0), Some("no" | "false"));
                if value != base_value {
                    lines.push(format!("{flag}={}", if value { "yes" } else { "no" }));
                }
            }
            _ => lines.push(compact(c, opts)),
        }
    }
    lines
}

/// Symbols grouped by reference (units of one part together), as blocks sorted by reference
fn symbol_blocks(symbols: Vec<Symbol>, opts: Options) -> Vec<String> {
    let mut groups: BTreeMap<SortKey, Vec<Symbol>> = BTreeMap::new();
    let mut unannotated = Vec::new();
    for s in symbols {
        if s.reference.ends_with('?') || s.reference.is_empty() {
            unannotated.push(vec![s]);
        } else {
            groups
                .entry(SortKey(s.reference.clone()))
                .or_default()
                .push(s);
        }
    }
    let mut blocks: Vec<String> = groups
        .into_values()
        .chain(unannotated)
        .map(|g| symbol_block(g, opts))
        .collect();
    sort_natural(&mut blocks);
    blocks
}

fn symbol_block(mut units: Vec<Symbol>, opts: Options) -> String {
    units.sort_by_key(|s| (s.unit, s.body_style));
    let first = &units[0];
    let mut head = format!(
        "{} {}  lib={}",
        q(&first.reference),
        q(&first.value),
        q(&first.lib_id)
    );
    if !first.footprint.is_empty() {
        head += &format!("  fp={}", q(&first.footprint));
    }
    let multi = units.len() > 1 || first.unit != 1;
    if multi {
        let list: Vec<String> = units.iter().map(|u| u.unit.to_string()).collect();
        head += &format!("  units={}", list.join(","));
    }
    for f in &first.flags {
        head += &format!("  {f}");
    }
    if opts.with_geometry && !multi {
        head += &format!("  {}", first.geometry);
    }
    let mut lines = vec![head];
    let mut sub: Vec<String> = first.fields.iter().map(|(n, v)| field_line(n, v)).collect();
    sub.extend(first.extra.iter().cloned());
    for u in &units[1..] {
        let tag = format!("[unit {}]", u.unit);
        for f in &u.fields {
            if !first.fields.contains(f) {
                sub.push(format!("{tag} {}", field_line(&f.0, &f.1)));
            }
        }
        if u.value != first.value
            || u.lib_id != first.lib_id
            || u.footprint != first.footprint
            || u.flags != first.flags
        {
            sub.push(format!(
                "{tag} {} lib={} fp={} {}",
                q(&u.value),
                q(&u.lib_id),
                q(&u.footprint),
                u.flags.join(" ")
            ));
        }
        for e in &u.extra {
            if !first.extra.contains(e) {
                sub.push(format!("{tag} {e}"));
            }
        }
    }
    if opts.with_geometry && multi {
        for u in &units {
            sub.push(format!("[unit {}] {}", u.unit, u.geometry));
        }
    }
    sort_natural(&mut sub);
    sub.dedup();
    lines.extend(sub.into_iter().map(|s| format!("  {s}")));
    lines.join("\n")
}

/// A pin of a placed symbol, in schematic coordinates
struct PlacedPin<'a> {
    x: f64,
    y: f64,
    reference: &'a str,
    number: &'a str,
    name: &'a str,
}

/// Pins of a symbol on the sheet: the lib pins of its unit and body style, moved, rotated and mirrored
fn placed_pins<'a>(sym: &'a Symbol, libs: &HashMap<&str, &'a Sexp<'a>>) -> Vec<PlacedPin<'a>> {
    let Some(lib) = libs.get(sym.lib_name()) else {
        return Vec::new();
    };
    let mut pins = Vec::new();
    for unit_sym in lib.children("symbol") {
        // Sub-symbols are named NAME_unit_style, unit 0 / style 0 meaning "all"
        let name = unit_sym.arg(0).unwrap_or("");
        let mut parts = name.rsplitn(3, '_');
        let style: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let unit: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        if (unit != 0 && unit != sym.unit) || (style != 0 && style != sym.body_style) {
            continue;
        }
        for pin in unit_sym.children("pin") {
            let Some(at) = pin.child("at") else { continue };
            let (px, py) = (at.num(0).unwrap_or(0.0), at.num(1).unwrap_or(0.0));
            let (x, y) = place(px, py, sym.at.2, sym.mirror.as_deref());
            let num_of = |head: &str| pin.child(head).and_then(|c| c.arg(0)).unwrap_or("");
            pins.push(PlacedPin {
                x: sym.at.0 + x,
                y: sym.at.1 + y,
                reference: &sym.reference,
                number: num_of("number"),
                name: num_of("name"),
            });
        }
    }
    pins
}

/// Lib coordinates (Y up) to sheet offsets (Y down): flip Y, rotate counter-clockwise on screen, then mirror
fn place(px: f64, py: f64, rot: f64, mirror: Option<&str>) -> (f64, f64) {
    let (mut x, mut y) = (px, -py);
    let turns = ((rot / 90.0).round() as i64).rem_euclid(4);
    for _ in 0..turns {
        (x, y) = (y, -x);
    }
    match mirror {
        Some("x") => y = -y,
        Some("y") => x = -x,
        _ => {}
    }
    (x, y)
}

/// Sheet pins with their sheet name, for no-connects placed on them: `(x, y, "sheet USB pin D+")`
fn sheet_pin_positions(sheet: &Sexp) -> Vec<(f64, f64, String)> {
    let name = sheet
        .children("property")
        .find(|p| matches!(p.arg(0), Some("Sheetname" | "Sheet name")))
        .and_then(|p| p.arg(1))
        .unwrap_or("");
    sheet
        .children("pin")
        .filter_map(|pin| {
            let at = pin.child("at")?;
            Some((
                at.num(0)?,
                at.num(1)?,
                format!("sheet {} pin {}", q(name), q(pin.arg(0).unwrap_or(""))),
            ))
        })
        .collect()
}

/// No-connect flags named by the pin they sit on (`nc U201.5 PA3`), the rest counted as free
fn no_connect_lines(
    ncs: &[(f64, f64, String)],
    symbols: &[Symbol],
    sheet_pins: &[(f64, f64, String)],
    libs: &HashMap<&str, &Sexp>,
    opts: Options,
) -> (Vec<String>, usize) {
    if ncs.is_empty() {
        return (Vec::new(), 0);
    }
    let pins: Vec<PlacedPin> = symbols.iter().flat_map(|s| placed_pins(s, libs)).collect();
    let near = |px: f64, py: f64, x: f64, y: f64| (px - x).abs() < 0.01 && (py - y).abs() < 0.01;
    let mut lines = Counted::default();
    for (x, y, at) in ncs {
        let hit = pins.iter().find(|p| near(p.x, p.y, *x, *y));
        let mut line = match hit {
            Some(p) if p.name.is_empty() || p.name == "~" => {
                format!("nc {}.{}", q(p.reference), q(p.number))
            }
            Some(p) => format!("nc {}.{} {}", q(p.reference), q(p.number), q(p.name)),
            None => match sheet_pins.iter().find(|p| near(p.0, p.1, *x, *y)) {
                Some(p) => format!("nc {}", p.2),
                None => "nc (not on a pin)".to_string(),
            },
        };
        if opts.with_geometry {
            line += &format!(" {at}");
        }
        lines.add(line);
    }
    (lines.lines(), ncs.len())
}

fn sheet(item: &Sexp, opts: Options) -> String {
    let prop = |names: &[&str]| {
        item.children("property")
            .find(|p| p.arg(0).is_some_and(|n| names.contains(&n)))
            .and_then(|p| p.arg(1))
            .unwrap_or("")
            .to_string()
    };
    let name_keys = ["Sheetname", "Sheet name"];
    let file_keys = ["Sheetfile", "Sheet file"];
    let mut head = format!(
        "sheet {}  file={}",
        q(&prop(&name_keys)),
        q(&prop(&file_keys))
    );
    let mut pages: Vec<&str> = item
        .children("instances")
        .flat_map(|i| i.children("project"))
        .flat_map(|p| p.children("path"))
        .filter_map(|p| p.value("page"))
        .collect();
    pages.sort_by(|a, b| natural_cmp(a, b));
    pages.dedup();
    if !pages.is_empty() {
        head += &format!("  page={}", pages.join(","));
    }
    for f in flags_of(item) {
        head += &format!("  {f}");
    }
    if opts.with_geometry {
        let size = item.child("size").map(|s| {
            format!(
                " size={}x{}",
                s.arg(0).unwrap_or("?"),
                s.arg(1).unwrap_or("?")
            )
        });
        head += &format!("  {}{}", at_of(item), size.unwrap_or_default());
    }
    let mut sub: Vec<String> = fields_of(
        item,
        &[name_keys[0], name_keys[1], file_keys[0], file_keys[1]],
    )
    .iter()
    .map(|(n, v)| field_line(n, v))
    .collect();
    for pin in item.children("pin") {
        let mut line = format!(
            "pin {} {}",
            q(pin.arg(0).unwrap_or("")),
            pin.arg(1).unwrap_or("?")
        );
        if opts.with_geometry {
            line += &format!(" {}", at_of(pin));
        }
        sub.push(line);
    }
    sort_natural(&mut sub);
    std::iter::once(head)
        .chain(sub.into_iter().map(|s| format!("  {s}")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn label(item: &Sexp, head: &str, opts: Options) -> String {
    let kind = if head == "hierarchical_label" {
        "hier_label"
    } else {
        head
    };
    let mut line = format!("{kind} {}", q(item.arg(0).unwrap_or("")));
    if let Some(shape) = item.value("shape") {
        line += &format!(" {shape}");
    }
    if matches!(head, "directive_label" | "netclass_flag") {
        for (n, v) in fields_of(item, &[]) {
            line += &format!(" {}", field_line(&n, &v));
        }
    }
    if opts.with_geometry {
        line += &format!(" {}", at_of(item));
    }
    line
}

/// A table as rows of cells: `table 3 cols` then `  | a | b | c |`
pub fn table(item: &Sexp) -> String {
    let cols: usize = item
        .value("column_count")
        .and_then(|c| c.parse().ok())
        .unwrap_or(1)
        .max(1);
    let cells: Vec<String> = item
        .child("cells")
        .map(|c| {
            c.children("table_cell")
                .map(|cell| quote(cell.arg(0).unwrap_or("")))
                .collect()
        })
        .unwrap_or_default();
    let mut lines = vec![format!("table {cols} cols")];
    for row in cells.chunks(cols) {
        lines.push(format!("  | {} |", row.join(" | ")));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sexpr::parse;

    fn render_str(text: &str) -> String {
        let v = parse(text).unwrap();
        render(&v[0], Options::default())
    }

    #[test]
    fn rotation_and_mirror() {
        // A pin 3.81 above the origin in the lib (Y up) is 3.81 above on the sheet (Y down) at rotation 0
        assert_eq!(place(0.0, 3.81, 0.0, None), (0.0, -3.81));
        // Rotated 90° counter-clockwise: up turns to the left
        let (x, y) = place(0.0, 3.81, 90.0, None);
        assert!((x + 3.81).abs() < 1e-9 && y.abs() < 1e-9, "{x} {y}");
        assert_eq!(place(1.0, 0.0, 0.0, Some("y")), (-1.0, 0.0));
    }

    #[test]
    fn variant_shows_only_differences() {
        let out = render_str(
            r#"(kicad_sch (version 1)
            (symbol (lib_id "Device:R") (unit 1) (dnp no)
              (property "Reference" "R1") (property "Value" "10k") (property "Footprint" "R_0402")
              (instances (project "p" (path "/a" (reference "R1") (unit 1)
                (variant (name "Main"))
                (variant (name "Lite") (dnp yes) (field (name "Value") (value "10k")))
                (variant (name "Plus") (field (name "Value") (value "4k7"))))))))"#,
        );
        assert!(out.contains("R1 10k  lib=Device:R  fp=R_0402\n"), "{out}");
        assert!(out.contains("  variant Lite dnp=yes\n"), "{out}");
        assert!(out.contains("  variant Plus Value=4k7\n"), "{out}");
        assert!(!out.contains("Main"), "{out}");
        assert!(!out.contains("Lite Value"), "{out}");
    }

    #[test]
    fn units_grouped_and_sorted() {
        let sym = |unit: u32, r: &str| {
            format!(
                r#"(symbol (lib_id "MCU:X") (unit {unit}) (property "Reference" "{r}") (property "Value" "X")
                   (property "Mpn" "ABC"))"#
            )
        };
        let text = format!(
            "(kicad_sch {} {} {} {})",
            sym(2, "U10"),
            sym(1, "U2"),
            sym(1, "U10"),
            sym(1, "R1")
        );
        let out = render_str(&text);
        let r1 = out.find("R1 X").unwrap();
        let u2 = out.find("U2 X").unwrap();
        let u10 = out.find("U10 X  lib=MCU:X  units=1,2").unwrap();
        assert!(r1 < u2 && u2 < u10, "{out}");
        assert!(out.contains("# symbols (4)"), "{out}");
        assert_eq!(out.matches("Mpn=ABC").count(), 3, "{out}");
    }

    #[test]
    fn power_labels_text_counted() {
        let out = render_str(
            r##"(kicad_sch
            (symbol (lib_id "power:GND") (property "Reference" "#PWR01") (property "Value" "GND"))
            (symbol (lib_id "power:GND") (property "Reference" "#PWR02") (property "Value" "GND"))
            (label "SDA" (at 1 2 0)) (label "SDA" (at 3 4 0))
            (global_label "VBUS" (shape input) (at 0 0 0) (property "Intersheetrefs" "${INTERSHEET_REFS}"))
            (hierarchical_label "LED" (shape output))
            (text "two\nlines" (at 0 0 0))
            (wire (pts (xy 0 0) (xy 1 0))) (wire (pts (xy 0 0) (xy 0 1))) (junction (at 0 0))
            (future_thing "x" (uuid "abc") (at 1 2)))"##,
        );
        assert!(out.contains("power GND x2"), "{out}");
        assert!(out.contains("label SDA x2"), "{out}");
        assert!(out.contains("global_label VBUS input"), "{out}");
        assert!(out.contains("hier_label LED output"), "{out}");
        assert!(out.contains(r#"text "two\nlines""#), "{out}");
        assert!(out.contains("wire x2"), "{out}");
        assert!(out.contains("junction\n"), "{out}");
        assert!(out.contains(r#"(future_thing "x")"#), "{out}");
        assert!(!out.contains("abc"), "{out}");
    }

    #[test]
    fn no_connect_on_pin() {
        let out = render_str(
            r#"(kicad_sch
            (lib_symbols (symbol "Device:R" (symbol "R_1_1"
              (pin passive line (at 0 3.81 270) (length 1.27) (name "~") (number "1"))
              (pin passive line (at 0 -3.81 90) (length 1.27) (name "B") (number "2")))))
            (symbol (lib_id "Device:R") (at 10 20 90) (unit 1) (property "Reference" "R1") (property "Value" "1k"))
            (no_connect (at 6.19 20)) (no_connect (at 13.81 20)) (no_connect (at 50 50)))"#,
        );
        assert!(out.contains("nc R1.1\n"), "{out}");
        assert!(out.contains("nc R1.2 B\n"), "{out}");
        assert!(out.contains("nc (not on a pin)\n"), "{out}");
    }
}
