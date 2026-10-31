//! The drawing of one sheet file (`NET-26`): read in two steps, because what a placed symbol shows depends on its
//! unit and reference, known only per sheet instance. [`read`] takes everything from the s-expressions, [`place`]
//! puts the library graphics, pins and fields of each symbol on the sheet.

use super::SymRes;
use super::connect::Transform;
use super::font;
use crate::sexpr::{Lines, Sexp};
use erc_model::geometry::*;
use erc_model::layout::mm_to_nm;
use erc_model::{EcadPinType, LabelKind, Pt};
use std::collections::HashMap;

/// Default font size, as KiCad's
const DEFAULT_TEXT: i64 = 1_270_000;
/// Where a pin's name sits from the body when the library does not say
const DEFAULT_PIN_NAME_OFFSET: i64 = 508_000;
/// Gap between a pin and the name or number drawn along it
const PIN_TEXT_GAP: i64 = 254_000;
/// Gap between a label's wire and its text, over the text height
const LABEL_TEXT_OFFSET: f64 = 0.4;
/// Where a global label's text starts from the connection point, over the text height: flat end, arrow at the wire
const GLOBAL_LEAD_FLAT: f64 = 0.6;
const GLOBAL_LEAD_ARROW: f64 = 1.35;
/// Where a hierarchical label's text starts, over the text height
const HIERARCHICAL_LEAD: f64 = 1.38;
/// How far KiCad lifts a free text with bottom justification: a fixed part and a part of the text height
const TEXT_LIFT: i64 = 250_000;
const TEXT_LIFT_RATIO: f64 = 0.13;
/// Room inside a global label's flag around its text, over the text height
const LABEL_FLAG_MARGIN: f64 = 0.375;

/// The drawing of a sheet file before its symbols are placed
#[derive(Debug, Default)]
pub(crate) struct RawSheet {
    pub sheet: SheetGeometry,
    libs: HashMap<String, LibDrawing>,
    symbols: Vec<RawSymbol>,
}

/// A library symbol's graphics and pins, in library coordinates (y up)
#[derive(Debug, Default, Clone)]
struct LibDrawing {
    extends: Option<String>,
    power: bool,
    units: u32,
    graphics: Vec<(u32, u32, Vec<(i64, i64)>)>,
    pins: Vec<LibPinDrawing>,
    /// `None`: names hidden
    name_offset: Option<i64>,
    numbers_shown: bool,
}

#[derive(Debug, Clone)]
struct LibPinDrawing {
    unit: u32,
    body: u32,
    number: String,
    name: String,
    etype: EcadPinType,
    at: (i64, i64),
    angle: i32,
    length: i64,
    hidden: bool,
    name_size: (i64, i64),
    number_size: (i64, i64),
}

#[derive(Debug)]
struct RawSymbol {
    lib_id: String,
    lib_key: String,
    at: Pt,
    rotation: i32,
    mirror: Option<char>,
    body_style: u32,
    fields: Vec<RawField>,
    uuid: String,
    line: Option<u32>,
}

#[derive(Debug)]
struct RawField {
    name: String,
    value: String,
    at: Pt,
    angle: i32,
    effects: Effects,
    hidden: bool,
    show_name: bool,
}

/// A text's `(effects ...)`
#[derive(Debug, Clone)]
struct Effects {
    size: (i64, i64),
    bold: bool,
    italic: bool,
    halign: HAlign,
    valign: VAlign,
    colour: Option<Colour>,
    hidden: bool,
}

fn point(xy: &Sexp) -> Option<Pt> {
    Some(Pt::from_mm(xy.num(0)?, xy.num(1)?))
}

fn lib_point(xy: &Sexp) -> Option<(i64, i64)> {
    Some((mm_to_nm(xy.num(0)?), mm_to_nm(xy.num(1)?)))
}

fn nm(item: Option<&Sexp>, n: usize) -> Option<i64> {
    item?.num(n).map(mm_to_nm)
}

