//! The canonical dump: a design as sorted text, one fact per line, deterministic, in the style of `berc
//! textconv`. It is the golden-file format of every importer test (a parser change shows as a readable diff) and
//! it is lossless: [`read`] gives back the design that [`write`] wrote, so `import → dump → import` is checked.
//!
//! A line is tokens separated by spaces: bare words, or quoted strings with `\"`, `\\`, `\n`, `\t`, `\r`
//! escapes. An entity line names the entity and its structural attributes as keyword-value pairs:
//!
//! ```text
//! part R1 lib Device:R sheet /Power/ symbol Device:R
//! pin R1 2 name ~ unit 1 net /Power/VIN
//! ```
//!
//! A fact line ends in `= value @ source`, with the rank when it is not the source's default, the time and the
//! note when there are any:
//!
//! ```text
//! part R1 field Value = 10k @ sch power.kicad_sch
//! part R1 flag dnp = yes @ manual Roman rank 1000 at 2026-10-22T00:00:00Z note "fitted later"
//! ```
//!
//! Entities with the same key (two `R?`, two pads `MP`) carry `#2`, `#3` after the key in canonical order.
//! Coordinates are millimetres (`x,y`), lengths millimetres, rotations degrees.

use crate::canon::natural_cmp;
use crate::design::*;
use crate::fact::{Fact, Facts, Provenance, Rank, Time};
use crate::layout::*;
use crate::value::{Quantity, Unit, Value};
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------------------------------------------
// Tokens

/// A token as written: bare when it can be, quoted otherwise
pub fn tok(s: &str) -> String {
    let bare = !s.is_empty()
        && s != "="
        && s != "@"
        && !s.starts_with('#')
        && s.chars()
            .all(|c| !c.is_whitespace() && c != '"' && c != '\\' && !c.is_control());
    if bare {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A token read back: its text, and whether it was quoted (only bare `=`, `@` and `#n` are syntax)
#[derive(Clone, Debug, PartialEq)]
struct Tok {
    text: String,
    quoted: bool,
}

fn tokenize(line: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == ' ' {
            chars.next();
            continue;
        }
        if c == '"' {
            chars.next();
            let mut s = String::new();
            loop {
                match chars.next() {
                    None => return Err("unterminated string".into()),
                    Some('"') => break,
                    Some('\\') => match chars.next() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        _ => return Err("bad escape".into()),
                    },
                    Some(c) => s.push(c),
                }
            }
            out.push(Tok {
                text: s,
                quoted: true,
            });
        } else {
            let mut s = String::new();
            while let Some(&c) = chars.peek() {
                if c == ' ' {
                    break;
                }
                s.push(c);
                chars.next();
            }
            out.push(Tok {
                text: s,
                quoted: false,
            });
        }
    }
    Ok(out)
}

fn yes(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

fn dup(k: usize) -> Option<String> {
    (k > 1).then(|| format!("#{k}"))
}

/// Appends the `= value @ source ...` tail of a fact
fn fact_tail<T>(out: &mut String, f: &Fact<T>, value: impl FnOnce(&T) -> String) {
    out.push_str(" = ");
    out.push_str(&value(&f.value));
    out.push_str(" @ ");
    out.push_str(f.source.kind());
    for a in f.source.args() {
        out.push(' ');
        out.push_str(&tok(a));
    }
    if f.rank != f.source.default_rank() {
        out.push_str(&format!(" rank {}", f.rank.0));
    }
    if let Some(t) = f.at {
        out.push_str(&format!(" at {t}"));
    }
    if let Some(n) = &f.note {
        out.push_str(" note ");
        out.push_str(&tok(n));
    }
}

fn fact_lines<T>(
    out: &mut Vec<String>,
    head: &str,
    facts: &Facts<T>,
    value: impl Fn(&T) -> String,
) {
    for f in facts.iter() {
        let mut line = head.to_string();
        fact_tail(&mut line, f, &value);
        out.push(line);
    }
}

fn value_tokens(v: &Value) -> String {
    match v {
        Value::Quantity(q) => format!("qty {} {}", q.value, q.unit.name()),
        Value::Text(t) => format!("text {}", tok(t)),
    }
}

fn layers(l: &[String]) -> String {
    tok(&l.join(","))
}

// ---------------------------------------------------------------------------------------------------------------
// Writer

/// The design as canonical text. Each board starts with its `board` line; its lines follow, sorted naturally.
pub fn write(design: &Design) -> String {
    let mut out = String::new();
    for b in &design.boards {
        out.push_str(&format!("board {}\n", tok(&b.name)));
        let mut lines = board_lines(b);
        lines.sort_by(|a, b| natural_cmp(a, b));
        for l in lines {
            out.push_str(&l);
            out.push('\n');
        }
    }
    let mut links: Vec<String> = design
        .links
        .iter()
        .map(|l| {
            let ba = design.board(l.a.0);
            let bb = design.board(l.b.0);
            let mut line = format!(
                "link {} {} {} {}",
                tok(&ba.name),
                tok(&ba.part(l.a.1).designator),
                tok(&bb.name),
                tok(&bb.part(l.b.1).designator)
            );
            match &l.map {
                LinkMap::ByNumber => line.push_str(" by-number"),
                LinkMap::Table(t) => {
                    line.push_str(" table");
                    for (a, b) in t {
                        line.push_str(&format!(" {} {}", tok(a), tok(b)));
                    }
                }
            }
            line.push_str(" @ ");
            line.push_str(l.source.kind());
            for a in l.source.args() {
                line.push(' ');
                line.push_str(&tok(a));
            }
            line
        })
        .collect();
    links.sort_by(|a, b| natural_cmp(a, b));
    for l in links {
        out.push_str(&l);
        out.push('\n');
    }
    out
}

/// Key tokens of every part, pin, net, footprint and pad, with `#n` on repeats
struct Keys {
    part: Vec<String>,
    pin: Vec<String>,
    net: Vec<String>,
    fp: Vec<String>,
    pad: Vec<String>,
}

fn keyed<'a>(names: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    names
        .map(|n| {
            let k = seen.entry(n.to_string()).or_insert(0);
            *k += 1;
            match dup(*k) {
                Some(d) => format!("{} {d}", tok(n)),
                None => tok(n),
            }
        })
        .collect()
}

impl Keys {
    fn new(b: &Board) -> Keys {
        let part = keyed(b.parts.iter().map(|p| p.designator.as_str()));
        let net = keyed(b.nets.iter().map(|n| n.name.as_str()));
        let mut pin = Vec::with_capacity(b.pins.len());
        let mut seen: HashMap<(PartId, &str), usize> = HashMap::new();
        for p in &b.pins {
            let k = seen.entry((p.part, p.number.as_str())).or_insert(0);
            *k += 1;
            let mut s = format!("{} {}", part[p.part.index()], tok(&p.number));
            if let Some(d) = dup(*k) {
                s += &format!(" {d}");
            }
            pin.push(s);
        }
        let (fp, pad) = match &b.layout {
            Some(l) => {
                let fp = keyed(l.footprints.iter().map(|f| f.reference.as_str()));
                let mut pad = Vec::with_capacity(l.pads.len());
                let mut seen: HashMap<(FootprintId, &str), usize> = HashMap::new();
                for p in &l.pads {
                    let k = seen.entry((p.footprint, p.number.as_str())).or_insert(0);
                    *k += 1;
                    let mut s = format!("{} {}", fp[p.footprint.index()], tok(&p.number));
                    if let Some(d) = dup(*k) {
                        s += &format!(" {d}");
                    }
                    pad.push(s);
                }
                (fp, pad)
            }
            None => (Vec::new(), Vec::new()),
        };
        Keys {
            part,
            pin,
            net,
            fp,
            pad,
        }
    }
}

