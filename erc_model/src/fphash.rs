//! The footprint review hash: a short, versioned fingerprint of what a footprint review looks at, so a footprint
//! once checked against its datasheet can be told unchanged later, as a library file or as a board copy.
//!
//! It covers every pad (number, type, shape, size, position, rotation, drill and its offset, layers, mask, paste,
//! clearance and thermal settings, shape parameters, custom pad primitives, padstack), the footprint's own
//! clearance, mask and paste settings, its copper graphics and zones, and its graphics on the mask, paste,
//! courtyard and `Edge.Cuts` layers. It leaves out silk, fab and user layers, texts, the 3D model, UUIDs, fields,
//! attributes and the footprint's place on a board.
//!
//! The hash is taken over a canonical text ([`canonical`]) of the footprint in its own frame ([`LocalFootprint`],
//! board copies through [`crate::Layout::local_footprint`]): lengths on a 100 nm grid (KiCad rounds a footprint at
//! an odd angle to the nanometre on each save), angles in `[0, 360)`, pads, graphics and zones sorted, arcs by their
//! ends and true middle, polygons from their smallest corner, pads of a symmetric shape turned by less than 180°,
//! and values KiCad leaves out when they are its default left out too. A re-save, a reorder or a placement never
//! changes it; any change of the scope above does.
//!
//! It is written `fp1:` and 16 hex digits: the first 8 bytes of the SHA-256 of the canonical lines joined by `\n`
//! (no trailing newline), in lower-case hex. It is a fingerprint, not a signature: 64 bits keep a field short enough
//! to type and never collide by chance between footprints. A later change of the canonical text is a new version
//! (`fp2:`), so an old hash reads as made by another version, never as a silent mismatch.

use crate::dump::{shape_tokens, tok, zone_tokens};
use crate::footprint::{LocalFootprint, canonical_arc, deg360};
use crate::layout::*;
use sha2::{Digest, Sha256};

/// The version of the canonical text: the hash's prefix
pub const VERSION: &str = "fp1";

/// The review hash of a footprint in its own frame: `fp1:` and 16 hex digits
pub fn hash(f: &LocalFootprint) -> String {
    let text = canonical(f).join("\n");
    let digest = Sha256::digest(text.as_bytes());
    let hex: String = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
    format!("{VERSION}:{hex}")
}

/// How a stored review hash compares with a footprint
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Compared {
    /// The footprint is the one reviewed
    Same,
    /// The footprint changed since: the hash it has now
    Changed { now: String },
    /// Made by another version of the hash (`fp2:...`): this build cannot tell
    OtherVersion { version: String },
    /// Not a review hash at all
    Malformed,
}

/// Whether `s` is a review hash of this version: `fp1:` and 16 hex digits
pub fn is_hash(s: &str) -> bool {
    s.strip_prefix(VERSION)
        .and_then(|r| r.strip_prefix(':'))
        .is_some_and(|hex| hex.len() == 16 && hex.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Compares a stored review hash (surrounding spaces ignored) with a footprint
pub fn compare(stored: &str, f: &LocalFootprint) -> Compared {
    let stored = stored.trim();
    let Some((version, hex)) = stored.split_once(':') else {
        return Compared::Malformed;
    };
    if version.is_empty() || !version.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Compared::Malformed;
    }
    if version != VERSION {
        return Compared::OtherVersion {
            version: version.to_string(),
        };
    }
    if hex.len() != 16 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Compared::Malformed;
    }
    let now = hash(f);
    if now.eq_ignore_ascii_case(stored) {
        Compared::Same
    } else {
        Compared::Changed { now }
    }
}

/// The canonical text the hash is taken over, one sorted line per item: what a review covers, readable, so two
/// footprints with different hashes can be diffed line by line
pub fn canonical(f: &LocalFootprint) -> Vec<String> {
    let mut lines = Vec::new();
    let st = &f.footprint.settings;
    for (k, v) in [
        ("clearance", st.clearance),
        ("mask", st.solder_mask_margin),
        ("paste", st.solder_paste_margin),
    ] {
        if let Some(v) = v {
            lines.push(format!("footprint {k} {}", Nm(q(v))));
        }
    }
    if let Some(r) = st.solder_paste_ratio {
        lines.push(format!("footprint paste-ratio {}", ratio(r)));
    }
    if let Some(z) = st.zone_connect {
        lines.push(format!("footprint zone-connect {z}"));
    }
    lines.extend(f.pads.iter().map(pad_line));
    for g in f.footprint.graphics.iter().filter(|g| in_scope(&g.layer)) {
        lines.push(format!("gfx {}", shape_tokens(&shape(g)).join(" ")));
    }
    for z in &f.footprint.zones {
        lines.push(format!("zone {}", zone_tokens(&zone(z)).join(" ")));
    }
    lines.sort();
    lines
}

