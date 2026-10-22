//! The KiCad board (`.kicad_pcb`, KiCad 8, 9 and 10) into a [`Layout`]: the layer table and stackup, the board
//! setup, footprints with their fields, attributes, pads, graphics (courtyard, silk, fab) and texts in absolute
//! coordinates, tracks, vias, zones and rule areas, board graphics (the outline on `Edge.Cuts`) and texts.
//! KiCad 8 and 9 give nets by number through the board's net table, KiCad 10 by name; both end up as names.
//! Footprints link to the schematic's parts by designator and pads to its nets by name when the board is read
//! with its schematic ([`erc_model::link_layout`]).

use crate::notes::Notes;
use crate::sexpr::{self, Sexp};
use erc_model::layout::*;
use erc_model::*;
use std::collections::HashMap;

/// A length in millimetres as nanometres
fn nm(s: &Sexp, n: usize) -> Option<i64> {
    let t = s.arg(n)?;
    Nm::parse(t).or_else(|| t.parse::<f64>().ok().map(mm_to_nm))
}

fn xy(s: Option<&Sexp>) -> Option<Pt> {
    let s = s?;
    Some(Pt::new(nm(s, 0)?, nm(s, 1)?))
}

/// A `(width w)` or `(stroke (width w))`
fn width(s: &Sexp) -> i64 {
    s.child("stroke")
        .and_then(|st| st.child("width"))
        .or_else(|| s.child("width"))
        .and_then(|w| nm(w, 0))
        .unwrap_or(0)
}

/// `(fill yes)`, `(fill solid)`, `(fill (type outline))`...
fn filled(s: &Sexp) -> bool {
    match s.child("fill") {
        Some(f) => match f.arg(0) {
            Some("yes" | "solid") => true,
            Some(_) => false,
            None => matches!(f.value("type"), Some("outline" | "background" | "color")),
        },
        None => false,
    }
}

/// `(hide yes)`, a bare `hide`, or `(effects ... (hide yes))` / `(effects ... hide)`
fn hidden(s: &Sexp) -> bool {
    let here = |x: &Sexp| {
        x.flag("hide") == Some(true) || x.args().iter().any(|a| a.text() == Some("hide"))
    };
    here(s) || s.child("effects").is_some_and(here)
}