fn colour(item: Option<&Sexp>) -> Option<Colour> {
    let c = item?;
    let (r, g, b) = (c.num(0)?, c.num(1)?, c.num(2)?);
    let a = c.num(3).unwrap_or(1.0) as f32;
    // KiCad writes `0 0 0 0` for "the theme's colour"
    (a > 0.0).then_some(Colour {
        r: r as u8,
        g: g as u8,
        b: b as u8,
        a,
    })
}

fn stroke(item: &Sexp) -> Stroke {
    let Some(s) = item.child("stroke") else {
        return Stroke::default();
    };
    Stroke {
        width: nm(s.child("width"), 0).unwrap_or(0),
        style: s.value("type").unwrap_or("default").to_string(),
        colour: colour(s.child("color")),
    }
}

fn fill(item: &Sexp) -> Option<Colour> {
    let f = item.child("fill")?;
    match f.value("type") {
        Some("color") => colour(f.child("color")),
        _ => None,
    }
}

fn atom(item: &Sexp, word: &str) -> bool {
    item.args()
        .iter()
        .any(|a| matches!(a, Sexp::Atom(w) if *w == word))
}

fn effects(item: &Sexp) -> Effects {
    let mut out = Effects {
        size: (DEFAULT_TEXT, DEFAULT_TEXT),
        bold: false,
        italic: false,
        halign: HAlign::Center,
        valign: VAlign::Center,
        colour: None,
        hidden: atom(item, "hide") || item.flag("hide").unwrap_or(false),
    };
    let Some(e) = item.child("effects") else {
        return out;
    };
    out.hidden |= atom(e, "hide") || e.flag("hide").unwrap_or(false);
    if let Some(f) = e.child("font") {
        if let Some(s) = f.child("size") {
            out.size = (
                nm(Some(s), 0).unwrap_or(DEFAULT_TEXT),
                nm(Some(s), 1).unwrap_or(DEFAULT_TEXT),
            );
        }
        out.bold = atom(f, "bold") || f.flag("bold").unwrap_or(false);
        out.italic = atom(f, "italic") || f.flag("italic").unwrap_or(false);
        out.colour = colour(f.child("color"));
    }
    if let Some(j) = e.child("justify") {
        for a in j.args() {
            match a.text() {
                Some("left") => out.halign = HAlign::Left,
                Some("right") => out.halign = HAlign::Right,
                Some("top") => out.valign = VAlign::Top,
                Some("bottom") => out.valign = VAlign::Bottom,
                _ => {}
            }
        }
    }
    out
}

/// A text as shown, unescaped and without markup
fn shown(text: &str) -> String {
    font::plain(&super::unescape(text))
}

/// The box of a text on its anchor. A vertical text reads upwards: its start is at the bottom.
fn text_box(
    text: &str,
    at: Pt,
    vertical: bool,
    size: (i64, i64),
    halign: HAlign,
    valign: VAlign,
) -> Rect {
    let lines = text.split('\n').count().max(1) as f64;
    let width = text
        .split('\n')
        .map(|l| font::line_width(l, size.1 as f64))
        .fold(0.0, f64::max) as i64;
    let height = (size.0 as f64 * (1.0 + (lines - 1.0) * font::LINE_PITCH)) as i64;
    // Along the reading direction, then across it towards the top of the letters
    let (along0, along1) = match halign {
        HAlign::Left => (0, width),
        HAlign::Center => (-width / 2, width - width / 2),
        HAlign::Right => (-width, 0),
    };
    let (across0, across1) = match valign {
        VAlign::Bottom => (0, height),
        VAlign::Center => (-height / 2, height - height / 2),
        VAlign::Top => (-height, 0),
    };
    if vertical {
        Rect::of(
            Pt::new(at.x - across1, at.y - along1),
            Pt::new(at.x - across0, at.y - along0),
        )
    } else {
        Rect::of(
            Pt::new(at.x + along0, at.y - across1),
            Pt::new(at.x + along1, at.y - across0),
        )
    }
}

fn make_text(text: String, at: Pt, vertical: bool, e: &Effects) -> Text {
    let bbox = text_box(&text, at, vertical, e.size, e.halign, e.valign);
    Text {
        text,
        at,
        vertical,
        size: e.size,
        bold: e.bold,
        italic: e.italic,
        halign: e.halign,
        valign: e.valign,
        colour: e.colour,
        bbox,
    }
}