fn board_lines(b: &Board) -> Vec<String> {
    let keys = Keys::new(b);
    let mut out = Vec::new();
    let sheet = |s: SheetId| tok(&b.sheet_path(s));

    out.push(format!("source tool {}", tok(&b.source.tool)));
    if let Some(v) = &b.source.version {
        out.push(format!("source version {}", tok(v)));
    }
    for f in &b.source.files {
        out.push(format!("source file {}", tok(f)));
    }

    for (i, s) in b.sheets.iter().enumerate() {
        let mut line = format!(
            "sheet {} name {} file {} uuid {}",
            sheet(SheetId::from_index(i)),
            tok(&s.name),
            tok(&s.file),
            tok(&s.uuid_path)
        );
        if let Some(p) = &s.page {
            line += &format!(" page {}", tok(p));
        }
        out.push(line);
    }

    for s in &b.symbols {
        let n = tok(&s.name);
        let mut line = format!("symbol {n} units {}", s.units);
        match s.power {
            Some(PowerScope::Global) => line += " power global",
            Some(PowerScope::Local) => line += " power local",
            None => {}
        }
        if s.duplicate_pins_are_jumpers {
            line += " jumpers";
        }
        out.push(line);
        if !s.description.is_empty() {
            out.push(format!("symbol {n} description {}", tok(&s.description)));
        }
        if !s.keywords.is_empty() {
            out.push(format!("symbol {n} keywords {}", tok(&s.keywords)));
        }
        for f in &s.footprint_filters {
            out.push(format!("symbol {n} fp-filter {}", tok(f)));
        }
        for (k, v) in &s.fields {
            out.push(format!("symbol {n} field {} {}", tok(k), tok(v)));
        }
        if let Some(h) = &s.hash {
            out.push(format!("symbol {n} hash {}", tok(h)));
        }
        for g in &s.jumper_pin_groups {
            let g: Vec<String> = g.iter().map(|x| tok(x)).collect();
            out.push(format!("symbol {n} jumper-group {}", g.join(" ")));
        }
        for p in &s.pins {
            let key = format!(
                "symbol {n} pin {} unit {} body {}",
                tok(&p.number),
                p.unit,
                p.body_style
            );
            let mut line = format!(
                "{key} name {} type {} style {}",
                tok(&p.name),
                p.etype.name(),
                tok(&p.style)
            );
            if p.hidden {
                line += " hidden";
            }
            out.push(line);
            for a in &p.alternates {
                out.push(format!(
                    "{key} alt {} type {} style {}",
                    tok(&a.name),
                    a.etype.name(),
                    tok(&a.style)
                ));
            }
        }
    }

    for v in &b.variants {
        let mut line = format!("variant {}", tok(&v.name));
        if let Some(d) = &v.description {
            line += &format!(" description {}", tok(d));
        }
        out.push(line);
    }

    for c in &b.net_classes {
        let n = tok(&c.name);
        out.push(format!("netclass {n}"));
        for p in &c.patterns {
            out.push(format!("netclass {n} pattern {}", tok(p)));
        }
        for a in &c.assigned {
            out.push(format!("netclass {n} assign {}", tok(a)));
        }
        for p in &c.params {
            fact_lines(
                &mut out,
                &format!("netclass {n} param {}", tok(&p.key)),
                &p.value,
                |v| tok(v),
            );
        }
    }

    let s = &b.settings;
    for (group, list) in [
        ("textvar", &s.text_vars),
        ("erc-severity", &s.erc_severities),
        ("erc-exclusion", &s.erc_exclusions),
        ("drc-severity", &s.drc_severities),
        ("drc-exclusion", &s.drc_exclusions),
        ("rule", &s.rules),
    ] {
        for p in list {
            fact_lines(
                &mut out,
                &format!("setting {group} {}", tok(&p.key)),
                &p.value,
                |v| tok(v),
            );
        }
    }

    for (i, n) in b.nets.iter().enumerate() {
        let k = &keys.net[i];
        out.push(format!("net {k} kind {}", n.kind.name()));
        fact_lines(&mut out, &format!("net {k} class"), &n.class, |v| tok(v));
    }

    for (i, p) in b.parts.iter().enumerate() {
        let k = &keys.part[i];
        let mut line = format!("part {k} lib {} sheet {}", tok(&p.lib_id), sheet(p.sheet));
        if let Some(s) = p.symbol {
            line += &format!(" symbol {}", tok(&b.symbols[s.index()].name));
        }
        out.push(line);
        for u in &p.units {
            let mut line = format!(
                "part {k} unit {} sheet {} uuid {}",
                u.unit,
                sheet(u.sheet),
                tok(&u.uuid)
            );
            if let Some(pl) = &u.at {
                line += &format!(" at {} rot {}", pl.at, pl.rotation);
                if let Some(m) = pl.mirror {
                    line += &format!(" mirror {m}");
                }
            }
            out.push(line);
        }
        for f in &p.fields {
            fact_lines(
                &mut out,
                &format!("part {k} field {}", tok(&f.name)),
                &f.value,
                |v| tok(v),
            );
        }
        fact_lines(&mut out, &format!("part {k} value"), &p.value, value_tokens);
        flag_lines(&mut out, &format!("part {k}"), &p.flags);
        for v in &p.variants {
            let vk = format!(
                "part {k} variant {}",
                tok(&b.variants[v.variant.index()].name)
            );
            out.push(vk.clone());
            flag_lines(&mut out, &vk, &v.flags);
            for f in &v.fields {
                fact_lines(
                    &mut out,
                    &format!("{vk} field {}", tok(&f.name)),
                    &f.value,
                    |v| tok(v),
                );
            }
        }
        for t in &p.ties {
            let t: Vec<String> = t.iter().map(|x| tok(x)).collect();
            out.push(format!("part {k} tie {}", t.join(" ")));
        }
    }

    for (i, p) in b.pins.iter().enumerate() {
        let k = &keys.pin[i];
        let mut line = format!("pin {k} name {} unit {}", tok(&p.name), p.unit);
        if p.hidden {
            line += " hidden";
        }
        if p.no_connect {
            line += " nc";
        }
        if let Some(a) = &p.alt {
            line += &format!(" alt {}", tok(a));
        }
        if let Some(n) = p.net {
            line += &format!(" net {}", keys.net[n.index()]);
        }
        out.push(line);
        fact_lines(&mut out, &format!("pin {k} type"), &p.etype, |t| {
            t.name().to_string()
        });
    }

    for l in &b.labels {
        out.push(label_line_keys(b, l, &keys.net));
    }
    for n in &b.no_connects {
        out.push(nc_line_keys(b, n, &keys.pin));
    }

    if let Some(lay) = &b.layout {
        layout_lines(lay, &keys, &mut out);
    }
    out
}

fn flag_lines(out: &mut Vec<String>, head: &str, flags: &PartFlags) {
    for name in PartFlags::NAMES {
        if let Some(f) = flags.by_name(name) {
            fact_lines(out, &format!("{head} flag {name}"), f, |v| yes(*v).into());
        }
    }
}

/// The dump line of a label (also its canonical sort key)
pub fn label_line(b: &Board, l: &Label) -> String {
    let keys = keyed(b.nets.iter().map(|n| n.name.as_str()));
    label_line_keys(b, l, &keys)
}

fn label_line_keys(b: &Board, l: &Label, net_keys: &[String]) -> String {
    let mut line = format!(
        "label {} {} {} at {}",
        tok(&b.sheet_path(l.sheet)),
        l.kind.name(),
        tok(&l.text),
        l.at
    );
    if let Some(s) = &l.shape {
        line += &format!(" shape {}", tok(s));
    }
    if let Some(c) = l.child {
        line += &format!(" child {}", tok(&b.sheet_path(c)));
    }
    if let Some(n) = l.net {
        line += &format!(" net {}", net_keys[n.index()]);
    }
    if l.dangling {
        line += " dangling";
    }
    for (k, v) in &l.fields {
        line += &format!(" field {} {}", tok(k), tok(v));
    }
    line
}