/// The points of a `(pts (xy ..) (arc (start ..) (mid ..) (end ..)) ...)`, an arc as its three points
fn pts(s: &Sexp) -> Vec<Pt> {
    let mut out = Vec::new();
    if let Some(p) = s.child("pts") {
        for x in p.args() {
            match x.head() {
                Some("xy") => out.extend(xy(Some(x))),
                Some("arc") => {
                    for k in ["start", "mid", "end"] {
                        out.extend(xy(x.child(k)));
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// A placement: footprint position and rotation; children's local coordinates map through it
#[derive(Clone, Copy)]
struct Frame {
    at: Pt,
    /// Degrees, counter-clockwise on screen (y down)
    rot: f64,
}

impl Frame {
    const BOARD: Frame = Frame {
        at: Pt::new(0, 0),
        rot: 0.0,
    };

    fn map(&self, p: Pt) -> Pt {
        if self.rot == 0.0 {
            return Pt::new(self.at.x + p.x, self.at.y + p.y);
        }
        let (s, c) = self.rot.to_radians().sin_cos();
        let (x, y) = (p.x as f64, p.y as f64);
        Pt::new(
            self.at.x + (x * c + y * s).round() as i64,
            self.at.y + (-x * s + y * c).round() as i64,
        )
    }

    /// Whether a rectangle stays axis-parallel
    fn square(&self) -> bool {
        (self.rot / 90.0).fract() == 0.0
    }
}

/// A graphic item (`fp_line`, `gr_rect`, ...) as a shape in absolute coordinates; `None` when it is not one
fn shape(s: &Sexp, frame: Frame) -> Option<Shape> {
    let head = s.head()?;
    let kind = head
        .strip_prefix("fp_")
        .or_else(|| head.strip_prefix("gr_"))?;
    let geom = match kind {
        "line" => Geom::Line(
            frame.map(xy(s.child("start"))?),
            frame.map(xy(s.child("end"))?),
        ),
        "rect" => {
            let (a, b) = (xy(s.child("start"))?, xy(s.child("end"))?);
            if frame.square() {
                let (a, b) = (frame.map(a), frame.map(b));
                Geom::Rect(
                    Pt::new(a.x.min(b.x), a.y.min(b.y)),
                    Pt::new(a.x.max(b.x), a.y.max(b.y)),
                )
            } else {
                Geom::Poly(
                    [a, Pt::new(b.x, a.y), b, Pt::new(a.x, b.y)]
                        .into_iter()
                        .map(|p| frame.map(p))
                        .collect(),
                )
            }
        }
        "circle" => {
            let (c, e) = (xy(s.child("center"))?, xy(s.child("end"))?);
            let r = ((e.x - c.x) as f64).hypot((e.y - c.y) as f64).round() as i64;
            Geom::Circle {
                center: frame.map(c),
                radius: r,
            }
        }
        "arc" => Geom::Arc {
            start: frame.map(xy(s.child("start"))?),
            mid: frame.map(xy(s.child("mid"))?),
            end: frame.map(xy(s.child("end"))?),
        },
        "poly" => Geom::Poly(pts(s).into_iter().map(|p| frame.map(p)).collect()),
        "curve" | "bezier" => {
            let p: Vec<Pt> = pts(s).into_iter().map(|p| frame.map(p)).collect();
            Geom::Bezier(p.try_into().ok()?)
        }
        _ => return None,
    };
    Some(Shape {
        layer: s.value("layer").unwrap_or("").to_string(),
        geom,
        width: width(s),
        filled: filled(s),
    })
}

/// A text (`fp_text`, `gr_text`, a footprint property) in absolute coordinates
fn text(s: &Sexp, text: &str, frame: Frame) -> BoardText {
    let font = s.child("effects").and_then(|e| e.child("font"));
    BoardText {
        layer: s.value("layer").unwrap_or("").to_string(),
        text: text.to_string(),
        at: frame.map(xy(s.child("at")).unwrap_or_default()),
        size: font
            .and_then(|f| f.child("size"))
            .and_then(|z| nm(z, 0))
            .unwrap_or(0),
        hidden: hidden(s),
        mirrored: s
            .child("effects")
            .and_then(|e| e.child("justify"))
            .is_some_and(|j| j.args().iter().any(|a| a.text() == Some("mirror"))),
    }
}

struct Reader<'a> {
    file: &'a str,
    notes: Notes,
    /// The board's net table (KiCad 8 and 9): net code to name
    codes: HashMap<String, String>,
    lay: Layout,
}

impl Reader<'_> {
    fn src(&self) -> Provenance {
        Provenance::Board {
            file: self.file.to_string(),
        }
    }

    /// `(net 3 "GND")`, `(net 3)` with the table, `(net "GND")` (KiCad 10), or a zone's `(net_name "GND")`; net 0
    /// and "" are no net
    fn net(&self, s: &Sexp) -> Option<String> {
        let name = match s.value("net_name") {
            Some(n) => n.to_string(),
            None => {
                let n = s.child("net")?;
                match (n.args(), n.arg(1)) {
                    (_, Some(name)) => name.to_string(),
                    ([Sexp::Atom(code)], None) => self.codes.get(*code)?.clone(),
                    ([Sexp::Str(name)], None) => name.to_string(),
                    _ => return None,
                }
            }
        };
        Some(name).filter(|n| !n.is_empty())
    }

    fn layers(&mut self, s: &Sexp) {
        for l in s.args() {
            let (Some(ord), Some(name)) = (l.head(), l.arg(0)) else {
                self.notes.unknown("layers", l);
                continue;
            };
            let Ok(ordinal) = ord.parse::<i32>() else {
                self.notes
                    .invalid("layers", &format!("layer number `{ord}`"));
                continue;
            };
            self.lay.layers.push(LayerDef {
                ordinal,
                name: name.to_string(),
                kind: l.arg(1).unwrap_or("").to_string(),
                user_name: l.arg(2).map(String::from),
            });
        }
    }

    fn setup(&mut self, s: &Sexp) {
        for item in s.args() {
            let Some(head) = item.head() else {
                self.notes.unknown("setup", item);
                continue;
            };
            match head {
                "stackup" => {
                    for l in item.children("layer") {
                        self.lay.stackup.push(StackupLayer {
                            name: l.arg(0).unwrap_or("").to_string(),
                            kind: l.value("type").unwrap_or("").to_string(),
                            thickness: l.child("thickness").and_then(|t| nm(t, 0)),
                            material: l.value("material").map(String::from),
                            epsilon_r: l.child("epsilon_r").and_then(|e| e.num(0)),
                        });
                    }
                    for x in item.args().iter().filter(|x| x.head() != Some("layer")) {
                        if let (Some(h), Some(v)) = (x.head(), x.arg(0)) {
                            self.lay.setup.push((format!("stackup.{h}"), v.to_string()));
                        }
                    }
                }
                // Plot settings are an editor's state, not the board
                "pcbplotparams" => {}
                // A value (`(pad_to_mask_clearance 0.05)`), or one level of named values (`(tenting (front yes)
                // (back yes))`)
                _ => {
                    let vals: Vec<&str> = item.args().iter().filter_map(Sexp::text).collect();
                    if vals.len() == item.args().len() {
                        self.lay.setup.push((head.to_string(), vals.join(" ")));
                        continue;
                    }
                    for x in item.args() {
                        let vals: Vec<&str> = x.args().iter().filter_map(Sexp::text).collect();
                        match x.head() {
                            Some(h) if vals.len() == x.args().len() => {
                                self.lay.setup.push((format!("{head}.{h}"), vals.join(" ")))
                            }
                            _ => self
                                .notes
                                .ignored("setup", &format!("({head} ...) not kept")),
                        }
                    }
                }
            }
        }
    }

    fn footprint(&mut self, s: &Sexp) {
        let lib_id = s.arg(0).unwrap_or("").to_string();
        let at = s.child("at");
        let frame = Frame {
            at: xy(at).unwrap_or_default(),
            rot: at.and_then(|a| a.num(2)).unwrap_or(0.0),
        };
        let side = match s.value("layer") {
            Some("B.Cu") => Side::Back,
            _ => Side::Front,
        };
        let id = FootprintId::from_index(self.lay.footprints.len());
        let mut fp = Footprint {
            reference: String::new(),
            part: None,
            lib_id,
            value: String::new(),
            side,
            at: frame.at,
            rotation: frame.rot,
            attrs: FootprintAttrs::default(),
            fields: Vec::new(),
            pads: Vec::new(),
            graphics: Vec::new(),
            texts: Vec::new(),
            net_tie_pad_groups: Vec::new(),
            models: Vec::new(),
        };
        let mut ctx = format!("footprint {}", fp.lib_id);
        for item in s.args().iter().skip(1) {
            let Some(head) = item.head() else {
                // KiCad 7's bare `locked` and `placed`
                if !matches!(item.text(), Some("locked" | "placed")) {
                    self.notes.unknown(&ctx, item);
                }
                continue;
            };
            match head {
                "property" => {
                    let (Some(k), v) = (item.arg(0), item.arg(1).unwrap_or("")) else {
                        continue;
                    };
                    match k {
                        "Reference" => {
                            fp.reference = v.to_string();
                            ctx = format!("footprint {v}");
                        }
                        "Value" => fp.value = v.to_string(),
                        _ if v.is_empty() => {}
                        _ => fp
                            .fields
                            .push(Field::new(k, Fact::new(v.to_string(), self.src()))),
                    }
                    if item.child("layer").is_some() && !v.is_empty() {
                        fp.texts.push(text(item, v, frame));
                    }
                }
                "fp_text" => {
                    let v = item.arg(1).unwrap_or("");
                    match item.arg(0) {
                        Some("reference") => fp.reference = v.to_string(),
                        Some("value") => fp.value = v.to_string(),
                        _ => {}
                    }
                    fp.texts.push(text(item, v, frame));
                }
                "fp_text_box" => {
                    let v = item.arg(0).unwrap_or("");
                    fp.texts.push(text(item, v, frame));
                }
                "attr" => {
                    for a in item.args().iter().filter_map(Sexp::text) {
                        match a {
                            "smd" | "through_hole" => fp.attrs.mount = Some(a.to_string()),
                            "board_only" => fp.attrs.board_only = true,
                            "exclude_from_pos_files" => fp.attrs.exclude_from_pos_files = true,
                            "exclude_from_bom" => fp.attrs.exclude_from_bom = true,
                            "allow_missing_courtyard" => fp.attrs.allow_missing_courtyard = true,
                            "dnp" => fp.attrs.dnp = true,
                            "allow_soldermask_bridges" => fp.attrs.allow_soldermask_bridges = true,
                            other => self.notes.add(
                                &ctx,
                                NoteKind::Unknown,
                                format!("attribute `{other}` not read"),
                            ),
                        }
                    }
                }
                "net_tie_pad_groups" => {
                    for g in item.args().iter().filter_map(Sexp::text) {
                        fp.net_tie_pad_groups
                            .push(g.split(',').map(|n| n.trim().to_string()).collect());
                    }
                }
                "model" => fp.models.extend(item.arg(0).map(String::from)),
                "pad" => self.pad(item, id, frame, &ctx),
                h if h.starts_with("fp_") => match shape(item, frame) {
                    Some(sh) => fp.graphics.push(sh),
                    None => self.notes.unknown(&ctx, item),
                },
                // Identity, library bookkeeping, editor state and defaults that carry no board fact
                "layer"
                | "uuid"
                | "tstamp"
                | "at"
                | "path"
                | "sheetname"
                | "sheetfile"
                | "descr"
                | "tags"
                | "locked"
                | "placed"
                | "units"
                | "duplicate_pad_numbers_are_jumpers"
                | "jumper_pad_groups"
                | "embedded_fonts"
                | "embedded_files"
                | "zone_connect"
                | "clearance"
                | "solder_mask_margin"
                | "solder_paste_margin"
                | "solder_paste_margin_ratio"
                | "solder_paste_ratio"
                | "thermal_width"
                | "thermal_gap"
                | "private_layers"
                | "component_classes"
                | "autoplace_cost90"
                | "autoplace_cost180"
                | "zone"
                | "group"
                | "dimension"
                | "point"
                | "barcode"
                | "table"
                | "image"
                | "stackup"
                | "variant"
                | "variants"
                | "unit" => {}
                _ => self.notes.unknown(&ctx, item),
            }
        }
        self.lay.footprints.push(fp);
    }

    fn pad(&mut self, s: &Sexp, footprint: FootprintId, frame: Frame, ctx: &str) {
        let at = s.child("at");
        let drill = s.child("drill").and_then(|d| {
            let nums: Vec<i64> = d
                .args()
                .iter()
                .enumerate()
                .filter(|(_, a)| a.head().is_none() && a.text() != Some("oval"))
                .filter_map(|(i, _)| nm(d, i))
                .collect();
            match nums.as_slice() {
                [d] => Some(Pt::new(*d, *d)),
                [w, h, ..] => Some(Pt::new(*w, *h)),
                _ => None,
            }
        });
        let ctx = format!("{ctx} pad {}", s.arg(0).unwrap_or(""));
        let mut primitives = Vec::new();
        if let Some(prims) = s.child("primitives") {
            for p in prims.args() {
                match shape(p, Frame::BOARD) {
                    Some(sh) => primitives.push(sh),
                    None => self.notes.unknown(&ctx, p),
                }
            }
        }
        let layers: Vec<String> = s
            .child("layers")
            .map(|l| {
                l.args()
                    .iter()
                    .filter_map(Sexp::text)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let pad = Pad {
            footprint,
            number: s.arg(0).unwrap_or("").to_string(),
            kind: s.arg(1).unwrap_or("").to_string(),
            shape: s.arg(2).unwrap_or("").to_string(),
            at: frame.map(xy(at).unwrap_or_default()),
            rotation: at.and_then(|a| a.num(2)).unwrap_or(0.0),
            size: xy(s.child("size")).unwrap_or_default(),
            drill,
            layers,
            net_name: self.net(s),
            net: None,
            property: s.value("property").map(String::from),
            pin_function: s.value("pinfunction").map(String::from),
            pin_type: s.value("pintype").map(String::from),
            solder_mask_margin: s.child("solder_mask_margin").and_then(|m| nm(m, 0)),
            solder_paste_margin: s.child("solder_paste_margin").and_then(|m| nm(m, 0)),
            solder_paste_ratio: s
                .child("solder_paste_margin_ratio")
                .or_else(|| s.child("solder_paste_ratio"))
                .and_then(|m| m.num(0)),
            zone_connect: s.child("zone_connect").and_then(|z| z.arg(0)?.parse().ok()),
            primitives,
        };
        for item in s.args().iter().skip(3) {
            let known = match item.head() {
                Some(h) => matches!(
                    h,
                    "at" | "size"
                        | "drill"
                        | "layers"
                        | "net"
                        | "property"
                        | "pinfunction"
                        | "pintype"
                        | "solder_mask_margin"
                        | "solder_paste_margin"
                        | "solder_paste_margin_ratio"
                        | "solder_paste_ratio"
                        | "zone_connect"
                        | "primitives"
                        | "uuid"
                        | "tstamp"
                        | "roundrect_rratio"
                        | "chamfer_ratio"
                        | "chamfer"
                        | "rect_delta"
                        | "die_length"
                        | "die_delay"
                        | "clearance"
                        | "thermal_width"
                        | "thermal_bridge_width"
                        | "thermal_gap"
                        | "thermal_bridge_angle"
                        | "remove_unused_layers"
                        | "keep_end_layers"
                        | "zone_layer_connections"
                        | "options"
                        | "teardrops"
                        | "tenting"
                        | "padstack"
                        | "locked"
                        | "backdrill"
                        | "front_post_machining"
                        | "back_post_machining"
                        | "covering"
                        | "plugging"
                        | "capping"
                        | "filling"
                ),
                None => matches!(item.text(), Some("locked")),
            };
            if !known {
                self.notes.unknown(&ctx, item);
            }
        }
        self.lay.pads.push(pad);
    }

    fn zone(&mut self, s: &Sexp) {
        let mut layers: Vec<String> = Vec::new();
        if let Some(l) = s.value("layer") {
            layers.push(l.to_string());
        }
        if let Some(ls) = s.child("layers") {
            layers.extend(ls.args().iter().filter_map(Sexp::text).map(String::from));
        }
        let connect = s.child("connect_pads");
        let connect_pads = match connect.and_then(|c| c.arg(0)) {
            Some("yes") => "solid",
            Some("no") => "none",
            Some("thru_hole_only") => "thru_hole_only",
            _ => "thermal",
        }
        .to_string();
        let fill = s.child("fill");
        let keepout = s
            .child("keepout")
            .map(|k| {
                k.args()
                    .iter()
                    .filter_map(|x| Some((x.head()?.to_string(), x.arg(0)?.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        let zone = Zone {
            name: s.value("name").map(String::from),
            net_name: self.net(s),
            layers,
            outline: s.child("polygon").map(pts).unwrap_or_default(),
            priority: s
                .child("priority")
                .and_then(|p| p.arg(0)?.parse().ok())
                .unwrap_or(0),
            connect_pads,
            clearance: connect
                .and_then(|c| c.child("clearance"))
                .and_then(|c| nm(c, 0)),
            min_thickness: s.child("min_thickness").and_then(|m| nm(m, 0)),
            thermal_gap: fill
                .and_then(|f| f.child("thermal_gap"))
                .and_then(|t| nm(t, 0)),
            thermal_bridge_width: fill
                .and_then(|f| f.child("thermal_bridge_width"))
                .and_then(|t| nm(t, 0)),
            keepout,
            filled: fill.is_some_and(|f| f.arg(0) == Some("yes"))
                || s.child("filled_polygon").is_some(),
        };
        self.lay.zones.push(zone);
    }

    fn top(&mut self, root: &Sexp) {
        // The net table first: tracks and pads may come before it in hand-edited files
        for n in root.children("net") {
            if let (Some(code), Some(name)) = (n.arg(0), n.arg(1)) {
                self.codes.insert(code.to_string(), name.to_string());
                if !name.is_empty() {
                    self.lay.net_names.push(name.to_string());
                }
            }
        }
        for item in root.args() {
            let Some(head) = item.head() else {
                self.notes.unknown("kicad_pcb", item);
                continue;
            };
            match head {
                "general" => {
                    self.lay.thickness = item.child("thickness").and_then(|t| nm(t, 0));
                }
                "layers" => self.layers(item),
                "setup" => self.setup(item),
                "property" => {
                    if let (Some(k), Some(v)) = (item.arg(0), item.arg(1)) {
                        self.lay
                            .setup
                            .push((format!("property.{k}"), v.to_string()));
                    }
                }
                "footprint" => self.footprint(item),
                "segment" | "arc" => {
                    let (Some(start), Some(end)) = (xy(item.child("start")), xy(item.child("end")))
                    else {
                        self.notes
                            .invalid("kicad_pcb", &format!("({head} ...) without its ends"));
                        continue;
                    };
                    self.lay.tracks.push(Track {
                        layer: item.value("layer").unwrap_or("").to_string(),
                        start,
                        mid: xy(item.child("mid")),
                        end,
                        width: item.child("width").and_then(|w| nm(w, 0)).unwrap_or(0),
                        net_name: self.net(item),
                    });
                }
                "via" => {
                    let kind = item
                        .args()
                        .iter()
                        .filter_map(Sexp::text)
                        .find(|t| matches!(*t, "blind" | "micro" | "buried"))
                        .unwrap_or("through")
                        .to_string();
                    self.lay.vias.push(Via {
                        at: xy(item.child("at")).unwrap_or_default(),
                        size: item.child("size").and_then(|w| nm(w, 0)).unwrap_or(0),
                        drill: item.child("drill").and_then(|w| nm(w, 0)).unwrap_or(0),
                        layers: item
                            .child("layers")
                            .map(|l| {
                                l.args()
                                    .iter()
                                    .filter_map(Sexp::text)
                                    .map(String::from)
                                    .collect()
                            })
                            .unwrap_or_default(),
                        kind,
                        net_name: self.net(item),
                    });
                }
                "zone" => self.zone(item),
                "gr_text" => {
                    let v = item.arg(0).unwrap_or("");
                    self.lay.texts.push(text(item, v, Frame::BOARD));
                }
                "gr_text_box" => {
                    let v = item.arg(0).unwrap_or("");
                    self.lay.texts.push(text(item, v, Frame::BOARD));
                }
                h if h.starts_with("gr_") => match shape(item, Frame::BOARD) {
                    Some(sh) => self.lay.graphics.push(sh),
                    None => self.notes.unknown("kicad_pcb", item),
                },
                // The net table is read above; the rest is bookkeeping, documentation or editor state
                "net" | "version" | "generator" | "generator_version" | "paper" | "title_block"
                | "embedded_fonts" | "embedded_files" | "group" | "generated" | "image"
                | "target" | "dimension" | "table" | "net_class" | "page" | "point" | "barcode"
                | "component_classes" | "variants" => {}
                _ => self.notes.unknown("kicad_pcb", item),
            }
        }
    }
}

/// Reads a board file; `file` names it in the layout and the notes
pub fn read(text: &str, file: &str) -> Result<(Layout, Vec<ImportNote>), ImportError> {
    let parsed = sexpr::parse(text).map_err(|e| ImportError::Invalid {
        file: file.to_string(),
        message: e.to_string(),
    })?;
    let root = match parsed.as_slice() {
        [r] if r.head() == Some("kicad_pcb") => r,
        _ => {
            return Err(ImportError::Invalid {
                file: file.to_string(),
                message: "not a KiCad board: no `(kicad_pcb ...)`".into(),
            });
        }
    };
    let mut r = Reader {
        file,
        notes: Notes::new(file),
        codes: HashMap::new(),
        lay: Layout {
            file: file.to_string(),
            ..Layout::default()
        },
    };
    r.top(root);
    // KiCad 10 has no net table: the nets are those its items name
    if r.lay.net_names.is_empty() {
        let lay = &r.lay;
        let mut names: Vec<String> = lay
            .pads
            .iter()
            .filter_map(|p| p.net_name.clone())
            .chain(lay.tracks.iter().filter_map(|t| t.net_name.clone()))
            .chain(lay.vias.iter().filter_map(|v| v.net_name.clone()))
            .chain(lay.zones.iter().filter_map(|z| z.net_name.clone()))
            .collect();
        names.sort();
        names.dedup();
        r.lay.net_names = names;
    }
    Ok((r.lay, r.notes.into_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_rotates_counter_clockwise() {
        let f = Frame {
            at: Pt::from_mm(10.0, 10.0),
            rot: 90.0,
        };
        assert_eq!(f.map(Pt::from_mm(1.0, 0.0)), Pt::from_mm(10.0, 9.0));
        assert_eq!(f.map(Pt::from_mm(0.0, 1.0)), Pt::from_mm(11.0, 10.0));
    }

    #[test]
    fn nets_by_code_and_by_name() {
        let text = r#"(kicad_pcb (version 20240108) (net 0 "") (net 1 "GND")
            (footprint "R" (layer "B.Cu") (at 10 20 90)
              (property "Reference" "R1" (at 0 -1.5 90) (layer "B.SilkS") (effects (font (size 1 1)) (justify mirror)))
              (property "Mpn1" "RC0603" (at 0 0 0) (layer "B.Fab") (hide yes))
              (attr smd)
              (fp_rect (start -1 -0.5) (end 1 0.5) (stroke (width 0.05)) (layer "B.CrtYd"))
              (pad "1" smd roundrect (at -0.8 0 90) (size 0.8 0.9) (layers "B.Cu" "B.Mask") (net 1 "GND"))
              (pad "2" smd roundrect (at 0.8 0 90) (size 0.8 0.9) (layers "B.Cu" "B.Mask") (net "VCC")))
            (segment (start 0 0) (end 1 0) (width 0.2) (layer "F.Cu") (net 1))
            (via (at 1 0) (size 0.6) (drill 0.3) (layers "F.Cu" "B.Cu") (net 0)))"#;
        let (lay, notes) = read(text, "b.kicad_pcb").unwrap();
        assert!(notes.is_empty(), "{notes:?}");
        let fp = &lay.footprints[0];
        assert_eq!((fp.reference.as_str(), fp.side), ("R1", Side::Back));
        assert_eq!(fp.texts[0].at, Pt::from_mm(8.5, 20.0));
        assert!(fp.texts[0].mirrored && fp.texts[1].hidden);
        assert_eq!(
            fp.graphics[0].geom,
            Geom::Rect(Pt::from_mm(9.5, 19.0), Pt::from_mm(10.5, 21.0))
        );
        assert_eq!(lay.pads[0].at, Pt::from_mm(10.0, 20.8));
        assert_eq!(lay.pads[0].net_name.as_deref(), Some("GND"));
        assert_eq!(lay.pads[1].net_name.as_deref(), Some("VCC"));
        assert_eq!(lay.tracks[0].net_name.as_deref(), Some("GND"));
        assert_eq!(lay.vias[0].net_name, None);
    }
}