fn is_vertical(angle: i32) -> bool {
    angle.rem_euclid(180) == 90
}

/// Paper sizes in millimetres, long side first as KiCad's landscape
fn paper_mm(name: &str) -> Option<(f64, f64)> {
    Some(match name {
        "A5" => (210.0, 148.0),
        "A4" => (297.0, 210.0),
        "A3" => (420.0, 297.0),
        "A2" => (594.0, 420.0),
        "A1" => (841.0, 594.0),
        "A0" => (1189.0, 841.0),
        "A" | "USLetter" => (279.4, 215.9),
        "B" | "USLedger" => (431.8, 279.4),
        "USLegal" => (355.6, 215.9),
        "C" => (558.8, 431.8),
        "D" => (863.6, 558.8),
        "E" => (1117.6, 863.6),
        _ => return None,
    })
}

fn paper(item: &Sexp, sheet: &mut SheetGeometry) {
    let name = item.arg(0).unwrap_or("");
    sheet.paper = name.to_string();
    let (w, h) = if name == "User" {
        (item.num(1).unwrap_or(0.0), item.num(2).unwrap_or(0.0))
    } else {
        paper_mm(name).unwrap_or((0.0, 0.0))
    };
    sheet.paper_size = if atom(item, "portrait") {
        (mm_to_nm(h), mm_to_nm(w))
    } else {
        (mm_to_nm(w), mm_to_nm(h))
    };
}

fn title_block(item: &Sexp) -> TitleBlock {
    let value = |k: &str| item.value(k).unwrap_or("").to_string();
    TitleBlock {
        title: value("title"),
        date: value("date"),
        rev: value("rev"),
        company: value("company"),
        comments: item
            .children("comment")
            .filter_map(|c| Some((c.arg(0)?.parse().ok()?, c.arg(1).unwrap_or("").to_string())))
            .collect(),
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

/// The points that bound a library or sheet graphic
fn graphic_points<'a>(g: &Sexp<'a>) -> Vec<Sexp<'a>> {
    let mut out = Vec::new();
    if let Some(pts) = g.child("pts") {
        out.extend(pts.children("xy").cloned());
    }
    for k in ["start", "mid", "end", "center"] {
        out.extend(g.child(k).cloned());
    }
    out
}

fn lib_pin(p: &Sexp, unit: u32, body: u32) -> LibPinDrawing {
    let at = p.child("at");
    let size_of = |k: &str| {
        p.child(k).map_or((DEFAULT_TEXT, DEFAULT_TEXT), |n| {
            let e = effects(n);
            e.size
        })
    };
    LibPinDrawing {
        unit,
        body,
        number: p.value("number").unwrap_or("").to_string(),
        name: shown(p.value("name").unwrap_or("")),
        etype: EcadPinType::from_name(p.arg(0).unwrap_or("")).unwrap_or(EcadPinType::Unspecified),
        at: (nm(at, 0).unwrap_or(0), nm(at, 1).unwrap_or(0)),
        angle: at.and_then(|a| a.num(2)).unwrap_or(0.0).round() as i32,
        length: nm(p.child("length"), 0).unwrap_or(0),
        hidden: atom(p, "hide") || p.flag("hide").unwrap_or(false),
        name_size: size_of("name"),
        number_size: size_of("number"),
    }
}