/// The dump line of a no-connect flag (also its canonical sort key)
pub fn nc_line(b: &Board, n: &NoConnect) -> String {
    let parts = keyed(b.parts.iter().map(|p| p.designator.as_str()));
    let pin = n.pin.map(|p| {
        let pin = b.pin(p);
        let same = b
            .part(pin.part)
            .pins
            .iter()
            .filter(|q| b.pin(**q).number == pin.number && **q < p)
            .count();
        let mut s = format!("{} {}", parts[pin.part.index()], tok(&pin.number));
        if let Some(d) = dup(same + 1) {
            s += &format!(" {d}");
        }
        s
    });
    nc_line_with(b, n, pin)
}

fn nc_line_keys(b: &Board, n: &NoConnect, pin_keys: &[String]) -> String {
    nc_line_with(b, n, n.pin.map(|p| pin_keys[p.index()].clone()))
}

fn nc_line_with(b: &Board, n: &NoConnect, pin: Option<String>) -> String {
    let mut line = format!("nc {} at {}", tok(&b.sheet_path(n.sheet)), n.at);
    if let Some(p) = pin {
        line += &format!(" pin {p}");
    }
    line
}

/// Tokens of a shape: `F.SilkS width 0.12 line 1,2 3,4`
pub fn shape_tokens(s: &Shape) -> Vec<String> {
    let mut t = vec![tok(&s.layer), "width".into(), Nm(s.width).to_string()];
    if s.filled {
        t.push("filled".into());
    }
    match &s.geom {
        Geom::Line(a, b) => t.extend(["line".into(), a.to_string(), b.to_string()]),
        Geom::Rect(a, b) => t.extend(["rect".into(), a.to_string(), b.to_string()]),
        Geom::Circle { center, radius } => {
            t.extend(["circle".into(), center.to_string(), Nm(*radius).to_string()])
        }
        Geom::Arc { start, mid, end } => t.extend([
            "arc".into(),
            start.to_string(),
            mid.to_string(),
            end.to_string(),
        ]),
        Geom::Poly(pts) => {
            t.push("poly".into());
            t.extend(pts.iter().map(Pt::to_string));
        }
        Geom::Bezier(pts) => {
            t.push("bezier".into());
            t.extend(pts.iter().map(Pt::to_string));
        }
    }
    t
}

/// Tokens of a text: `F.SilkS "R1" at 1,2 size 1`
pub fn text_tokens(t: &BoardText) -> Vec<String> {
    let mut v = vec![
        tok(&t.layer),
        tok(&t.text),
        "at".into(),
        t.at.to_string(),
        "size".into(),
        Nm(t.size).to_string(),
    ];
    if t.hidden {
        v.push("hidden".into());
    }
    if t.mirrored {
        v.push("mirrored".into());
    }
    v
}

/// Tokens of a track
pub fn track_tokens(t: &Track) -> Vec<String> {
    let mut v = vec![tok(&t.layer), t.start.to_string()];
    if let Some(m) = t.mid {
        v.extend(["mid".into(), m.to_string()]);
    }
    v.extend([t.end.to_string(), "width".into(), Nm(t.width).to_string()]);
    if let Some(n) = &t.net_name {
        v.extend(["net".into(), tok(n)]);
    }
    v
}

/// Tokens of a via
pub fn via_tokens(t: &Via) -> Vec<String> {
    let mut v = vec![
        "at".into(),
        t.at.to_string(),
        "size".into(),
        Nm(t.size).to_string(),
        "drill".into(),
        Nm(t.drill).to_string(),
        "layers".into(),
        layers(&t.layers),
        "kind".into(),
        tok(&t.kind),
    ];
    if let Some(n) = &t.net_name {
        v.extend(["net".into(), tok(n)]);
    }
    v
}

/// Tokens of a zone; the outline last
pub fn zone_tokens(z: &Zone) -> Vec<String> {
    let mut v = Vec::new();
    if let Some(n) = &z.name {
        v.extend(["name".into(), tok(n)]);
    }
    if let Some(n) = &z.net_name {
        v.extend(["net".into(), tok(n)]);
    }
    v.extend([
        "layers".into(),
        layers(&z.layers),
        "priority".into(),
        z.priority.to_string(),
        "connect".into(),
        tok(&z.connect_pads),
    ]);
    for (k, x) in [
        ("clearance", z.clearance),
        ("min-thickness", z.min_thickness),
        ("thermal-gap", z.thermal_gap),
        ("thermal-bridge", z.thermal_bridge_width),
    ] {
        if let Some(x) = x {
            v.extend([k.into(), Nm(x).to_string()]);
        }
    }
    if z.filled {
        v.push("filled".into());
    }
    for (k, x) in &z.keepout {
        v.extend(["keepout".into(), tok(k), tok(x)]);
    }
    v.push("outline".into());
    v.extend(z.outline.iter().map(Pt::to_string));
    v
}

