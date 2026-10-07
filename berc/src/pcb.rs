//! `.kicad_pcb` to sorted text: board settings and stackup, footprints with their pad nets, nets, per-layer
//! track, via and zone counts, the outline size, graphics counts and text.

use crate::common::{
    Counted, Options, Out, SortKey, at_of, compact, hash, natural_cmp, num, q, quote, sort_natural,
};
use crate::sch::{header, table, title_block};
use crate::sexpr::Sexp;
use std::collections::BTreeMap;

/// Footprint properties shown on the footprint line, or never shown
const MAIN_FIELDS: &[&str] = &["Reference", "Value", "Footprint"];

#[derive(Default)]
struct NetStats {
    pads: usize,
    tracks: usize,
    vias: usize,
}

#[derive(Default)]
struct LayerStats {
    tracks: usize,
    arcs: usize,
    zones: usize,
}

pub fn render(root: &Sexp, opts: Options) -> String {
    let mut out = Out::default();
    out.line(header("kicad_pcb", root));

    let mut title = Out::default();
    let mut board = Vec::new();
    let mut footprints = Vec::new();
    let mut nets: BTreeMap<SortKey, NetStats> = BTreeMap::new();
    let mut layers: BTreeMap<String, LayerStats> = BTreeMap::new();
    let mut vias = Counted::default();
    let mut zones = Counted::default();
    let mut graphics = Counted::default();
    let mut texts = Counted::default();
    let mut tables = Vec::new();
    let mut routing = Counted::default();
    let mut other = Counted::default();
    let mut outline = BBox::default();
    let mut net_names: BTreeMap<String, String> = BTreeMap::new();

    // Before KiCad 10 tracks name their net by number, declared at the top: `(net 3 "GND")`
    for n in root.children("net") {
        if let (Some(id), Some(name)) = (n.arg(0), n.arg(1)) {
            net_names.insert(id.to_string(), name.to_string());
            if !name.is_empty() {
                nets.entry(SortKey(name.to_string())).or_default();
            }
        }
    }
    let net_of = |e: &Sexp| -> Option<String> {
        let n = e.child("net")?;
        let name = match n.args() {
            [Sexp::Str(s)] => s.to_string(),
            [id] => net_names.get(id.text()?).cloned().unwrap_or_default(),
            [_, name, ..] => name.text()?.to_string(),
            [] => return None,
        };
        (!name.is_empty()).then_some(name)
    };

    for item in root.args() {
        let Some(head) = item.head() else {
            other.add(compact(item, opts));
            continue;
        };
        match head {
            "version" | "generator" | "generator_version" | "uuid" | "net" => {}
            "general" => board.push(format!("general {}", args_compact(item, opts))),
            "paper" => board.push(format!(
                "paper {}",
                item.args()
                    .iter()
                    .filter_map(Sexp::text)
                    .map(q)
                    .collect::<Vec<_>>()
                    .join(" ")
            )),
            "title_block" => title_block(item, &mut title),
            "layers" => {
                for l in item.args() {
                    let names: Vec<String> = l
                        .items()
                        .iter()
                        .skip(1)
                        .filter_map(Sexp::text)
                        .map(q)
                        .collect();
                    board.push(format!("layer {}", names.join(" ")));
                }
            }
            "setup" => setup(item, opts, &mut board),
            "property" => board.push(format!(
                "property {}={}",
                q(item.arg(0).unwrap_or("")),
                q(item.arg(1).unwrap_or(""))
            )),
            "variants" => {
                for v in item.children("variant") {
                    let mut line = format!("variant {}", q(v.value("name").unwrap_or("?")));
                    if let Some(d) = v.value("description") {
                        line += &format!(" {}", quote(d));
                    }
                    board.push(line);
                }
            }
            "footprint" | "module" => {
                let fp = footprint(item, opts, &net_of);
                for pad_net in &fp.nets {
                    nets.entry(SortKey(pad_net.clone())).or_default().pads += 1;
                }
                outline.add_footprint(item);
                footprints.push(fp.text);
            }
            "segment" | "arc" => {
                let layer = item.value("layer").unwrap_or("?").to_string();
                let stats = layers.entry(layer.clone()).or_default();
                if head == "segment" {
                    stats.tracks += 1
                } else {
                    stats.arcs += 1
                }
                let net = net_of(item);
                if let Some(n) = &net {
                    nets.entry(SortKey(n.clone())).or_default().tracks += 1;
                }
                if opts.with_geometry {
                    routing.add(format!(
                        "{head} {} {} {} w={}",
                        q(&layer),
                        q(net.as_deref().unwrap_or("-")),
                        ends(item),
                        item.value("width").unwrap_or("?")
                    ));
                }
            }
            "via" => {
                let net = net_of(item);
                if let Some(n) = &net {
                    nets.entry(SortKey(n.clone())).or_default().vias += 1;
                }
                let kind = item
                    .arg(0)
                    .filter(|a| matches!(*a, "blind" | "micro" | "through" | "buried"));
                let span: Vec<&str> = item
                    .child("layers")
                    .map(|l| l.args().iter().filter_map(Sexp::text).collect())
                    .unwrap_or_default();
                let mut line = format!(
                    "via {}/{} {}",
                    item.value("size").unwrap_or("?"),
                    item.value("drill").unwrap_or("?"),
                    span.join("-")
                );
                if let Some(k) = kind {
                    line += &format!(" {k}");
                }
                if opts.with_geometry {
                    line += &format!(" {} {}", q(net.as_deref().unwrap_or("-")), at_of(item));
                }
                vias.add(line);
            }
            "zone" => {
                for layer in zone_layers(item) {
                    layers.entry(layer).or_default().zones += 1;
                }
                zones.add(zone(item, opts, net_of(item)));
            }
            "gr_line" | "gr_arc" | "gr_circle" | "gr_rect" | "gr_poly" | "gr_curve" | "gr_bbox" => {
                let layer = item.value("layer").unwrap_or("?");
                if layer == "Edge.Cuts" {
                    outline.add_item(item, None);
                }
                if opts.with_geometry {
                    graphics.add(compact(item, opts));
                } else {
                    graphics.add(format!("{head} {}", q(layer)));
                }
            }
            "gr_text" | "gr_text_box" => {
                let mut line = format!(
                    "{head} {} {}",
                    q(item.value("layer").unwrap_or("?")),
                    quote(item.arg(0).unwrap_or(""))
                );
                if opts.with_geometry {
                    line += &format!(" {}", at_of(item));
                }
                texts.add(line);
            }
            "table" => tables.push(table(item)),
            "dimension" => {
                let text = item.child("gr_text").and_then(|t| t.arg(0)).unwrap_or("");
                let mut line = format!(
                    "dimension {} {} {}",
                    item.value("type").unwrap_or("?"),
                    q(item.value("layer").unwrap_or("?")),
                    quote(text)
                );
                if opts.with_geometry {
                    line += &format!(" {}", points(item));
                }
                texts.add(line);
            }
            "image" => graphics.add(format!("image {}", hash(item.value("data").unwrap_or("")))),
            "group" => {
                let members = item.child("members").map_or(0, |m| m.args().len());
                other.add(format!(
                    "group {} members={members}",
                    q(item.arg(0).unwrap_or(""))
                ));
            }
            "embedded_files" => {
                for f in item.children("file") {
                    let data = hash(f.value("data").unwrap_or(""));
                    other.add(format!(
                        "embedded_file {} {} {data}",
                        q(f.value("name").unwrap_or("?")),
                        f.value("type").unwrap_or("?")
                    ));
                }
            }
            _ => other.add(compact(item, opts)),
        }
    }

    if let Some((w, h)) = outline.size() {
        board.push(format!("outline {} x {} mm", num(w, 1), num(h, 1)));
    }

    out.append(title);
    let fp_count = footprints.len();
    sort_natural(&mut footprints);
    tables.sort();
    let net_lines: Vec<String> = nets
        .iter()
        .map(|(name, s)| {
            format!(
                "net {}  pads={} tracks={} vias={}",
                q(&name.0),
                s.pads,
                s.tracks,
                s.vias
            )
        })
        .collect();
    let mut layer_names: Vec<&String> = layers.keys().collect();
    layer_names.sort_by(|a, b| {
        layer_order(a)
            .cmp(&layer_order(b))
            .then_with(|| natural_cmp(a, b))
    });
    let mut copper: Vec<String> = layer_names
        .into_iter()
        .map(|l| {
            let s = &layers[l];
            format!(
                "{}  tracks={} arcs={} zones={}",
                q(l),
                s.tracks,
                s.arcs,
                s.zones
            )
        })
        .collect();
    copper.extend(vias.lines());

    out.section("board", board.len(), &board);
    out.section("footprints", fp_count, &footprints);
    out.section("nets", net_lines.len(), &net_lines);
    out.section("copper", copper.len(), &copper);
    out.section("zones", zones.total(), &zones.lines());
    out.section("graphics", graphics.total(), &graphics.lines());
    out.section("text", texts.total(), &texts.lines());
    out.section("tables", tables.len(), &tables);
    out.section("routing", routing.total(), &routing.lines());
    out.section("other", other.total(), &other.lines());
    out.finish()
}