fn lib_drawing(s: &Sexp) -> LibDrawing {
    let mut l = LibDrawing {
        extends: s.value("extends").map(String::from),
        power: s.child("power").is_some(),
        units: 1,
        name_offset: Some(DEFAULT_PIN_NAME_OFFSET),
        numbers_shown: true,
        ..LibDrawing::default()
    };
    if let Some(n) = s.child("pin_names") {
        if atom(n, "hide") || n.flag("hide").unwrap_or(false) {
            l.name_offset = None;
        } else if let Some(o) = nm(n.child("offset"), 0) {
            l.name_offset = Some(o);
        }
    }
    if let Some(n) = s.child("pin_numbers") {
        l.numbers_shown = !(atom(n, "hide") || n.flag("hide").unwrap_or(false));
    }
    for sub in s.children("symbol") {
        let (unit, body) = unit_body(sub.arg(0).unwrap_or(""));
        l.units = l.units.max(unit);
        for g in sub.args() {
            match g.head() {
                Some("pin") => l.pins.push(lib_pin(g, unit, body)),
                Some("rectangle" | "polyline" | "arc" | "bezier") => {
                    let pts = graphic_points(g);
                    l.graphics
                        .push((unit, body, pts.iter().filter_map(lib_point).collect()));
                }
                Some("circle") => {
                    if let (Some((x, y)), Some(r)) = (
                        g.child("center").and_then(lib_point),
                        nm(g.child("radius"), 0),
                    ) {
                        l.graphics
                            .push((unit, body, vec![(x - r, y - r), (x + r, y + r)]));
                    }
                }
                _ => {}
            }
        }
    }
    l
}

fn raw_symbol(s: &Sexp, lines: &Lines) -> RawSymbol {
    let at = s.child("at");
    let fields = s
        .children("property")
        .map(|p| {
            let a = p.child("at");
            let e = effects(p);
            RawField {
                name: p.arg(0).unwrap_or("").to_string(),
                value: p.arg(1).unwrap_or("").to_string(),
                at: Pt::new(nm(a, 0).unwrap_or(0), nm(a, 1).unwrap_or(0)),
                angle: a.and_then(|a| a.num(2)).unwrap_or(0.0).round() as i32,
                hidden: e.hidden,
                effects: e,
                show_name: p.flag("show_name").unwrap_or(false),
            }
        })
        .collect();
    let lib_id = s.value("lib_id").unwrap_or("").to_string();
    RawSymbol {
        lib_key: s.value("lib_name").unwrap_or(&lib_id).to_string(),
        lib_id,
        at: Pt::new(nm(at, 0).unwrap_or(0), nm(at, 1).unwrap_or(0)),
        rotation: at.and_then(|a| a.num(2)).unwrap_or(0.0).round() as i32,
        mirror: s.value("mirror").and_then(|m| m.chars().next()),
        body_style: s
            .value("body_style")
            .or(s.value("convert"))
            .and_then(|u| u.parse().ok())
            .unwrap_or(1),
        fields,
        uuid: s.value("uuid").unwrap_or("").to_string(),
        line: lines.line_of(s),
    }
}

fn segment(item: &Sexp, lines: &Lines) -> Option<Segment> {
    let pts: Vec<Pt> = item
        .child("pts")?
        .children("xy")
        .filter_map(point)
        .collect();
    match pts.as_slice() {
        [a, b] => Some(Segment {
            a: *a,
            b: *b,
            stroke: stroke(item),
            uuid: item.value("uuid").unwrap_or("").to_string(),
            line: lines.line_of(item),
        }),
        _ => None,
    }
}

/// The direction a label's text runs: the justification decides, not the angle (KiCad 10)
fn label_runs(angle: i32, halign: HAlign) -> Dir {
    match (is_vertical(angle), halign) {
        (false, HAlign::Right) => Dir::Left,
        (false, _) => Dir::Right,
        (true, HAlign::Right) => Dir::Down,
        (true, _) => Dir::Up,
    }
}