fn layout_lines(lay: &Layout, keys: &Keys, out: &mut Vec<String>) {
    let mut line = format!("layout file {}", tok(&lay.file));
    if let Some(t) = lay.thickness {
        line += &format!(" thickness {}", Nm(t));
    }
    out.push(line);
    for l in &lay.layers {
        let mut line = format!("layer {} {} {}", l.ordinal, tok(&l.name), tok(&l.kind));
        if let Some(u) = &l.user_name {
            line += &format!(" user {}", tok(u));
        }
        out.push(line);
    }
    for (i, s) in lay.stackup.iter().enumerate() {
        let mut line = format!("stackup {i} {} {}", tok(&s.name), tok(&s.kind));
        if let Some(t) = s.thickness {
            line += &format!(" thickness {}", Nm(t));
        }
        if let Some(m) = &s.material {
            line += &format!(" material {}", tok(m));
        }
        if let Some(e) = s.epsilon_r {
            line += &format!(" er {e}");
        }
        out.push(line);
    }
    for n in &lay.net_names {
        out.push(format!("pcbnet {}", tok(n)));
    }
    for (k, v) in &lay.setup {
        out.push(format!("setup {} {}", tok(k), tok(v)));
    }
    for (i, f) in lay.footprints.iter().enumerate() {
        let k = &keys.fp[i];
        let side = match f.side {
            Side::Front => "front",
            Side::Back => "back",
        };
        let mut line = format!(
            "fp {k} lib {} value {} side {side} at {} rot {}",
            tok(&f.lib_id),
            tok(&f.value),
            f.at,
            f.rotation
        );
        let a = &f.attrs;
        if let Some(m) = &a.mount {
            line += &format!(" mount {}", tok(m));
        }
        for (flag, on) in [
            ("board_only", a.board_only),
            ("exclude_from_pos_files", a.exclude_from_pos_files),
            ("exclude_from_bom", a.exclude_from_bom),
            ("allow_missing_courtyard", a.allow_missing_courtyard),
            ("dnp", a.dnp),
            ("allow_soldermask_bridges", a.allow_soldermask_bridges),
        ] {
            if on {
                line += &format!(" {flag}");
            }
        }
        out.push(line);
        for fl in &f.fields {
            fact_lines(
                out,
                &format!("fp {k} field {}", tok(&fl.name)),
                &fl.value,
                |v| tok(v),
            );
        }
        for g in &f.graphics {
            out.push(format!("fp {k} gfx {}", shape_tokens(g).join(" ")));
        }
        for t in &f.texts {
            out.push(format!("fp {k} text {}", text_tokens(t).join(" ")));
        }
        for g in &f.net_tie_pad_groups {
            let g: Vec<String> = g.iter().map(|x| tok(x)).collect();
            out.push(format!("fp {k} tie {}", g.join(" ")));
        }
        for m in &f.models {
            out.push(format!("fp {k} model {}", tok(m)));
        }
    }
    for (i, p) in lay.pads.iter().enumerate() {
        let k = &keys.pad[i];
        let mut line = format!(
            "pad {k} kind {} shape {} at {} rot {} size {}",
            tok(&p.kind),
            tok(&p.shape),
            p.at,
            p.rotation,
            p.size
        );
        if let Some(d) = p.drill {
            line += &format!(" drill {d}");
        }
        line += &format!(" layers {}", layers(&p.layers));
        if let Some(n) = &p.net_name {
            line += &format!(" net {}", tok(n));
        }
        for (key, v) in [
            ("prop", &p.property),
            ("func", &p.pin_function),
            ("ptype", &p.pin_type),
        ] {
            if let Some(v) = v {
                line += &format!(" {key} {}", tok(v));
            }
        }
        if let Some(m) = p.solder_mask_margin {
            line += &format!(" mask {}", Nm(m));
        }
        if let Some(m) = p.solder_paste_margin {
            line += &format!(" paste {}", Nm(m));
        }
        if let Some(r) = p.solder_paste_ratio {
            line += &format!(" paste-ratio {r}");
        }
        if let Some(z) = p.zone_connect {
            line += &format!(" zone-connect {z}");
        }
        out.push(line);
        for g in &p.primitives {
            out.push(format!("pad {k} prim {}", shape_tokens(g).join(" ")));
        }
    }
    for t in &lay.tracks {
        out.push(format!("track {}", track_tokens(t).join(" ")));
    }
    for v in &lay.vias {
        out.push(format!("via {}", via_tokens(v).join(" ")));
    }
    for z in &lay.zones {
        out.push(format!("zone {}", zone_tokens(z).join(" ")));
    }
    for g in &lay.graphics {
        out.push(format!("gfx {}", shape_tokens(g).join(" ")));
    }
    for t in &lay.texts {
        out.push(format!("text {}", text_tokens(t).join(" ")));
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Reader

/// Why a dump could not be read
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DumpError {
    /// 1-based line number
    pub line: usize,
    pub message: String,
}

impl fmt::Display for DumpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "dump line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for DumpError {}

type R<T> = Result<T, String>;

/// A line's tokens with its 1-based line number
type NumberedLine = (usize, Vec<Tok>);

/// Reads a dump back into the design it was written from
pub fn read(text: &str) -> Result<Design, DumpError> {
    let mut design = Design::default();
    // Lines per board, with their line numbers
    let mut boards: Vec<(String, Vec<NumberedLine>)> = Vec::new();
    let mut links: Vec<(usize, Vec<Tok>)> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() {
            continue;
        }
        let toks = tokenize(line).map_err(|m| DumpError {
            line: n,
            message: m,
        })?;
        match toks.first().map(|t| t.text.as_str()) {
            Some("board") => {
                let name = toks.get(1).map(|t| t.text.clone()).unwrap_or_default();
                boards.push((name, Vec::new()));
            }
            Some("link") => links.push((n, toks)),
            _ => match boards.last_mut() {
                Some(b) => b.1.push((n, toks)),
                None => {
                    return Err(DumpError {
                        line: n,
                        message: "line before the first `board`".into(),
                    });
                }
            },
        }
    }
    for (name, lines) in boards {
        design.boards.push(read_board(name, lines)?);
    }
    for (n, toks) in links {
        let link = read_link(&design, &toks).map_err(|m| DumpError {
            line: n,
            message: m,
        })?;
        design.links.push(link);
    }
    Ok(design)
}

/// A cursor over a line's tokens
struct Cur<'a> {
    t: &'a [Tok],
    i: usize,
}

impl<'a> Cur<'a> {
    fn new(t: &'a [Tok]) -> Cur<'a> {
        Cur { t, i: 0 }
    }

    fn peek(&self) -> Option<&'a Tok> {
        self.t.get(self.i)
    }

    fn next(&mut self) -> R<&'a str> {
        let t = self.t.get(self.i).ok_or("line ends too early")?;
        self.i += 1;
        Ok(&t.text)
    }

    fn word(&mut self, w: &str) -> R<()> {
        let got = self.next()?;
        if got != w {
            return Err(format!("expected `{w}`, got `{got}`"));
        }
        Ok(())
    }

    fn done(&self) -> bool {
        self.i >= self.t.len()
    }

    /// The bare keyword at the cursor, if it is `w`; consumes it
    fn opt(&mut self, w: &str) -> bool {
        match self.peek() {
            Some(t) if !t.quoted && t.text == w => {
                self.i += 1;
                true
            }
            _ => false,
        }
    }

    /// A key and its optional `#n` repeat number
    fn key(&mut self) -> R<(String, usize)> {
        let k = self.next()?.to_string();
        let n = self.repeat();
        Ok((k, n))
    }

    fn repeat(&mut self) -> usize {
        match self.peek() {
            Some(t)
                if !t.quoted
                    && t.text.starts_with('#')
                    && t.text.len() > 1
                    && t.text[1..].chars().all(|c| c.is_ascii_digit()) =>
            {
                self.i += 1;
                t.text[1..].parse().unwrap_or(1)
            }
            _ => 1,
        }
    }

    fn num<T: std::str::FromStr>(&mut self) -> R<T> {
        let s = self.next()?;
        s.parse().map_err(|_| format!("not a number: `{s}`"))
    }

    fn nm(&mut self) -> R<i64> {
        let s = self.next()?;
        Nm::parse(s).ok_or_else(|| format!("not a length: `{s}`"))
    }

    fn pt(&mut self) -> R<Pt> {
        let s = self.next()?;
        parse_pt(s).ok_or_else(|| format!("not a point: `{s}`"))
    }

    fn list(&mut self) -> R<Vec<String>> {
        let s = self.next()?;
        Ok(if s.is_empty() {
            Vec::new()
        } else {
            s.split(',').map(String::from).collect()
        })
    }

    fn rest(&mut self) -> Vec<String> {
        let out = self.t[self.i..].iter().map(|t| t.text.clone()).collect();
        self.i = self.t.len();
        out
    }

    /// `= value @ source [rank n] [at time] [note text]`; `value` reads the value tokens
    fn fact<T>(&mut self, value: impl FnOnce(&mut Cur) -> R<T>) -> R<Fact<T>> {
        self.word("=")?;
        let v = value(self)?;
        self.word("@")?;
        let kind = self.next()?.to_string();
        let mut args = Vec::new();
        while let Some(t) = self.peek() {
            if !t.quoted && matches!(t.text.as_str(), "rank" | "at" | "note") {
                break;
            }
            args.push(t.text.clone());
            self.i += 1;
        }
        let source = Provenance::from_parts(&kind, &args)
            .ok_or_else(|| format!("bad source `{kind}` {args:?}"))?;
        let mut f = Fact::new(v, source);
        if self.opt("rank") {
            f.rank = Rank(self.num()?);
        }
        if self.opt("at") {
            let s = self.next()?;
            f.at = Some(Time::parse(s).ok_or_else(|| format!("bad time `{s}`"))?);
        }
        if self.opt("note") {
            f.note = Some(self.next()?.to_string());
        }
        if !self.done() {
            return Err("unexpected text after the fact".into());
        }
        Ok(f)
    }
}

fn parse_pt(s: &str) -> Option<Pt> {
    let (x, y) = s.split_once(',')?;
    Some(Pt::new(Nm::parse(x)?, Nm::parse(y)?))
}

fn read_value(c: &mut Cur) -> R<Value> {
    match c.next()? {
        "qty" => {
            let v: f64 = c.num()?;
            let u = c.next()?;
            let unit = Unit::from_name(u).ok_or_else(|| format!("bad unit `{u}`"))?;
            Ok(Value::Quantity(Quantity { value: v, unit }))
        }
        "text" => Ok(Value::Text(c.next()?.to_string())),
        o => Err(format!("bad value kind `{o}`")),
    }
}