/// Copper layers in stack order (F.Cu, In1.Cu, ..., B.Cu), then the rest
fn layer_order(name: &str) -> (u8, u32) {
    match name {
        "F.Cu" => (0, 0),
        "B.Cu" => (2, 0),
        _ => match name
            .strip_prefix("In")
            .and_then(|n| n.strip_suffix(".Cu"))
            .and_then(|n| n.parse().ok())
        {
            Some(n) => (1, n),
            None => (3, 0),
        },
    }
}

fn args_compact(item: &Sexp, opts: Options) -> String {
    item.args()
        .iter()
        .map(|a| compact(a, opts))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Setup: the stackup one layer per line, plot parameters one per line, every other rule on its own line
fn setup(item: &Sexp, opts: Options, lines: &mut Vec<String>) {
    for c in item.args() {
        match c.head() {
            Some("stackup") => {
                for l in c.args() {
                    match l.head() {
                        Some("layer") => lines.push(format!("stackup {}", args_compact(l, opts))),
                        _ => lines.push(format!("stackup {}", compact(l, opts))),
                    }
                }
            }
            Some("pcbplotparams") => {
                for p in c.args() {
                    lines.push(format!("plot {}", args_compact_head(p, opts)));
                }
            }
            _ => lines.push(format!("setup {}", args_compact_head(c, opts))),
        }
    }
}

/// `(pad_to_mask_clearance 0)` as `pad_to_mask_clearance 0`
fn args_compact_head(e: &Sexp, opts: Options) -> String {
    match e.head() {
        Some(h) => {
            let rest = args_compact(e, opts);
            if rest.is_empty() {
                h.to_string()
            } else {
                format!("{h} {rest}")
            }
        }
        None => compact(e, opts),
    }
}

/// `(pts (xy 1 2) (xy 3 4))` as `(1,2)-(3,4)`
fn points(item: &Sexp) -> String {
    let pts: Vec<String> = item
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
    pts.join("-")
}

/// `(start x y) (end x y)` as `(x,y)-(x,y)`, endpoints in a stable order
fn ends(item: &Sexp) -> String {
    let p = |h: &str| {
        item.child(h)
            .map(|c| format!("({},{})", c.arg(0).unwrap_or("?"), c.arg(1).unwrap_or("?")))
    };
    let mut pts: Vec<String> = [p("start"), p("mid"), p("end")]
        .into_iter()
        .flatten()
        .collect();
    if pts.len() == 2 {
        pts.sort();
    } else if pts.len() == 3 && pts[0] > pts[2] {
        pts.swap(0, 2);
    }
    pts.join("-")
}

struct Footprint {
    text: String,
    nets: Vec<String>,
}

fn footprint(item: &Sexp, opts: Options, net_of: &dyn Fn(&Sexp) -> Option<String>) -> Footprint {
    let prop = |name: &str| {
        item.children("property")
            .find(|p| p.arg(0) == Some(name))
            .and_then(|p| p.arg(1))
    };
    // Before KiCad 8 reference and value were fp_text items
    let fp_text = |kind: &str| {
        item.children("fp_text")
            .find(|t| t.arg(0) == Some(kind))
            .and_then(|t| t.arg(1))
    };
    let reference = prop("Reference")
        .or_else(|| fp_text("reference"))
        .unwrap_or("?");
    let value = prop("Value").or_else(|| fp_text("value")).unwrap_or("");
    let lib = item.arg(0).unwrap_or("?");
    let side = match item.value("layer") {
        Some("F.Cu") => "top",
        Some("B.Cu") => "bottom",
        Some(other) => other,
        None => "?",
    };
    let rot = item
        .child("at")
        .and_then(|a| a.num(2))
        .unwrap_or(0.0)
        .rem_euclid(360.0);
    let mut head = format!(
        "{} {}  fp={}  side={side}  rot={}",
        q(reference),
        q(value),
        q(lib),
        num(rot, 1)
    );
    if let Some(attr) = item.child("attr") {
        for a in attr.args().iter().filter_map(Sexp::text) {
            head += &format!("  {a}");
        }
    }
    if item.flag("locked") == Some(true)
        || item
            .args()
            .iter()
            .any(|a| matches!(a, Sexp::Atom("locked")))
    {
        head += "  locked";
    }
    if opts.with_geometry {
        let at = item.child("at");
        let c = |n| {
            at.and_then(|a| a.num(n))
                .map_or("?".to_string(), |v| num(v, 1))
        };
        head += &format!("  @({},{})", c(0), c(1));
    }

    let mut sub = Vec::new();
    for p in item.children("property") {
        let (Some(name), Some(v)) = (p.arg(0), p.arg(1)) else {
            continue;
        };
        if !MAIN_FIELDS.contains(&name) && !v.is_empty() && v != "~" && !name.starts_with("ki_") {
            sub.push(format!("{}={}", q(name), q(v)));
        }
    }
    for t in item.children("fp_text").chain(item.children("fp_text_box")) {
        if t.arg(0) == Some("user") {
            let text = t.arg(1).unwrap_or("");
            if text != "${REFERENCE}" && text != "${VALUE}" {
                sub.push(format!(
                    "text {} {}",
                    q(t.value("layer").unwrap_or("?")),
                    quote(text)
                ));
            }
        }
    }
    for m in item.children("model") {
        sub.push(format!("model {}", q(m.arg(0).unwrap_or(""))));
    }
    for v in item.children("variant") {
        let name = q(v.value("name").unwrap_or("?"));
        for c in v.args() {
            let line = match c.head() {
                Some("name") => continue,
                Some("field") => format!(
                    "{}={}",
                    q(c.value("name").unwrap_or("?")),
                    q(c.value("value").unwrap_or(""))
                ),
                Some(flag) if c.args().len() == 1 && c.arg(0).is_some() => {
                    format!("{flag}={}", c.arg(0).unwrap_or(""))
                }
                _ => compact(c, opts),
            };
            sub.push(format!("variant {name} {line}"));
        }
    }
    let mut pads = Counted::default();
    let mut nets = Vec::new();
    for pad in item.children("pad") {
        let net = net_of(pad);
        let mut line = format!(
            "pad {} {}",
            q(pad.arg(0).unwrap_or("")),
            q(net.as_deref().unwrap_or("-"))
        );
        if let Some(pin) = pad.value("pinfunction") {
            // KiCad 10 writes the pin name with the pad number appended: `K_1`
            let number = pad.arg(0).unwrap_or("");
            let pin = pin
                .strip_suffix(number)
                .and_then(|p| p.strip_suffix('_'))
                .unwrap_or(pin);
            if !pin.is_empty() && pin != number && pin != "~" {
                line += &format!(" ({})", q(pin));
            }
        }
        pads.add(line);
        nets.extend(net);
    }
    sort_natural(&mut sub);
    let mut lines = vec![head];
    lines.extend(
        sub.into_iter()
            .chain(pads.lines())
            .map(|l| format!("  {l}")),
    );
    Footprint {
        text: lines.join("\n"),
        nets,
    }
}

fn zone_layers(zone: &Sexp) -> Vec<String> {
    let mut layers: Vec<String> = zone
        .child("layers")
        .map(|l| {
            l.args()
                .iter()
                .filter_map(Sexp::text)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    layers.extend(zone.value("layer").map(str::to_string));
    layers
}

/// A zone in one line: net, layers, then its settings without outline and fill polygons
fn zone(item: &Sexp, opts: Options, net: Option<String>) -> String {
    let skip = [
        "net",
        "net_name",
        "name",
        "layer",
        "layers",
        "polygon",
        "filled_polygon",
        "fill_segments",
        "hatch",
        "uuid",
        "tstamp",
    ];
    let rest: Vec<String> = item
        .args()
        .iter()
        .filter(|c| c.head().is_none_or(|h| !skip.contains(&h)))
        .map(|c| compact(c, opts))
        .collect();
    let mut line = format!(
        "zone {} {}",
        q(net.as_deref().unwrap_or("-")),
        zone_layers(item).join(",")
    );
    if let Some(name) = item.value("name") {
        line += &format!(" {}", q(name));
    }
    line += &format!("  {}", rest.join(" "));
    if opts.with_geometry {
        let pts = item
            .child("polygon")
            .and_then(|p| p.child("pts"))
            .map_or(0, |p| p.args().len());
        line += &format!(" outline_points={pts}");
    }
    line
}

/// Bounding box of the Edge.Cuts items
#[derive(Default)]
struct BBox {
    min: Option<(f64, f64)>,
    max: (f64, f64),
}

impl BBox {
    fn add_point(&mut self, x: f64, y: f64) {
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        match self.min {
            None => {
                self.min = Some((x, y));
                self.max = (x, y);
            }
            Some((mx, my)) => {
                self.min = Some((mx.min(x), my.min(y)));
                self.max = (self.max.0.max(x), self.max.1.max(y));
            }
        }
    }

    /// Adds the points of a graphic item; `place` moves footprint-local points onto the board
    fn add_item(&mut self, item: &Sexp, place: Option<&dyn Fn(f64, f64) -> (f64, f64)>) {
        let mut pts = Vec::new();
        let xy = |c: &Sexp| Some((c.num(0)?, c.num(1)?));
        for h in ["start", "mid", "end"] {
            pts.extend(item.child(h).and_then(xy));
        }
        if let Some(p) = item.child("pts") {
            pts.extend(p.children("xy").filter_map(xy));
        }
        if let (Some(c), Some(e)) = (
            item.child("center").and_then(xy),
            item.child("end").and_then(xy),
        ) {
            // A circle: center and a point on it
            let r = ((e.0 - c.0).powi(2) + (e.1 - c.1).powi(2)).sqrt();
            pts.extend([(c.0 - r, c.1 - r), (c.0 + r, c.1 + r)]);
        }
        for (x, y) in pts {
            let (x, y) = match place {
                Some(f) => f(x, y),
                None => (x, y),
            };
            self.add_point(x, y);
        }
    }

    fn add_footprint(&mut self, fp: &Sexp) {
        let at = fp.child("at");
        let (fx, fy) = (
            at.and_then(|a| a.num(0)).unwrap_or(0.0),
            at.and_then(|a| a.num(1)).unwrap_or(0.0),
        );
        let rot = at.and_then(|a| a.num(2)).unwrap_or(0.0).to_radians();
        let place = |x: f64, y: f64| {
            (
                fx + x * rot.cos() + y * rot.sin(),
                fy - x * rot.sin() + y * rot.cos(),
            )
        };
        for g in fp.args() {
            if matches!(
                g.head(),
                Some("fp_line" | "fp_arc" | "fp_circle" | "fp_rect" | "fp_poly")
            ) && g.value("layer") == Some("Edge.Cuts")
            {
                self.add_item(g, Some(&place));
            }
        }
    }

    fn size(&self) -> Option<(f64, f64)> {
        let (x0, y0) = self.min?;
        Some((self.max.0 - x0, self.max.1 - y0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sexpr::parse;

    fn render_str(text: &str, with_geometry: bool) -> String {
        let v = parse(text).unwrap();
        render(&v[0], Options { with_geometry })
    }

    const BOARD: &str = r#"(kicad_pcb (version 20260206) (generator "pcbnew") (generator_version "10.0")
        (general (thickness 1.6))
        (layers (0 "F.Cu" signal) (2 "B.Cu" signal) (25 "Edge.Cuts" user))
        (setup (stackup (layer "F.Cu" (type "copper") (thickness 0.035))) (pad_to_mask_clearance 0)
               (pcbplotparams (layerselection 0x1) (outputdirectory "gerbers/")))
        (footprint "Capacitor_SMD:C_0402" (layer "B.Cu") (uuid "u1") (at 10.04 20.01 -90)
          (property "Reference" "C2") (property "Value" "100nF") (property "Mpn1" "X")
          (attr smd dnp)
          (pad "1" smd rect (at 0 0) (net "GND") (uuid "p1"))
          (pad "2" smd rect (at 1 0) (net "/a/VCC") (uuid "p2")))
        (footprint "R" (layer "F.Cu") (at 0 0) (property "Reference" "C10") (property "Value" "1k")
          (pad "1" smd rect (net "GND")) (pad "2" smd rect))
        (segment (start 0 0) (end 1 0) (width 0.2) (layer "F.Cu") (net "GND") (uuid "s1"))
        (segment (start 1 0) (end 2 0) (width 0.2) (layer "B.Cu") (net "GND") (uuid "s2"))
        (via (at 1 0) (size 0.6) (drill 0.3) (layers "F.Cu" "B.Cu") (net "GND") (uuid "v1"))
        (zone (net "GND") (layers "F.Cu" "B.Cu") (uuid "z1") (priority 1) (polygon (pts (xy 0 0) (xy 1 1))))
        (gr_rect (start 0 0) (end 45.04 20.5) (layer "Edge.Cuts") (uuid "g1"))
        (gr_text "Hello" (at 1 2 0) (layer "F.SilkS") (uuid "t1")))"#;

    #[test]
    fn board_summary() {
        let out = render_str(BOARD, false);
        let c2 = out
            .find("C2 100nF  fp=Capacitor_SMD:C_0402  side=bottom  rot=270  smd  dnp\n")
            .expect(&out);
        let c10 = out.find("C10 1k  fp=R  side=top  rot=0\n").expect(&out);
        assert!(c2 < c10, "{out}");
        assert!(out.contains("  pad 1 GND\n  pad 2 /a/VCC\n"), "{out}");
        assert!(out.contains("  pad 2 -\n"), "{out}");
        assert!(out.contains("  Mpn1=X\n"), "{out}");
        assert!(out.contains("net GND  pads=2 tracks=2 vias=1"), "{out}");
        assert!(out.contains("F.Cu  tracks=1 arcs=0 zones=1\nB.Cu  tracks=1 arcs=0 zones=1\nvia 0.6/0.3 F.Cu-B.Cu\n"), "{out}");
        assert!(out.contains("zone GND F.Cu,B.Cu  (priority 1)"), "{out}");
        assert!(out.contains("outline 45 x 20.5 mm"), "{out}");
        assert!(
            out.contains("stackup \"F.Cu\" (type \"copper\") (thickness 0.035)"),
            "{out}"
        );
        assert!(out.contains("plot outputdirectory \"gerbers/\""), "{out}");
        assert!(out.contains("gr_text F.SilkS \"Hello\""), "{out}");
        assert!(!out.contains("@("), "{out}");
        assert!(!out.contains("u1") && !out.contains("\"s1\""), "{out}");
    }

    #[test]
    fn geometry_flag() {
        let out = render_str(BOARD, true);
        assert!(out.contains("rot=270  smd  dnp  @(10,20)"), "{out}");
        assert!(out.contains("segment F.Cu GND (0,0)-(1,0) w=0.2"), "{out}");
    }

    #[test]
    fn old_numbered_nets() {
        let out = render_str(
            r#"(kicad_pcb (version 20171130) (net 0 "") (net 1 "GND")
              (module "R" (layer F.Cu) (at 0 0) (fp_text reference R1 (at 0 0)) (fp_text value 1k (at 0 0))
                (pad 1 smd rect (net 1 GND)) (pad 2 smd rect (net 0 "")))
              (segment (start 0 0) (end 1 0) (width 0.2) (layer F.Cu) (net 1)))"#,
            false,
        );
        assert!(
            out.contains("R1 1k  fp=R  side=top  rot=0\n  pad 1 GND\n  pad 2 -\n"),
            "{out}"
        );
        assert!(out.contains("net GND  pads=1 tracks=1 vias=0"), "{out}");
    }
}