fn label(item: &Sexp, head: &str, lines: &Lines) -> Option<DrawnLabel> {
    let a = item.child("at")?;
    let at = point(a)?;
    let angle = a.num(2).unwrap_or(0.0).round() as i32;
    let kind = match head {
        "label" => LabelKind::Local,
        "global_label" => LabelKind::Global,
        "hierarchical_label" => LabelKind::Hierarchical,
        _ => LabelKind::Directive,
    };
    let e = effects(item);
    let vertical = is_vertical(angle);
    let runs = label_runs(angle, e.halign);
    let h = e.size.0 as f64;
    let (dx, dy) = runs.step();
    // How far along the text starts from the connection point, and how far off the wire
    let (lead, off) = match kind {
        LabelKind::Global => match item.value("shape") {
            Some("output" | "passive") => ((h * GLOBAL_LEAD_FLAT) as i64, 0),
            _ => ((h * GLOBAL_LEAD_ARROW) as i64, 0),
        },
        LabelKind::Hierarchical => ((h * HIERARCHICAL_LEAD) as i64, 0),
        _ => (0, (h * LABEL_TEXT_OFFSET) as i64),
    };
    let start = Pt::new(at.x + dx * lead, at.y + dy * lead);
    let (halign, valign) = match kind {
        LabelKind::Global | LabelKind::Hierarchical => (e.halign, VAlign::Center),
        _ => (e.halign, VAlign::Bottom),
    };
    let start = if vertical {
        Pt::new(start.x - off, start.y)
    } else {
        Pt::new(start.x, start.y - off)
    };
    let body = Effects {
        halign,
        valign,
        ..e.clone()
    };
    let shape = item.value("shape").map(String::from);
    let text = if kind == LabelKind::Directive {
        let shown_fields: Vec<String> = item
            .children("property")
            .filter(|p| !effects(p).hidden)
            .filter_map(|p| p.arg(1).map(shown))
            .collect();
        make_text(shown_fields.join(" "), start, vertical, &body)
    } else {
        make_text(shown(item.arg(0).unwrap_or("")), start, vertical, &body)
    };
    let mut bbox = text.bbox.union(Rect::of(at, at));
    if kind == LabelKind::Global {
        let m = (h * LABEL_FLAG_MARGIN) as i64;
        bbox = bbox.grown(m).union(Rect::of(at, at));
    }
    if kind == LabelKind::Hierarchical {
        let half = (h / 2.0) as i64;
        bbox = bbox.union(Rect::of(at, at).grown(half));
    }
    Some(DrawnLabel {
        kind,
        at,
        shape,
        text,
        bbox,
        runs,
        uuid: item.value("uuid").unwrap_or("").to_string(),
        line: lines.line_of(item),
    })
}

fn free_text(item: &Sexp, lines: &Lines) -> Option<DrawnText> {
    let a = item.child("at")?;
    let e = effects(item);
    let mut at = point(a)?;
    let vertical = is_vertical(a.num(2).unwrap_or(0.0).round() as i32);
    if !vertical && e.valign == VAlign::Bottom {
        at.y -= TEXT_LIFT + (e.size.0 as f64 * TEXT_LIFT_RATIO) as i64;
    }
    let text = make_text(shown(item.arg(0).unwrap_or("")), at, vertical, &e);
    Some(DrawnText {
        text,
        uuid: item.value("uuid").unwrap_or("").to_string(),
        line: lines.line_of(item),
    })
}

/// The text inside a box: on its margins by the justification
fn boxed_text(item: &Sexp, rect: Rect) -> Text {
    let e = effects(item);
    let m = item.child("margins");
    let margin = |n| nm(m, n).unwrap_or(0);
    let (left, top, right, bottom) = (margin(0), margin(1), margin(2), margin(3));
    let x = match e.halign {
        HAlign::Left => rect.min.x + left,
        HAlign::Center => (rect.min.x + rect.max.x) / 2,
        HAlign::Right => rect.max.x - right,
    };
    let y = match e.valign {
        VAlign::Top => rect.min.y + top,
        VAlign::Center => (rect.min.y + rect.max.y) / 2,
        VAlign::Bottom => rect.max.y - bottom,
    };
    make_text(shown(item.arg(0).unwrap_or("")), Pt::new(x, y), false, &e)
}

fn text_box_item(item: &Sexp, lines: &Lines) -> Option<DrawnTextBox> {
    let a = item.child("at")?;
    let at = point(a)?;
    let s = item.child("size")?;
    let rect = Rect::of(at, Pt::new(at.x + nm(Some(s), 0)?, at.y + nm(Some(s), 1)?));
    Some(DrawnTextBox {
        rect,
        stroke: stroke(item),
        fill: fill(item),
        text: boxed_text(item, rect),
        uuid: item.value("uuid").unwrap_or("").to_string(),
        line: lines.line_of(item),
    })
}