fn read_bool(c: &mut Cur) -> R<bool> {
    match c.next()? {
        "yes" => Ok(true),
        "no" => Ok(false),
        o => Err(format!("expected yes or no, got `{o}`")),
    }
}

fn read_string(c: &mut Cur) -> R<String> {
    Ok(c.next()?.to_string())
}

fn read_etype(c: &mut Cur) -> R<EcadPinType> {
    let s = c.next()?;
    EcadPinType::from_name(s).ok_or_else(|| format!("bad pin type `{s}`"))
}

/// Builds the board from its lines, entity kinds in dependency order
fn read_board(name: String, lines: Vec<(usize, Vec<Tok>)>) -> Result<Board, DumpError> {
    let mut by_kind: HashMap<String, Vec<(usize, Vec<Tok>)>> = HashMap::new();
    for (n, toks) in lines {
        let kind = toks.first().map(|t| t.text.clone()).unwrap_or_default();
        by_kind.entry(kind).or_default().push((n, toks));
    }
    let mut r = BoardReader {
        b: Board {
            name,
            ..Board::default()
        },
        sheets: HashMap::new(),
        symbols: HashMap::new(),
        variants: HashMap::new(),
        nets: HashMap::new(),
        parts: HashMap::new(),
        pins: HashMap::new(),
        fps: HashMap::new(),
        pads: HashMap::new(),
    };
    let order = [
        "source", "sheet", "symbol", "variant", "netclass", "setting", "net", "part", "pin",
        "label", "nc", "layout", "layer", "stackup", "pcbnet", "setup", "fp", "pad", "track",
        "via", "zone", "gfx", "text",
    ];
    for k in by_kind.keys() {
        if !order.contains(&k.as_str()) {
            let n = by_kind[k][0].0;
            return Err(DumpError {
                line: n,
                message: format!("unknown line kind `{k}`"),
            });
        }
    }
    for kind in order {
        let Some(mut lines) = by_kind.remove(kind) else {
            continue;
        };
        if kind == "sheet" {
            // Parents before children
            lines.sort_by_key(|(_, t)| t.get(1).map_or(0, |p| p.text.matches('/').count()));
        }
        // Entity lines before the lines that add to them, alternates after their pins
        let has = |t: &[Tok], w: &str| t.iter().any(|x| !x.quoted && x.text == w);
        let keyword_at = |t: &[Tok], w: &str| {
            let i = if t
                .get(2)
                .is_some_and(|x| x.text.starts_with('#') && !x.quoted)
            {
                3
            } else {
                2
            };
            t.get(i).is_some_and(|x| x.text == w && !x.quoted)
        };
        let stage = |t: &[Tok]| -> u8 {
            let define = match kind {
                "symbol" => keyword_at(t, "units"),
                "part" | "fp" => keyword_at(t, "lib"),
                "netclass" => t.len() == 2,
                "pin" | "pad" => !has(t, "=") && !has(t, "prim"),
                "net" => !has(t, "="),
                _ => true,
            };
            match (define, kind == "symbol" && has(t, "alt")) {
                (true, _) => 0,
                (false, false) => 1,
                (false, true) => 2,
            }
        };
        lines.sort_by_key(|(_, t)| stage(t));
        for (n, toks) in lines {
            r.line(kind, &toks).map_err(|m| DumpError {
                line: n,
                message: m,
            })?;
        }
    }
    let mut b = r.b;
    b.finish();
    Ok(b)
}

struct BoardReader {
    b: Board,
    sheets: HashMap<String, SheetId>,
    symbols: HashMap<String, SymbolId>,
    variants: HashMap<String, VariantId>,
    nets: HashMap<(String, usize), NetId>,
    parts: HashMap<(String, usize), PartId>,
    pins: HashMap<(PartId, String, usize), PinId>,
    fps: HashMap<(String, usize), FootprintId>,
    pads: HashMap<(FootprintId, String, usize), PadId>,
}

impl BoardReader {
    fn sheet(&self, path: &str) -> R<SheetId> {
        self.sheets
            .get(path)
            .copied()
            .ok_or_else(|| format!("no sheet `{path}`"))
    }

    fn part(&self, c: &mut Cur) -> R<PartId> {
        let k = c.key()?;
        self.parts
            .get(&k)
            .copied()
            .ok_or_else(|| format!("no part `{}`", k.0))
    }

    fn net(&self, c: &mut Cur) -> R<NetId> {
        let k = c.key()?;
        self.nets
            .get(&k)
            .copied()
            .ok_or_else(|| format!("no net `{}`", k.0))
    }

    fn pin(&self, c: &mut Cur) -> R<PinId> {
        let part = self.part(c)?;
        let (num, n) = c.key()?;
        self.pins
            .get(&(part, num.clone(), n))
            .copied()
            .ok_or_else(|| format!("no pin `{num}`"))
    }

    fn fp(&self, c: &mut Cur) -> R<FootprintId> {
        let k = c.key()?;
        self.fps
            .get(&k)
            .copied()
            .ok_or_else(|| format!("no footprint `{}`", k.0))
    }

    fn layout(&mut self) -> R<&mut Layout> {
        self.b
            .layout
            .as_mut()
            .ok_or_else(|| "no `layout` line".into())
    }