/// Graphics a review covers: copper, mask, paste, courtyard and the board edge
fn in_scope(layer: &str) -> bool {
    layer.ends_with(".Cu")
        || matches!(
            layer,
            "F.Mask" | "B.Mask" | "F.Paste" | "B.Paste" | "F.CrtYd" | "B.CrtYd" | "Edge.Cuts"
        )
}

/// A length on the 100 nm grid
fn q(v: i64) -> i64 {
    (v as f64 / 100.0).round() as i64 * 100
}

fn qp(p: Pt) -> Pt {
    Pt::new(q(p.x), q(p.y))
}

/// A ratio to four decimals
fn ratio(r: f64) -> String {
    format!("{}", (r * 1e4).round() / 1e4 + 0.0)
}

/// An angle in `[0, 360)` to a hundredth of a degree
fn angle(a: f64) -> f64 {
    deg360((deg360(a) * 100.0).round() / 100.0)
}

fn layer_set(ls: &[String]) -> String {
    let mut ls = ls.to_vec();
    ls.sort();
    ls.dedup();
    tok(&ls.join(","))
}

fn pad_line(p: &Pad) -> String {
    let d = &p.details;
    let offset = d.drill_offset.map(qp).filter(|o| *o != Pt::default());
    let mut rot = angle(p.rotation);
    let mut size = qp(p.size);
    let mut drill = p.drill.map(qp);
    // Shapes that look the same turned half round: a turn of 90° is the sides swapped
    let symmetric = matches!(p.shape.as_str(), "rect" | "oval" | "roundrect")
        && d.chamfer.is_empty()
        && offset.is_none()
        && p.primitives.is_empty();
    let round = p.shape == "circle"
        && offset.is_none()
        && p.primitives.is_empty()
        && drill.is_none_or(|d| d.x == d.y);
    if round {
        rot = 0.0;
    } else if symmetric {
        rot %= 180.0;
        if rot == 90.0 {
            rot = 0.0;
            size = Pt::new(size.y, size.x);
            drill = drill.map(|d| Pt::new(d.y, d.x));
        }
    }
    let mut line = format!(
        "pad {} {} {} at {} rot {rot} size {size}",
        tok(&p.number),
        tok(&p.kind),
        tok(&p.shape),
        qp(p.at),
    );
    if let Some(d) = drill {
        line += &format!(" drill {d}");
    }
    if let Some(o) = offset {
        line += &format!(" drill-offset {o}");
    }
    line += &format!(" layers {}", layer_set(&p.layers));
    if let Some(prop) = &p.property {
        line += &format!(" prop {}", tok(prop));
    }
    for (k, v) in [
        ("mask", p.solder_mask_margin),
        ("paste", p.solder_paste_margin),
        ("clearance", d.clearance),
        ("thermal-width", d.thermal_bridge_width),
        ("thermal-gap", d.thermal_gap),
    ] {
        if let Some(v) = v {
            line += &format!(" {k} {}", Nm(q(v)));
        }
    }
    if let Some(r) = p.solder_paste_ratio {
        line += &format!(" paste-ratio {}", ratio(r));
    }
    if let Some(z) = p.zone_connect {
        line += &format!(" zone-connect {z}");
    }
    if let Some(r) = d.roundrect_rratio.filter(|_| p.shape == "roundrect") {
        line += &format!(" rratio {}", ratio(r));
    }
    if !d.chamfer.is_empty() {
        line += &format!(" chamfer {}", layer_set(&d.chamfer));
        if let Some(r) = d.chamfer_ratio {
            line += &format!(" chamfer-ratio {}", ratio(r));
        }
    }
    if let Some(delta) = d.rect_delta.map(qp).filter(|x| *x != Pt::default()) {
        line += &format!(" delta {delta}");
    }
    if p.shape == "custom" {
        for (k, v) in [
            ("anchor", &d.anchor),
            ("custom-clearance", &d.custom_clearance),
        ] {
            if let Some(v) = v {
                line += &format!(" {k} {}", tok(v));
            }
        }
    }
    // KiCad writes the spoke angle only when it is not the default: 45° for a round pad, 90° otherwise
    let round_anchor =
        p.shape == "circle" || p.shape == "custom" && d.anchor.as_deref() == Some("circle");
    let default_spokes = if round_anchor { 45.0 } else { 90.0 };
    if let Some(a) = d
        .thermal_bridge_angle
        .map(angle)
        .filter(|a| *a != default_spokes)
    {
        line += &format!(" thermal-angle {a}");
    }
    if let Some(ps) = &d.padstack {
        line += &format!(" padstack {}", tok(ps));
    }
    let mut prims: Vec<String> = p
        .primitives
        .iter()
        .map(|s| shape_tokens(&shape(s)).join(" "))
        .collect();
    prims.sort();
    for s in prims {
        line += &format!(" prim [{s}]");
    }
    line
}