fn table(item: &Sexp, lines: &Lines) -> DrawnTable {
    let cells: Vec<DrawnTextBox> = item
        .child("cells")
        .map(|c| {
            c.children("table_cell")
                .filter_map(|cell| text_box_item(cell, lines))
                .collect()
        })
        .unwrap_or_default();
    DrawnTable {
        rect: cells
            .iter()
            .map(|c| c.rect)
            .reduce(Rect::union)
            .unwrap_or_default(),
        cells,
        uuid: item.value("uuid").unwrap_or("").to_string(),
        line: lines.line_of(item),
    }
}

fn graphic(item: &Sexp, head: &str, lines: &Lines) -> Option<DrawnGraphic> {
    let mut points: Vec<Pt> = graphic_points(item).iter().filter_map(point).collect();
    let bbox = if head == "circle" {
        let c = *points.first()?;
        let r = nm(item.child("radius"), 0)?;
        Rect::of(c, c).grown(r)
    } else {
        Rect::around(points.iter().copied())?
    };
    points.shrink_to_fit();
    Some(DrawnGraphic {
        kind: head.to_string(),
        points,
        bbox,
        stroke: stroke(item),
        fill: fill(item),
        uuid: item.value("uuid").unwrap_or("").to_string(),
        line: lines.line_of(item),
    })
}

fn sheet_symbol(item: &Sexp) -> Option<DrawnSheet> {
    let at = point(item.child("at")?)?;
    let s = item.child("size")?;
    let rect = Rect::of(at, Pt::new(at.x + nm(Some(s), 0)?, at.y + nm(Some(s), 1)?));
    let mut sheet = DrawnSheet {
        rect,
        name: String::new(),
        file: String::new(),
        pins: Vec::new(),
        fields: Vec::new(),
        uuid: item.value("uuid").unwrap_or("").to_string(),
    };
    for p in item.children("property") {
        let value = p.arg(1).unwrap_or("").to_string();
        match p.arg(0) {
            Some("Sheetname" | "Sheet name") => sheet.name = value.clone(),
            Some("Sheetfile" | "Sheet file") => sheet.file = value.clone(),
            _ => {}
        }
        if let Some(a) = p.child("at") {
            let e = effects(p);
            sheet.fields.push(DrawnField {
                name: p.arg(0).unwrap_or("").to_string(),
                text: make_text(
                    shown(&value),
                    point(a)?,
                    is_vertical(a.num(2).unwrap_or(0.0).round() as i32),
                    &e,
                ),
                hidden: e.hidden,
            });
        }
    }
    for p in item.children("pin") {
        let a = p.child("at")?;
        let pos = point(a)?;
        let side = if pos.x <= rect.min.x {
            Dir::Left
        } else if pos.x >= rect.max.x {
            Dir::Right
        } else if pos.y <= rect.min.y {
            Dir::Up
        } else {
            Dir::Down
        };
        let (dx, dy) = side.opposite().step();
        let e = effects(p);
        let offset = (e.size.0 as f64 * (1.0 + LABEL_TEXT_OFFSET)) as i64;
        let name = p.arg(0).unwrap_or("").to_string();
        sheet.pins.push(DrawnSheetPin {
            text: make_text(
                shown(&name),
                Pt::new(pos.x + dx * offset, pos.y + dy * offset),
                !side.is_horizontal(),
                &Effects {
                    valign: VAlign::Center,
                    ..e
                },
            ),
            name,
            shape: p.arg(1).unwrap_or("").to_string(),
            at: pos,
            side,
        });
    }
    Some(sheet)
}