    fn line(&mut self, kind: &str, toks: &[Tok]) -> R<()> {
        let mut c = Cur::new(toks);
        c.next()?;
        match kind {
            "source" => match c.next()? {
                "tool" => self.b.source.tool = c.next()?.into(),
                "version" => self.b.source.version = Some(c.next()?.into()),
                "file" => self.b.source.files.push(c.next()?.into()),
                o => return Err(format!("bad source line `{o}`")),
            },
            "sheet" => {
                let path = c.next()?.to_string();
                c.word("name")?;
                let name = c.next()?.to_string();
                c.word("file")?;
                let file = c.next()?.to_string();
                c.word("uuid")?;
                let uuid_path = c.next()?.to_string();
                let page = if c.opt("page") {
                    Some(c.next()?.to_string())
                } else {
                    None
                };
                let parent = if path == "/" {
                    None
                } else {
                    let trimmed = path.trim_end_matches('/');
                    let cut = trimmed.rfind('/').ok_or("bad sheet path")?;
                    Some(self.sheet(&trimmed[..=cut])?)
                };
                let id = SheetId::from_index(self.b.sheets.len());
                self.b.sheets.push(Sheet {
                    name,
                    file,
                    parent,
                    uuid_path,
                    page,
                });
                self.sheets.insert(path, id);
            }
            "symbol" => self.symbol(&mut c)?,
            "variant" => {
                let name = c.next()?.to_string();
                let description = if c.opt("description") {
                    Some(c.next()?.to_string())
                } else {
                    None
                };
                self.variants
                    .insert(name.clone(), VariantId::from_index(self.b.variants.len()));
                self.b.variants.push(Variant { name, description });
            }
            "netclass" => {
                let name = c.next()?.to_string();
                if c.done() {
                    self.b.net_classes.push(NetClass {
                        name,
                        params: Vec::new(),
                        patterns: Vec::new(),
                        assigned: Vec::new(),
                    });
                    return Ok(());
                }
                let class = self
                    .b
                    .net_classes
                    .iter_mut()
                    .find(|n| n.name == name)
                    .ok_or_else(|| format!("no net class `{name}`"))?;
                match c.next()? {
                    "pattern" => class.patterns.push(c.next()?.into()),
                    "assign" => class.assigned.push(c.next()?.into()),
                    "param" => {
                        let key = c.next()?.to_string();
                        let f = c.fact(read_string)?;
                        add_param(&mut class.params, key, f);
                    }
                    o => return Err(format!("bad netclass line `{o}`")),
                }
            }
            "setting" => {
                let group = c.next()?.to_string();
                let key = c.next()?.to_string();
                let f = c.fact(read_string)?;
                let s = &mut self.b.settings;
                let list = match group.as_str() {
                    "textvar" => &mut s.text_vars,
                    "erc-severity" => &mut s.erc_severities,
                    "erc-exclusion" => &mut s.erc_exclusions,
                    "drc-severity" => &mut s.drc_severities,
                    "drc-exclusion" => &mut s.drc_exclusions,
                    "rule" => &mut s.rules,
                    o => return Err(format!("bad setting group `{o}`")),
                };
                add_param(list, key, f);
            }
            "net" => {
                let k = c.key()?;
                if c.opt("kind") {
                    let s = c.next()?;
                    let kind =
                        NetKind::from_name(s).ok_or_else(|| format!("bad net kind `{s}`"))?;
                    self.nets
                        .insert(k.clone(), NetId::from_index(self.b.nets.len()));
                    self.b.nets.push(Net {
                        name: k.0,
                        kind,
                        class: Facts::default(),
                        pins: Vec::new(),
                        labels: Vec::new(),
                    });
                } else {
                    c.word("class")?;
                    let f = c.fact(read_string)?;
                    let id = *self
                        .nets
                        .get(&k)
                        .ok_or_else(|| format!("no net `{}`", k.0))?;
                    self.b.nets[id.index()].class.0.push(f);
                }
            }
            "part" => self.part_line(&mut c)?,
            "pin" => self.pin_line(&mut c)?,
            "label" => {
                let sheet = self.sheet(c.next()?)?;
                let ks = c.next()?;
                let kind =
                    LabelKind::from_name(ks).ok_or_else(|| format!("bad label kind `{ks}`"))?;
                let text = c.next()?.to_string();
                c.word("at")?;
                let at = c.pt()?;
                let mut l = Label {
                    kind,
                    text,
                    sheet,
                    at,
                    shape: None,
                    child: None,
                    fields: Vec::new(),
                    net: None,
                    dangling: false,
                };
                while !c.done() {
                    match c.next()? {
                        "shape" => l.shape = Some(c.next()?.into()),
                        "child" => l.child = Some(self.sheet(c.next()?)?),
                        "net" => l.net = Some(self.net(&mut c)?),
                        "dangling" => l.dangling = true,
                        "field" => {
                            let k = c.next()?.to_string();
                            let v = c.next()?.to_string();
                            l.fields.push((k, v));
                        }
                        o => return Err(format!("bad label attribute `{o}`")),
                    }
                }
                self.b.labels.push(l);
            }
            "nc" => {
                let sheet = self.sheet(c.next()?)?;
                c.word("at")?;
                let at = c.pt()?;
                let pin = if c.opt("pin") {
                    Some(self.pin(&mut c)?)
                } else {
                    None
                };
                self.b.no_connects.push(NoConnect { sheet, at, pin });
            }
            "layout" => {
                c.word("file")?;
                let file = c.next()?.to_string();
                let thickness = if c.opt("thickness") {
                    Some(c.nm()?)
                } else {
                    None
                };
                self.b.layout = Some(Layout {
                    file,
                    thickness,
                    ..Layout::default()
                });
            }
            "layer" => {
                let ordinal = c.num()?;
                let name = c.next()?.to_string();
                let kind = c.next()?.to_string();
                let user_name = if c.opt("user") {
                    Some(c.next()?.to_string())
                } else {
                    None
                };
                self.layout()?.layers.push(LayerDef {
                    ordinal,
                    name,
                    kind,
                    user_name,
                });
            }
            "stackup" => {
                let _index: usize = c.num()?;
                let name = c.next()?.to_string();
                let kind = c.next()?.to_string();
                let mut s = StackupLayer {
                    name,
                    kind,
                    thickness: None,
                    material: None,
                    epsilon_r: None,
                };
                while !c.done() {
                    match c.next()? {
                        "thickness" => s.thickness = Some(c.nm()?),
                        "material" => s.material = Some(c.next()?.into()),
                        "er" => s.epsilon_r = Some(c.num()?),
                        o => return Err(format!("bad stackup attribute `{o}`")),
                    }
                }
                self.layout()?.stackup.push(s);
            }
            "pcbnet" => {
                let n = c.next()?.to_string();
                self.layout()?.net_names.push(n);
            }
            "setup" => {
                let k = c.next()?.to_string();
                let v = c.next()?.to_string();
                self.layout()?.setup.push((k, v));
            }
            "fp" => self.fp_line(&mut c)?,
            "pad" => self.pad_line(&mut c)?,
            "track" => {
                let layer = c.next()?.to_string();
                let start = c.pt()?;
                let mid = if c.opt("mid") { Some(c.pt()?) } else { None };
                let end = c.pt()?;
                c.word("width")?;
                let width = c.nm()?;
                let net_name = if c.opt("net") {
                    Some(c.next()?.to_string())
                } else {
                    None
                };
                self.layout()?.tracks.push(Track {
                    layer,
                    start,
                    mid,
                    end,
                    width,
                    net_name,
                });
            }
            "via" => {
                c.word("at")?;
                let at = c.pt()?;
                c.word("size")?;
                let size = c.nm()?;
                c.word("drill")?;
                let drill = c.nm()?;
                c.word("layers")?;
                let layers = c.list()?;
                c.word("kind")?;
                let kind = c.next()?.to_string();
                let net_name = if c.opt("net") {
                    Some(c.next()?.to_string())
                } else {
                    None
                };
                self.layout()?.vias.push(Via {
                    at,
                    size,
                    drill,
                    layers,
                    kind,
                    net_name,
                });
            }
            "zone" => {
                let mut z = Zone {
                    name: None,
                    net_name: None,
                    layers: Vec::new(),
                    outline: Vec::new(),
                    priority: 0,
                    connect_pads: String::new(),
                    clearance: None,
                    min_thickness: None,
                    thermal_gap: None,
                    thermal_bridge_width: None,
                    keepout: Vec::new(),
                    filled: false,
                };
                while !c.done() {
                    match c.next()? {
                        "name" => z.name = Some(c.next()?.into()),
                        "net" => z.net_name = Some(c.next()?.into()),
                        "layers" => z.layers = c.list()?,
                        "priority" => z.priority = c.num()?,
                        "connect" => z.connect_pads = c.next()?.into(),
                        "clearance" => z.clearance = Some(c.nm()?),
                        "min-thickness" => z.min_thickness = Some(c.nm()?),
                        "thermal-gap" => z.thermal_gap = Some(c.nm()?),
                        "thermal-bridge" => z.thermal_bridge_width = Some(c.nm()?),
                        "filled" => z.filled = true,
                        "keepout" => {
                            let k = c.next()?.to_string();
                            let v = c.next()?.to_string();
                            z.keepout.push((k, v));
                        }
                        "outline" => {
                            while !c.done() {
                                z.outline.push(c.pt()?);
                            }
                        }
                        o => return Err(format!("bad zone attribute `{o}`")),
                    }
                }
                self.layout()?.zones.push(z);
            }
            "gfx" => {
                let s = read_shape(&mut c)?;
                self.layout()?.graphics.push(s);
            }
            "text" => {
                let t = read_text(&mut c)?;
                self.layout()?.texts.push(t);
            }
            o => return Err(format!("unknown line kind `{o}`")),
        }
        if !c.done() {
            return Err(format!("unexpected `{}`", c.rest().join(" ")));
        }
        Ok(())
    }