/// A polygon from its smallest corner, in the direction that reads smaller; a closing repeat of the first corner
/// dropped
fn polygon(ps: &[Pt]) -> Vec<Pt> {
    let mut ps: Vec<Pt> = ps.iter().map(|p| qp(*p)).collect();
    if ps.len() > 1 && ps.first() == ps.last() {
        ps.pop();
    }
    let Some(start) = (0..ps.len()).min_by_key(|i| ps[*i]) else {
        return ps;
    };
    ps.rotate_left(start);
    let mut rev = ps.clone();
    rev[1..].reverse();
    ps.min(rev)
}

fn shape(s: &Shape) -> Shape {
    let geom = match &s.geom {
        Geom::Line(a, b) => {
            let (a, b) = (qp(*a), qp(*b));
            Geom::Line(a.min(b), a.max(b))
        }
        Geom::Rect(a, b) => {
            let (a, b) = (qp(*a), qp(*b));
            Geom::Rect(
                Pt::new(a.x.min(b.x), a.y.min(b.y)),
                Pt::new(a.x.max(b.x), a.y.max(b.y)),
            )
        }
        Geom::Circle { center, radius } => Geom::Circle {
            center: qp(*center),
            radius: q(*radius),
        },
        Geom::Arc { start, mid, end } => {
            let (start, mid, end) = canonical_arc(qp(*start), *mid, qp(*end));
            Geom::Arc {
                start,
                mid: qp(mid),
                end,
            }
        }
        Geom::Poly(ps) => Geom::Poly(polygon(ps)),
        Geom::Bezier(ps) => {
            let fwd = ps.map(qp);
            let mut rev = fwd;
            rev.reverse();
            Geom::Bezier(fwd.min(rev))
        }
    };
    Shape {
        layer: s.layer.clone(),
        geom,
        width: q(s.width),
        filled: s.filled,
    }
}