/// Reads the drawing of one sheet file
pub(crate) fn read(root: &Sexp, lines: &Lines) -> RawSheet {
    let mut r = RawSheet::default();
    for item in root.args() {
        let Some(head) = item.head() else { continue };
        match head {
            "paper" => paper(item, &mut r.sheet),
            "title_block" => r.sheet.title_block = title_block(item),
            "lib_symbols" => {
                for s in item.children("symbol") {
                    r.libs
                        .insert(s.arg(0).unwrap_or("").to_string(), lib_drawing(s));
                }
            }
            "symbol" => r.symbols.push(raw_symbol(item, lines)),
            "wire" => r.sheet.wires.extend(segment(item, lines)),
            "bus" => r.sheet.buses.extend(segment(item, lines)),
            "junction" => r.sheet.junctions.extend(item.child("at").and_then(point)),
            "no_connect" => r.sheet.no_connects.extend(item.child("at").and_then(point)),
            "label" | "global_label" | "hierarchical_label" | "directive_label"
            | "netclass_flag" => r.sheet.labels.extend(label(item, head, lines)),
            "text" => r.sheet.texts.extend(free_text(item, lines)),
            "text_box" => r.sheet.text_boxes.extend(text_box_item(item, lines)),
            "table" => r.sheet.tables.push(table(item, lines)),
            "polyline" | "rectangle" | "circle" | "arc" | "bezier" => {
                r.sheet.graphics.extend(graphic(item, head, lines))
            }
            "sheet" => r.sheet.sheets.extend(sheet_symbol(item)),
            "group" => r.sheet.groups.push(DrawnGroup {
                name: item.arg(0).unwrap_or("").to_string(),
                members: item
                    .child("members")
                    .map(|m| {
                        m.args()
                            .iter()
                            .filter_map(Sexp::text)
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default(),
                uuid: item.value("uuid").unwrap_or("").to_string(),
            }),
            _ => {}
        }
    }
    r
}

fn dir_of(v: (i64, i64)) -> Dir {
    match v {
        (x, _) if x > 0 => Dir::Right,
        (x, _) if x < 0 => Dir::Left,
        (_, y) if y > 0 => Dir::Down,
        _ => Dir::Up,
    }
}

/// The pin as drawn: its text along it by KiCad's rules (name inside the body with an offset, number above the
/// pin; with offset 0 the name above the pin and the number below it)
fn place_pin(p: &LibPinDrawing, lib: &LibDrawing, sym: &RawSymbol, t: Transform) -> DrawnPin {
    let (dx, dy) = t.apply(p.at);
    let at = Pt::new(sym.at.x + dx, sym.at.y + dy);
    let inward_lib = match p.angle.rem_euclid(360) {
        90 => (0, 1),
        180 => (-1, 0),
        270 => (0, -1),
        _ => (1, 0),
    };
    let outward = dir_of(t.apply(inward_lib)).opposite();
    let mut pin = DrawnPin {
        number: p.number.clone(),
        name: p.name.clone(),
        etype: p.etype,
        at,
        outward,
        length: p.length,
        hidden: p.hidden,
        name_box: None,
        number_box: None,
    };
    if p.hidden {
        return pin;
    }
    let (ox, oy) = outward.step();
    let root = pin.root();
    let middle = Pt::new((root.x + at.x) / 2, (root.y + at.y) / 2);
    let vertical = !outward.is_horizontal();
    let above = VAlign::Bottom;
    let shift = |pos: Pt| {
        if vertical {
            Pt::new(pos.x - PIN_TEXT_GAP, pos.y)
        } else {
            Pt::new(pos.x, pos.y - PIN_TEXT_GAP)
        }
    };
    if let Some(offset) = lib.name_offset.filter(|_| !p.name.is_empty()) {
        pin.name_box = Some(if offset > 0 {
            // Inside the body, running away from the pin
            let start = Pt::new(root.x - ox * offset, root.y - oy * offset);
            let halign = match outward {
                Dir::Left | Dir::Down => HAlign::Left,
                Dir::Right | Dir::Up => HAlign::Right,
            };
            text_box(
                &p.name,
                start,
                vertical,
                p.name_size,
                halign,
                VAlign::Center,
            )
        } else {
            text_box(
                &p.name,
                shift(middle),
                vertical,
                p.name_size,
                HAlign::Center,
                above,
            )
        });
    }
    if lib.numbers_shown && !p.number.is_empty() {
        let name_above = lib.name_offset == Some(0) && !p.name.is_empty();
        pin.number_box = Some(if name_above {
            let pos = if vertical {
                Pt::new(middle.x + PIN_TEXT_GAP, middle.y)
            } else {
                Pt::new(middle.x, middle.y + PIN_TEXT_GAP)
            };
            text_box(
                &p.number,
                pos,
                vertical,
                p.number_size,
                HAlign::Center,
                VAlign::Top,
            )
        } else {
            text_box(
                &p.number,
                shift(middle),
                vertical,
                p.number_size,
                HAlign::Center,
                above,
            )
        });
    }
    pin
}

/// The field as drawn: its position is on the sheet, its angle and justification relative to the symbol, so the
/// justification follows the side the text's own axes end up on, with the text kept readable
fn place_field(f: &RawField, text: String, sym: &RawSymbol, t: Transform) -> DrawnField {
    let vertical = is_vertical(f.angle + sym.rotation);
    let (reading, up) = if is_vertical(f.angle) {
        ((0, 1), (-1, 0))
    } else {
        ((1, 0), (0, 1))
    };
    let reading = dir_of(t.apply(reading));
    let up = dir_of(t.apply(up));
    let halign = match (f.effects.halign, matches!(reading, Dir::Left | Dir::Down)) {
        (HAlign::Left, true) => HAlign::Right,
        (HAlign::Right, true) => HAlign::Left,
        (h, _) => h,
    };
    let valign = match (f.effects.valign, matches!(up, Dir::Right | Dir::Down)) {
        (VAlign::Top, true) => VAlign::Bottom,
        (VAlign::Bottom, true) => VAlign::Top,
        (v, _) => v,
    };
    let e = Effects {
        halign,
        valign,
        ..f.effects.clone()
    };
    DrawnField {
        name: f.name.clone(),
        text: make_text(text, f.at, vertical, &e),
        hidden: f.hidden,
    }
}

/// Places the symbols of a sheet file by what one instance of the sheet resolved
pub(crate) fn place(raw: RawSheet, res: &[SymRes], file: &str) -> SheetGeometry {
    let RawSheet {
        mut sheet,
        libs,
        symbols,
    } = raw;
    sheet.file = file.to_string();
    let none = LibDrawing::default();
    for (i, s) in symbols.iter().enumerate() {
        let lib = libs.get(&s.lib_key).unwrap_or(&none);
        let base = lib
            .extends
            .as_ref()
            .and_then(|e| libs.get(e))
            .filter(|_| lib.graphics.is_empty() && lib.pins.is_empty())
            .unwrap_or(lib);
        let reference = res.get(i).map_or("", |r| r.reference.as_str());
        let unit = res.get(i).map_or(1, |r| r.unit);
        let t = Transform::new(s.rotation, s.mirror);
        let shows = |u: u32, b: u32| (u == 0 || u == unit) && (b == 0 || b == s.body_style);
        let body = Rect::around(
            base.graphics
                .iter()
                .filter(|(u, b, _)| shows(*u, *b))
                .flat_map(|(_, _, pts)| pts)
                .map(|p| {
                    let (x, y) = t.apply(*p);
                    Pt::new(s.at.x + x, s.at.y + y)
                }),
        );
        let pins = base
            .pins
            .iter()
            .filter(|p| shows(p.unit, p.body))
            .map(|p| place_pin(p, base, s, t))
            .collect();
        let value = s
            .fields
            .iter()
            .find(|f| f.name == "Value")
            .map_or("", |f| f.value.as_str());
        let fields = s
            .fields
            .iter()
            .filter(|f| !f.name.starts_with("ki_"))
            .map(|f| {
                let mut text = if f.name == "Reference" && !reference.is_empty() {
                    reference.to_string()
                } else {
                    shown(&f.value)
                };
                if f.name == "Reference" && lib.units.max(base.units) > 1 {
                    text.push_str(&super::connect::unit_letter(unit));
                }
                if f.show_name {
                    text = format!("{}: {text}", f.name);
                }
                place_field(f, text, s, t)
            })
            .collect();
        sheet.symbols.push(DrawnSymbol {
            reference: reference.to_string(),
            lib_id: s.lib_id.clone(),
            unit,
            power: lib.power || base.power,
            value: shown(value),
            at: s.at,
            rotation: s.rotation,
            mirror: s.mirror,
            body,
            pins,
            fields,
            uuid: s.uuid.clone(),
            line: s.line,
        });
    }
    sheet
}
