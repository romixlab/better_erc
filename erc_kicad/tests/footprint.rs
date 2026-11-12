//! Library footprints (`.kicad_mod`) and their board copies. `fixtures/footprints/Test.pretty` holds two test
//! footprints written for this test; `placed.kicad_pcb` is a board KiCad 10 saved with each of them placed six
//! times (front and back, flipped top-bottom and left-right, at right and odd angles) by `place.py`
//! (`just footprint-fixture`). Every board copy brought back to its own frame equals its library file.

use erc_model::LocalFootprint;
use erc_model::footprint::{canonical_arc, deg360};
use erc_model::layout::*;
use serde_json::Value;
use std::path::Path;

fn fixture(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/footprints")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn library(name: &str) -> LocalFootprint {
    let file = format!("Test.pretty/{name}.kicad_mod");
    let (f, notes) = erc_kicad::footprint::read(&fixture(&file), &file).unwrap();
    assert!(notes.is_empty(), "{file}: {notes:#?}");
    f
}

#[test]
fn library_footprint_reads_every_pad_detail() {
    let f = library("Mixed_Pads");
    let fp = &f.footprint;
    assert_eq!(fp.lib_id, "Mixed_Pads");
    assert_eq!(
        (fp.at, fp.rotation, fp.side),
        (Pt::default(), 0.0, Side::Front)
    );
    assert_eq!(fp.attrs.mount.as_deref(), Some("through_hole"));
    assert_eq!(
        fp.settings,
        FootprintSettings {
            clearance: Some(200_000),
            zone_connect: Some(2),
            ..FootprintSettings::default()
        }
    );
    assert_eq!(fp.pads, (0..5).map(PadId::from_index).collect::<Vec<_>>());
    let pad = |n: &str| f.pads.iter().find(|p| p.number == n).unwrap();
    let p1 = pad("1");
    assert_eq!((p1.shape.as_str(), p1.rotation), ("oval", 30.0));
    assert_eq!(p1.drill, Some(Pt::from_mm(0.8, 1.4)));
    assert_eq!(p1.details.drill_offset, Some(Pt::from_mm(0.0, 0.2)));
    assert_eq!(p1.details.thermal_bridge_angle, Some(45.0));
    assert_eq!(pad("2").details.rect_delta, Some(Pt::from_mm(0.0, 0.3)));
    let p3 = &pad("3").details;
    assert_eq!(
        (p3.chamfer_ratio, p3.roundrect_rratio),
        (Some(0.25), Some(0.0))
    );
    assert_eq!(p3.chamfer, ["top_left", "bottom_right"]);
    assert_eq!(pad("3").solder_mask_margin, Some(20_000));
    let p4 = pad("4");
    assert_eq!(p4.primitives.len(), 2);
    assert_eq!(p4.details.anchor.as_deref(), Some("circle"));
    assert_eq!(p4.details.custom_clearance.as_deref(), Some("convexhull"));
    assert_eq!(pad("").kind, "np_thru_hole");
    assert_eq!(fp.zones.len(), 1);
    let z = &fp.zones[0];
    assert_eq!(z.layers, ["F.Cu"]);
    assert_eq!(z.outline[3], Pt::from_mm(1.4, 1.2));
    assert!(z.keepout.contains(&("tracks".into(), "not_allowed".into())));
    let layers: Vec<&str> = fp.graphics.iter().map(|g| g.layer.as_str()).collect();
    assert_eq!(
        layers,
        [
            "F.CrtYd",
            "F.CrtYd",
            "F.CrtYd",
            "F.CrtYd",
            "F.CrtYd",
            "F.Fab",
            "F.Cu",
            "Edge.Cuts"
        ]
    );
    let sot = library("Asym_SOT");
    assert_eq!(sot.footprint.settings.solder_mask_margin, Some(50_000));
    assert_eq!(sot.pads[0].details.roundrect_rratio, Some(0.25));
    assert_eq!(sot.pads[2].solder_paste_margin, Some(-30_000));
}

#[test]
fn kicad_8_footprint_thermal_settings() {
    // KiCad 8 wrote them on a footprint; KiCad 9 and later drop them
    let text = r#"(footprint "T" (version 20240108) (layer "F.Cu") (thermal_width 0.3) (thermal_gap 0.4)
        (solder_paste_margin -0.04) (solder_paste_ratio -0.1)
        (pad "1" smd rect (at 0 0) (size 1 1) (layers "F.Cu") (thermal_bridge_width 0.2) (thermal_gap 0.25)
          (clearance 0.15)))"#;
    let (f, notes) = erc_kicad::footprint::read(text, "T.kicad_mod").unwrap();
    assert!(notes.is_empty(), "{notes:#?}");
    let s = &f.footprint.settings;
    assert_eq!(
        (s.thermal_width, s.thermal_gap),
        (Some(300_000), Some(400_000))
    );
    assert_eq!(
        (s.solder_paste_margin, s.solder_paste_ratio),
        (Some(-40_000), Some(-0.1))
    );
    let d = &f.pads[0].details;
    assert_eq!(
        (d.thermal_bridge_width, d.thermal_gap, d.clearance),
        (Some(200_000), Some(250_000), Some(150_000))
    );
}