/// A zone without what a review does not look at (name, net, fill state), its keepouts sorted
fn zone(z: &Zone) -> Zone {
    let mut keepout = z.keepout.clone();
    keepout.sort();
    let mut layers = z.layers.clone();
    layers.sort();
    Zone {
        name: None,
        net_name: None,
        layers,
        outline: polygon(&z.outline),
        priority: z.priority,
        connect_pads: z.connect_pads.clone(),
        clearance: z.clearance.map(q),
        min_thickness: z.min_thickness.map(q),
        thermal_gap: z.thermal_gap.map(q),
        thermal_bridge_width: z.thermal_bridge_width.map(q),
        keepout,
        filled: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(number: &str, shape: &str, at: Pt, rot: f64, size: Pt) -> Pad {
        Pad {
            footprint: FootprintId(0),
            number: number.into(),
            kind: "smd".into(),
            shape: shape.into(),
            at,
            rotation: rot,
            size,
            drill: None,
            layers: vec!["F.Cu".into(), "F.Mask".into(), "F.Paste".into()],
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
            details: PadDetails::default(),
        }
    }

    fn footprint(pads: Vec<Pad>, graphics: Vec<Shape>) -> LocalFootprint {
        LocalFootprint {
            footprint: Footprint {
                reference: "R1".into(),
                part: None,
                schematic_path: None,
                lib_id: "R".into(),
                value: "10k".into(),
                side: Side::Front,
                at: Pt::default(),
                rotation: 0.0,
                attrs: FootprintAttrs::default(),
                fields: Vec::new(),
                pads: (0..pads.len()).map(PadId::from_index).collect(),
                graphics,
                texts: Vec::new(),
                net_tie_pad_groups: Vec::new(),
                models: Vec::new(),
                settings: FootprintSettings::default(),
                zones: Vec::new(),
            },
            pads,
        }
    }

    fn sample() -> LocalFootprint {
        footprint(
            vec![
                pad(
                    "1",
                    "roundrect",
                    Pt::from_mm(-0.8, 0.0),
                    0.0,
                    Pt::from_mm(0.8, 0.95),
                ),
                pad(
                    "2",
                    "roundrect",
                    Pt::from_mm(0.8, 0.0),
                    0.0,
                    Pt::from_mm(0.8, 0.95),
                ),
            ],
            vec![
                Shape {
                    layer: "F.CrtYd".into(),
                    geom: Geom::Rect(Pt::from_mm(-1.5, -0.7), Pt::from_mm(1.5, 0.7)),
                    width: 50_000,
                    filled: false,
                },
                Shape {
                    layer: "F.SilkS".into(),
                    geom: Geom::Line(Pt::from_mm(-0.2, -0.6), Pt::from_mm(0.2, -0.6)),
                    width: 120_000,
                    filled: false,
                },
            ],
        )
    }

    #[test]
    fn form_and_version() {
        let h = hash(&sample());
        assert!(h.starts_with("fp1:") && h.len() == 4 + 16, "{h}");
        assert!(
            is_hash(&h)
                && !is_hash("fp1:123")
                && !is_hash("fp2:0123456789abcdef")
                && !is_hash(" fp1:0123456789abcdef")
        );
        assert_eq!(compare(&h, &sample()), Compared::Same);
        assert_eq!(
            compare(
                &format!(" {} ", h.to_uppercase().replace("FP1", "fp1")),
                &sample()
            ),
            Compared::Same
        );
        assert_eq!(
            compare("fp2:0123", &sample()),
            Compared::OtherVersion {
                version: "fp2".into()
            }
        );
        assert_eq!(compare("fp1:xyz", &sample()), Compared::Malformed);
        assert_eq!(compare("reviewed", &sample()), Compared::Malformed);
        let mut other = sample();
        other.pads[0].size.x += 50_000;
        assert_eq!(compare(&h, &other), Compared::Changed { now: hash(&other) });
    }

    #[test]
    fn silk_texts_and_order_keep_the_hash() {
        let h = hash(&sample());
        let mut f = sample();
        f.footprint.graphics[1].geom = Geom::Circle {
            center: Pt::default(),
            radius: 1,
        };
        f.footprint.graphics.reverse();
        f.pads.reverse();
        f.footprint.texts.push(BoardText {
            layer: "F.SilkS".into(),
            text: "REF**".into(),
            at: Pt::default(),
            size: 1_000_000,
            hidden: false,
            mirrored: false,
        });
        f.footprint.reference = "R2".into();
        f.footprint.models.push("R.step".into());
        f.pads[0].layers.reverse();
        f.pads[1].net_name = Some("GND".into());
        assert_eq!(hash(&f), h);
    }

    #[test]
    fn what_a_review_covers_changes_it() {
        let h = hash(&sample());
        type Change = Box<dyn Fn(&mut LocalFootprint)>;
        let changes: Vec<(&str, Change)> = vec![
            ("pad size", Box::new(|f| f.pads[0].size.y = 1_000_000)),
            ("pad number", Box::new(|f| f.pads[1].number = "3".into())),
            ("pad position", Box::new(|f| f.pads[1].at.x += 100_000)),
            (
                "pad mask margin",
                Box::new(|f| f.pads[0].solder_mask_margin = Some(50_000)),
            ),
            (
                "pad paste margin",
                Box::new(|f| f.pads[0].solder_paste_margin = Some(-50_000)),
            ),
            (
                "pad layers",
                Box::new(|f| f.pads[0].layers.pop().map(|_| ()).unwrap()),
            ),
            (
                "roundrect ratio",
                Box::new(|f| f.pads[0].details.roundrect_rratio = Some(0.1)),
            ),
            (
                "courtyard",
                Box::new(|f| f.footprint.graphics[0].width = 100_000),
            ),
            (
                "footprint mask",
                Box::new(|f| f.footprint.settings.solder_mask_margin = Some(1)),
            ),
            (
                "copper graphic",
                Box::new(|f| {
                    let mut g = f.footprint.graphics[1].clone();
                    g.layer = "F.Cu".into();
                    f.footprint.graphics.push(g)
                }),
            ),
        ];
        for (what, change) in changes {
            let mut f = sample();
            change(&mut f);
            assert_ne!(hash(&f), h, "{what}");
        }
    }

    #[test]
    fn same_pad_drawn_another_way() {
        let a = footprint(
            vec![pad("1", "rect", Pt::default(), 90.0, Pt::from_mm(1.0, 2.0))],
            vec![],
        );
        let b = footprint(
            vec![pad(
                "1",
                "rect",
                Pt::default(),
                180.0,
                Pt::from_mm(2.0, 1.0),
            )],
            vec![],
        );
        assert_eq!(hash(&a), hash(&b));
        let c = footprint(
            vec![pad(
                "1",
                "circle",
                Pt::default(),
                37.0,
                Pt::from_mm(1.0, 1.0),
            )],
            vec![],
        );
        let d = footprint(
            vec![pad(
                "1",
                "circle",
                Pt::default(),
                0.0,
                Pt::from_mm(1.0, 1.0),
            )],
            vec![],
        );
        assert_eq!(hash(&c), hash(&d));
        // A trapezoid is not symmetric
        let mut e = footprint(
            vec![pad(
                "1",
                "trapezoid",
                Pt::default(),
                0.0,
                Pt::from_mm(1.0, 1.0),
            )],
            vec![],
        );
        e.pads[0].details.rect_delta = Some(Pt::from_mm(0.0, 0.2));
        let mut f = e.clone();
        f.pads[0].rotation = 180.0;
        assert_ne!(hash(&e), hash(&f));
        // KiCad's 1 nm of rounding at an odd angle
        let mut g = sample();
        g.pads[0].at.x += 1;
        assert_eq!(hash(&g), hash(&sample()));
    }

    /// A footprint without vias keeps the hash it had before vias were looked at (hashes are written into symbols)
    #[test]
    fn hash_without_vias_stays() {
        assert_eq!(hash(&sample()), "fp1:a5158b65d05aa955");
    }

    /// A package's vias (KiCad has none in footprints: they are round plated pads, often numbered as the exposed pad
    /// they cool) are in the canonical text: one more, or one moved or resized, changes the hash
    #[test]
    fn package_vias_are_covered() {
        let mut f = sample();
        let mut via = pad(
            "1",
            "circle",
            Pt::from_mm(0.0, 0.0),
            0.0,
            Pt::from_mm(0.6, 0.6),
        );
        via.kind = "thru_hole".into();
        via.drill = Some(Pt::from_mm(0.3, 0.3));
        via.layers = vec!["*.Cu".into()];
        via.property = Some("pad_prop_heatsink".into());
        f.pads.push(via);
        let lines = canonical(&f);
        assert!(
            lines.contains(
                &"pad 1 thru_hole circle at 0,0 rot 0 size 0.6,0.6 drill 0.3,0.3 layers *.Cu prop pad_prop_heatsink"
                    .to_string()
            ),
            "{lines:#?}"
        );
        let with = hash(&f);
        assert_ne!(with, hash(&sample()));
        type Change = Box<dyn Fn(&mut Pad)>;
        let changes: Vec<(&str, Change)> = vec![
            ("moved", Box::new(|p| p.at.x += 100_000)),
            (
                "drill",
                Box::new(|p| p.drill = Some(Pt::from_mm(0.25, 0.25))),
            ),
            ("ring", Box::new(|p| p.size = Pt::from_mm(0.5, 0.5))),
            ("tented", Box::new(|p| p.layers.push("F.Mask".into()))),
        ];
        for (what, change) in changes {
            let mut g = f.clone();
            change(g.pads.last_mut().unwrap());
            assert_ne!(hash(&g), with, "{what}");
        }
    }

    #[test]
    fn polygons_from_any_corner_either_way() {
        let ps = [
            Pt::new(0, 0),
            Pt::new(1000, 0),
            Pt::new(1000, 1000),
            Pt::new(0, 1000),
        ];
        let mut rotated = ps.to_vec();
        rotated.rotate_left(2);
        let mut reversed = ps.to_vec();
        reversed.reverse();
        let mut closed = ps.to_vec();
        closed.push(ps[0]);
        for other in [rotated, reversed, closed] {
            assert_eq!(polygon(&other), polygon(&ps));
        }
    }
}