    fn symbol(&mut self, c: &mut Cur) -> R<()> {
        let name = c.next()?.to_string();
        if c.opt("units") {
            let mut s = SymbolDef {
                name: name.clone(),
                units: c.num()?,
                ..SymbolDef::default()
            };
            if c.opt("power") {
                s.power = Some(match c.next()? {
                    "global" => PowerScope::Global,
                    "local" => PowerScope::Local,
                    o => return Err(format!("bad power scope `{o}`")),
                });
            }
            s.duplicate_pins_are_jumpers = c.opt("jumpers");
            self.symbols
                .insert(name, SymbolId::from_index(self.b.symbols.len()));
            self.b.symbols.push(s);
            return Ok(());
        }
        let id = *self
            .symbols
            .get(&name)
            .ok_or_else(|| format!("no symbol `{name}`"))?;
        let s = &mut self.b.symbols[id.index()];
        match c.next()? {
            "description" => s.description = c.next()?.into(),
            "keywords" => s.keywords = c.next()?.into(),
            "fp-filter" => s.footprint_filters.push(c.next()?.into()),
            "field" => {
                let k = c.next()?.to_string();
                let v = c.next()?.to_string();
                s.fields.push((k, v));
            }
            "hash" => s.hash = Some(c.next()?.into()),
            "jumper-group" => s.jumper_pin_groups.push(c.rest()),
            "pin" => {
                let number = c.next()?.to_string();
                c.word("unit")?;
                let unit: u32 = c.num()?;
                c.word("body")?;
                let body_style: u32 = c.num()?;
                if c.opt("alt") {
                    let name = c.next()?.to_string();
                    c.word("type")?;
                    let etype = read_etype(c)?;
                    c.word("style")?;
                    let style = c.next()?.to_string();
                    let pin = s
                        .pins
                        .iter_mut()
                        .rev()
                        .find(|p| {
                            p.number == number && p.unit == unit && p.body_style == body_style
                        })
                        .ok_or_else(|| format!("no symbol pin `{number}`"))?;
                    pin.alternates.push(PinAlternate { name, etype, style });
                } else {
                    c.word("name")?;
                    let pname = c.next()?.to_string();
                    c.word("type")?;
                    let etype = read_etype(c)?;
                    c.word("style")?;
                    let style = c.next()?.to_string();
                    let hidden = c.opt("hidden");
                    s.pins.push(PinDef {
                        number,
                        name: pname,
                        unit,
                        body_style,
                        etype,
                        style,
                        hidden,
                        alternates: Vec::new(),
                    });
                }
            }
            o => return Err(format!("bad symbol line `{o}`")),
        }
        Ok(())
    }

    fn part_line(&mut self, c: &mut Cur) -> R<()> {
        let key = c.key()?;
        if c.opt("lib") {
            let lib_id = c.next()?.to_string();
            c.word("sheet")?;
            let sheet = self.sheet(c.next()?)?;
            let symbol = if c.opt("symbol") {
                let n = c.next()?;
                Some(
                    *self
                        .symbols
                        .get(n)
                        .ok_or_else(|| format!("no symbol `{n}`"))?,
                )
            } else {
                None
            };
            self.parts
                .insert(key.clone(), PartId::from_index(self.b.parts.len()));
            self.b.parts.push(Part {
                designator: key.0,
                symbol,
                lib_id,
                sheet,
                units: Vec::new(),
                fields: Vec::new(),
                value: Facts::default(),
                flags: PartFlags::default(),
                variants: Vec::new(),
                pins: Vec::new(),
                ties: Vec::new(),
            });
            return Ok(());
        }
        let id = *self
            .parts
            .get(&key)
            .ok_or_else(|| format!("no part `{}`", key.0))?;
        match c.next()? {
            "unit" => {
                let unit: u32 = c.num()?;
                c.word("sheet")?;
                let sheet = self.sheet(c.next()?)?;
                c.word("uuid")?;
                let uuid = c.next()?.to_string();
                let at = if c.opt("at") {
                    let at = c.pt()?;
                    c.word("rot")?;
                    let rotation: f64 = c.num()?;
                    let mirror = if c.opt("mirror") {
                        c.next()?.chars().next()
                    } else {
                        None
                    };
                    Some(Placement {
                        at,
                        rotation,
                        mirror,
                    })
                } else {
                    None
                };
                self.b.parts[id.index()].units.push(UnitPlacement {
                    unit,
                    sheet,
                    uuid,
                    at,
                });
            }
            "field" => {
                let name = c.next()?.to_string();
                let f = c.fact(read_string)?;
                add_field(&mut self.b.parts[id.index()].fields, name, f);
            }
            "value" => {
                let f = c.fact(read_value)?;
                self.b.parts[id.index()].value.0.push(f);
            }
            "flag" => {
                let name = c.next()?.to_string();
                let f = c.fact(read_bool)?;
                self.b.parts[id.index()]
                    .flags
                    .by_name_mut(&name)
                    .ok_or_else(|| format!("bad flag `{name}`"))?
                    .0
                    .push(f);
            }
            "variant" => {
                let vname = c.next()?;
                let vid = *self
                    .variants
                    .get(vname)
                    .ok_or_else(|| format!("no variant `{vname}`"))?;
                let part = &mut self.b.parts[id.index()];
                let pos = match part.variants.iter().position(|v| v.variant == vid) {
                    Some(p) => p,
                    None => {
                        part.variants.push(PartVariant {
                            variant: vid,
                            flags: PartFlags::default(),
                            fields: Vec::new(),
                        });
                        part.variants.len() - 1
                    }
                };
                let pv = &mut part.variants[pos];
                if c.done() {
                    return Ok(());
                }
                match c.next()? {
                    "flag" => {
                        let name = c.next()?.to_string();
                        let f = c.fact(read_bool)?;
                        pv.flags
                            .by_name_mut(&name)
                            .ok_or_else(|| format!("bad flag `{name}`"))?
                            .0
                            .push(f);
                    }
                    "field" => {
                        let name = c.next()?.to_string();
                        let f = c.fact(read_string)?;
                        add_field(&mut pv.fields, name, f);
                    }
                    o => return Err(format!("bad variant line `{o}`")),
                }
            }
            "tie" => {
                let t = c.rest();
                self.b.parts[id.index()].ties.push(t);
            }
            o => return Err(format!("bad part line `{o}`")),
        }
        Ok(())
    }

    fn pin_line(&mut self, c: &mut Cur) -> R<()> {
        let part = self.part(c)?;
        let (number, n) = c.key()?;
        if c.opt("type") {
            let f = c.fact(read_etype)?;
            let id = *self
                .pins
                .get(&(part, number.clone(), n))
                .ok_or_else(|| format!("no pin `{number}`"))?;
            self.b.pins[id.index()].etype.0.push(f);
            return Ok(());
        }
        c.word("name")?;
        let name = c.next()?.to_string();
        c.word("unit")?;
        let unit: u32 = c.num()?;
        let mut pin = PinInst {
            part,
            number: number.clone(),
            name,
            unit,
            etype: Facts::default(),
            hidden: false,
            net: None,
            no_connect: false,
            alt: None,
        };
        while !c.done() {
            match c.next()? {
                "hidden" => pin.hidden = true,
                "nc" => pin.no_connect = true,
                "alt" => pin.alt = Some(c.next()?.into()),
                "net" => pin.net = Some(self.net(c)?),
                o => return Err(format!("bad pin attribute `{o}`")),
            }
        }
        self.pins
            .insert((part, number, n), PinId::from_index(self.b.pins.len()));
        self.b.pins.push(pin);
        Ok(())
    }