#[test]
fn not_a_footprint() {
    assert!(erc_kicad::footprint::read("(kicad_pcb (version 1))", "x").is_err());
    let e = erc_kicad::footprint::read("(module \"R\" (layer F.Cu))", "x").unwrap_err();
    assert!(e.to_string().contains("KiCad 5"), "{e}");
}

/// Arcs by their ends in order and their true middle (KiCad turns them round when it flips a footprint)
fn arc_canonical(g: &Shape) -> Shape {
    let mut g = g.clone();
    if let Geom::Arc { start, mid, end } = g.geom {
        let (start, mid, end) = canonical_arc(start, mid, end);
        g.geom = Geom::Arc { start, mid, end };
    }
    g
}

/// What a footprint review looks at, as JSON with lengths on a 100 nm grid (KiCad rounds a footprint at an odd
/// angle to the nanometre, there and back) and angles in `[0, 360)`; layer lists and chamfered corners as sets,
/// pads by number and position
fn comparable(f: &LocalFootprint) -> Value {
    fn snap(v: &mut Value) {
        match v {
            // Lengths; small numbers (zone_connect) stay
            Value::Number(n) if n.as_i64().is_some_and(|x| x.abs() >= 1000) => {
                let x = n.as_i64().unwrap();
                *v = Value::from((x as f64 / 100.0).round() as i64 * 100);
            }
            Value::Array(a) => {
                a.iter_mut().for_each(snap);
                if a.iter().all(Value::is_string) {
                    a.sort_by(|x, y| x.as_str().cmp(&y.as_str()));
                }
            }
            Value::Object(o) => o.values_mut().for_each(snap),
            _ => {}
        }
    }
    let mut pads: Vec<Pad> = f
        .pads
        .iter()
        .map(|p| Pad {
            rotation: deg360(p.rotation),
            primitives: p.primitives.iter().map(arc_canonical).collect(),
            ..p.clone()
        })
        .collect();
    pads.sort_by_key(|p| (p.number.clone(), p.at));
    let fp = &f.footprint;
    // KiCad keeps a footprint's graphics sorted its own way, not in the library file's order
    let mut graphics: Vec<String> = fp
        .graphics
        .iter()
        .map(|g| serde_json::to_string(&arc_canonical(g)).unwrap())
        .collect();
    graphics.sort();
    let graphics: Vec<Value> = graphics
        .iter()
        .map(|g| serde_json::from_str(g).unwrap())
        .collect();
    let mut v = serde_json::json!({
        "pads": pads,
        "graphics": graphics,
        "zones": fp.zones,
        "settings": fp.settings,
        "attrs": fp.attrs,
    });
    for p in v["pads"].as_array_mut().unwrap() {
        let o = p.as_object_mut().unwrap();
        for k in ["net_name", "net", "pin_function", "pin_type"] {
            o.remove(k);
        }
    }
    snap(&mut v);
    v
}

#[test]
fn board_copies_equal_their_library_file() {
    let (lay, notes) =
        erc_kicad::pcb::read(&fixture("placed.kicad_pcb"), "placed.kicad_pcb").unwrap();
    assert!(notes.is_empty(), "{notes:#?}");
    assert_eq!(lay.footprints.len(), 12);
    assert_eq!(lay.copper_layers(), 4);
    let libs = [library("Asym_SOT"), library("Mixed_Pads")];
    let mut sides = Vec::new();
    for (i, f) in lay.footprints.iter().enumerate() {
        let lib = libs
            .iter()
            .find(|l| l.footprint.lib_id == f.lib_id)
            .unwrap_or_else(|| panic!("{}: no library footprint", f.lib_id));
        let local = lay.local_footprint(FootprintId::from_index(i));
        assert_eq!(
            (local.footprint.at, local.footprint.side),
            (Pt::default(), Side::Front)
        );
        assert_eq!(
            comparable(&local),
            comparable(lib),
            "{} {} at {} rotation {} on the {:?}",
            f.reference,
            f.lib_id,
            f.at,
            f.rotation,
            f.side
        );
        sides.push((f.side, f.rotation));
    }
    // The fixture covers what it claims: both sides, right and odd angles
    assert!(sides.iter().any(|s| s.0 == Side::Back && s.1 % 90.0 != 0.0));
    assert!(
        sides
            .iter()
            .any(|s| s.0 == Side::Front && s.1 % 90.0 != 0.0)
    );
    assert!(
        sides
            .iter()
            .any(|s| s.0 == Side::Back && s.1 == 180.0 || s.1 == -180.0)
    );
}

#[test]
fn a_changed_copy_differs() {
    let text = fixture("placed.kicad_pcb").replacen("(size 1.05 0.6)", "(size 1.05 0.65)", 1);
    let (lay, _) = erc_kicad::pcb::read(&text, "placed.kicad_pcb").unwrap();
    let lib = library("Asym_SOT");
    let changed = (0..lay.footprints.len())
        .filter(|i| lay.footprints[*i].lib_id == "Asym_SOT")
        .filter(|i| {
            comparable(&lay.local_footprint(FootprintId::from_index(*i))) != comparable(&lib)
        })
        .count();
    assert_eq!(changed, 1);
}