    fn fp_line(&mut self, c: &mut Cur) -> R<()> {
        let key = c.key()?;
        if c.opt("lib") {
            let lib_id = c.next()?.to_string();
            c.word("value")?;
            let value = c.next()?.to_string();
            c.word("side")?;
            let side = match c.next()? {
                "front" => Side::Front,
                "back" => Side::Back,
                o => return Err(format!("bad side `{o}`")),
            };
            c.word("at")?;
            let at = c.pt()?;
            c.word("rot")?;
            let rotation: f64 = c.num()?;
            let mut attrs = FootprintAttrs::default();
            while !c.done() {
                match c.next()? {
                    "mount" => attrs.mount = Some(c.next()?.into()),
                    "board_only" => attrs.board_only = true,
                    "exclude_from_pos_files" => attrs.exclude_from_pos_files = true,
                    "exclude_from_bom" => attrs.exclude_from_bom = true,
                    "allow_missing_courtyard" => attrs.allow_missing_courtyard = true,
                    "dnp" => attrs.dnp = true,
                    "allow_soldermask_bridges" => attrs.allow_soldermask_bridges = true,
                    o => return Err(format!("bad footprint attribute `{o}`")),
                }
            }
            let lay = self.layout()?;
            let id = FootprintId::from_index(lay.footprints.len());
            lay.footprints.push(Footprint {
                reference: key.0.clone(),
                part: None,
                lib_id,
                value,
                side,
                at,
                rotation,
                attrs,
                fields: Vec::new(),
                pads: Vec::new(),
                graphics: Vec::new(),
                texts: Vec::new(),
                net_tie_pad_groups: Vec::new(),
                models: Vec::new(),
            });
            self.fps.insert(key, id);
            return Ok(());
        }
        let id = *self
            .fps
            .get(&key)
            .ok_or_else(|| format!("no footprint `{}`", key.0))?;
        let what = c.next()?;
        let fp_field = if what == "field" {
            let name = c.next()?.to_string();
            Some((name, c.fact(read_string)?))
        } else {
            None
        };
        let shape = if what == "gfx" {
            Some(read_shape(c)?)
        } else {
            None
        };
        let text = if what == "text" {
            Some(read_text(c)?)
        } else {
            None
        };
        let rest = if matches!(what, "tie" | "model") {
            c.rest()
        } else {
            Vec::new()
        };
        let f = &mut self.layout()?.footprints[id.index()];
        match what {
            "field" => {
                if let Some((name, fact)) = fp_field {
                    add_field(&mut f.fields, name, fact);
                }
            }
            "gfx" => f.graphics.extend(shape),
            "text" => f.texts.extend(text),
            "tie" => f.net_tie_pad_groups.push(rest),
            "model" => f.models.extend(rest),
            o => return Err(format!("bad footprint line `{o}`")),
        }
        Ok(())
    }

    fn pad_line(&mut self, c: &mut Cur) -> R<()> {
        let fp = self.fp(c)?;
        let (number, n) = c.key()?;
        if c.opt("prim") {
            let s = read_shape(c)?;
            let id = *self
                .pads
                .get(&(fp, number.clone(), n))
                .ok_or_else(|| format!("no pad `{number}`"))?;
            self.layout()?.pads[id.index()].primitives.push(s);
            return Ok(());
        }
        c.word("kind")?;
        let kind = c.next()?.to_string();
        c.word("shape")?;
        let shape = c.next()?.to_string();
        c.word("at")?;
        let at = c.pt()?;
        c.word("rot")?;
        let rotation: f64 = c.num()?;
        c.word("size")?;
        let size = c.pt()?;
        let key = (fp, number.clone(), n);
        let mut p = Pad {
            footprint: fp,
            number,
            kind,
            shape,
            at,
            rotation,
            size,
            drill: None,
            layers: Vec::new(),
            net_name: None,
            net: None,
            property: None,
            pin_function: None,
            pin_type: None,
            solder_mask_margin: None,
            solder_paste_margin: None,
            solder_paste_ratio: None,
            zone_connect: None,
            primitives: Vec::new(),
        };
        while !c.done() {
            match c.next()? {
                "drill" => p.drill = Some(c.pt()?),
                "layers" => p.layers = c.list()?,
                "net" => p.net_name = Some(c.next()?.into()),
                "prop" => p.property = Some(c.next()?.into()),
                "func" => p.pin_function = Some(c.next()?.into()),
                "ptype" => p.pin_type = Some(c.next()?.into()),
                "mask" => p.solder_mask_margin = Some(c.nm()?),
                "paste" => p.solder_paste_margin = Some(c.nm()?),
                "paste-ratio" => p.solder_paste_ratio = Some(c.num()?),
                "zone-connect" => p.zone_connect = Some(c.num()?),
                o => return Err(format!("bad pad attribute `{o}`")),
            }
        }
        let id = PadId::from_index(self.layout()?.pads.len());
        self.pads.insert(key, id);
        self.layout()?.pads.push(p);
        Ok(())
    }
}

fn add_field(fields: &mut Vec<Field>, name: String, f: Fact<String>) {
    match fields.iter_mut().find(|x| x.name == name) {
        Some(x) => x.value.0.push(f),
        None => fields.push(Field {
            name,
            value: Facts::one(f),
        }),
    }
}

fn add_param(params: &mut Vec<Param>, key: String, f: Fact<String>) {
    match params.iter_mut().find(|x| x.key == key) {
        Some(x) => x.value.0.push(f),
        None => params.push(Param {
            key,
            value: Facts::one(f),
        }),
    }
}

fn read_shape(c: &mut Cur) -> R<Shape> {
    let layer = c.next()?.to_string();
    c.word("width")?;
    let width = c.nm()?;
    let filled = c.opt("filled");
    let geom = match c.next()? {
        "line" => Geom::Line(c.pt()?, c.pt()?),
        "rect" => Geom::Rect(c.pt()?, c.pt()?),
        "circle" => Geom::Circle {
            center: c.pt()?,
            radius: c.nm()?,
        },
        "arc" => Geom::Arc {
            start: c.pt()?,
            mid: c.pt()?,
            end: c.pt()?,
        },
        "poly" => {
            let mut pts = Vec::new();
            while !c.done() {
                pts.push(c.pt()?);
            }
            Geom::Poly(pts)
        }
        "bezier" => Geom::Bezier([c.pt()?, c.pt()?, c.pt()?, c.pt()?]),
        o => return Err(format!("bad shape `{o}`")),
    };
    Ok(Shape {
        layer,
        geom,
        width,
        filled,
    })
}

fn read_text(c: &mut Cur) -> R<BoardText> {
    let layer = c.next()?.to_string();
    let text = c.next()?.to_string();
    c.word("at")?;
    let at = c.pt()?;
    c.word("size")?;
    let size = c.nm()?;
    let hidden = c.opt("hidden");
    let mirrored = c.opt("mirrored");
    Ok(BoardText {
        layer,
        text,
        at,
        size,
        hidden,
        mirrored,
    })
}

fn read_link(design: &Design, toks: &[Tok]) -> R<BoardLink> {
    let mut c = Cur::new(toks);
    c.word("link")?;
    let end = |c: &mut Cur| -> R<(BoardId, PartId)> {
        let bname = c.next()?;
        let bi = design
            .boards
            .iter()
            .position(|b| b.name == bname)
            .ok_or_else(|| format!("no board `{bname}`"))?;
        let d = c.next()?;
        let p = design.boards[bi]
            .part_by_designator(d)
            .ok_or_else(|| format!("no part `{d}`"))?;
        Ok((BoardId::from_index(bi), p))
    };
    let a = end(&mut c)?;
    let b = end(&mut c)?;
    let map = match c.next()? {
        "by-number" => LinkMap::ByNumber,
        "table" => {
            let mut t = Vec::new();
            while c.peek().is_some_and(|t| t.quoted || t.text != "@") {
                t.push((c.next()?.to_string(), c.next()?.to_string()));
            }
            LinkMap::Table(t)
        }
        o => return Err(format!("bad link map `{o}`")),
    };
    c.word("@")?;
    let kind = c.next()?.to_string();
    let args = c.rest();
    let source = Provenance::from_parts(&kind, &args).ok_or("bad link source")?;
    Ok(BoardLink { a, b, map, source })
}
