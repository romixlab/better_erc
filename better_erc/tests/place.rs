//! `berc place-fields` on a schematic written here: moved fields end up clear of everything, drawn where the plan
//! put them, and symbols with nothing in the way stay byte for byte

use better_erc::Project;
use better_erc::model::geometry::DrawnSymbol;
use better_erc::model::geometry::HAlign;
use better_erc::place::{PlaceOptions, Side, place};
use std::path::{Path, PathBuf};

const LIB: &str = r#"(lib_symbols
(symbol "Device:R" (pin_numbers (hide yes)) (pin_names (hide yes)) (exclude_from_sim no) (in_bom yes) (on_board yes)
(property "Reference" "R" (at 0 0 0) (effects (font (size 1.27 1.27))))
(property "Value" "R" (at 0 0 0) (effects (font (size 1.27 1.27))))
(symbol "R_0_1" (rectangle (start -1.016 -2.54) (end 1.016 2.54) (stroke (width 0.254) (type default)) (fill (type none))))
(symbol "R_1_1" (pin passive line (at 0 5.08 270) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27))))) (pin passive line (at 0 -5.08 90) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27))))))))
"#;

/// A two-pin part drawn as a resistor at `x`, `y` turned and mirrored, with its fields where `fields` puts them:
/// name, value, x, y, angle; its reference is its Reference field's
fn resistor(
    n: u32,
    at: (f64, f64, u32),
    mirror: &str,
    fields: &[(&str, &str, f64, f64, u32)],
) -> String {
    let (x, y, r) = at;
    let mut s = format!(
        "(symbol (lib_id \"Device:R\") (at {x} {y} {r}) {mirror}(unit 1) (exclude_from_sim no) (in_bom yes) \
         (on_board yes) (dnp no) (fields_autoplaced yes) (uuid \"a0000000-0000-4000-8000-00000000000{n}\")\n"
    );
    for (name, value, fx, fy, a) in fields {
        s += &format!(
            "  (property \"{name}\" \"{value}\" (at {fx} {fy} {a}) (effects (font (size 1.27 1.27)) (justify left)))\n"
        );
    }
    let reference = fields
        .iter()
        .find(|f| f.0 == "Reference")
        .map_or_else(|| format!("R{n}"), |f| f.1.to_owned());
    s += &format!(
        "  (pin \"1\" (uuid \"b{n}000000-0000-4000-8000-000000000001\")) (pin \"2\" (uuid \"b{n}000000-0000-4000-8000-000000000002\"))\n  \
         (instances (project \"place\" (path \"/a0000000-0000-4000-8000-000000000000\" (reference \"{reference}\") (unit 1)))))\n"
    );
    s
}

/// Four resistors: three with their fields piled on each other at several turns, one with its fields clear
fn schematic() -> String {
    let mut s = String::from(
        "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n\
         (uuid \"a0000000-0000-4000-8000-000000000000\")\n(paper \"A4\")\n",
    );
    s += LIB;
    let piled = |x: f64, y: f64| {
        vec![
            ("Reference", "R", x, y, 0),
            ("Value", "10k", x, y, 0),
            ("Power", "63mW", x, y + 0.5, 0),
        ]
    };
    let name = |n: u32, f: Vec<(&'static str, &'static str, f64, f64, u32)>| {
        f.into_iter()
            .map(|(k, v, x, y, a)| {
                (
                    k,
                    if k == "Reference" {
                        ["R1", "R2", "R3", "R4"][n as usize - 1]
                    } else {
                        v
                    },
                    x,
                    y,
                    a,
                )
            })
            .collect::<Vec<_>>()
    };
    s += &resistor(1, (50.8, 50.8, 0), "", &name(1, piled(52.07, 50.8)));
    s += &resistor(2, (101.6, 50.8, 90), "", &name(2, piled(100.33, 48.26)));
    s += &resistor(
        3,
        (152.4, 50.8, 180),
        "(mirror x) ",
        &name(3, piled(153.67, 50.8)),
    );
    let clear = vec![
        ("Reference", "R4", 53.34, 100.33, 0),
        ("Value", "1k", 53.34, 102.87, 0),
    ];
    s += &resistor(4, (50.8, 101.6, 0), "", &clear);
    // A wire past R1's right side, so its column goes elsewhere
    s += "(wire (pts (xy 54.61 40.64) (xy 54.61 60.96)) (stroke (width 0) (type default)) (uuid \"c0000000-0000-4000-8000-000000000001\"))\n";
    s += "(sheet_instances (path \"/\" (page \"1\")))\n)\n";
    s
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The text of the symbol with this UUID
fn symbol_text<'t>(text: &'t str, uuid: &str) -> &'t str {
    let at = text.find(uuid).unwrap();
    let start = text[..at].rfind("(symbol (lib_id").unwrap();
    let end = start + text[start..].find("(instances").unwrap();
    &text[start..end]
}

fn symbols(file: &Path) -> Vec<DrawnSymbol> {
    let p = Project::open(file).unwrap();
    let g = p.imported.design.boards[0].geometry.as_ref().unwrap();
    g.sheets[0].symbols.clone()
}

#[test]
fn moves_only_what_overlaps() {
    let dir = scratch("place_fields");
    let file = dir.join("place.kicad_sch");
    let before = schematic();
    std::fs::write(&file, &before).unwrap();

    let dry = place(&file, &PlaceOptions::default()).unwrap();
    assert!(dry.written.is_empty());
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        before,
        "a dry run writes nothing"
    );
    assert!(dry.before.draw_9_fields >= 3, "{:?}", dry.before);

    let opts = PlaceOptions {
        write: true,
        ..PlaceOptions::default()
    };
    let placed = place(&file, &opts).unwrap();
    assert_eq!(placed.written, std::slice::from_ref(&file));
    let moved: Vec<&str> = placed
        .plan
        .symbols
        .iter()
        .map(|s| s.reference.as_str())
        .collect();
    assert_eq!(moved, ["R1", "R2", "R3"]);
    assert_eq!(
        (placed.after.draw_9_fields, placed.after.draw_34_fields),
        (0, 0),
        "{:?}",
        placed.after
    );
    for s in &placed.plan.symbols {
        assert!(s.side.is_some() && s.left.is_empty(), "{s:?}");
        let order: Vec<&str> = s.moves.iter().map(|m| m.field.as_str()).collect();
        assert_eq!(order, ["Reference", "Value", "Power"]);
    }
    // R1's right side has the wire
    assert_ne!(
        placed.plan.symbols[0].side,
        Some(better_erc::place::Side::Right)
    );

    // Read back, each moved field is drawn upright where the plan put it, whatever the symbol's turn and mirror
    let after = std::fs::read_to_string(&file).unwrap();
    let drawn = symbols(&file);
    for s in &placed.plan.symbols {
        let sym = drawn.iter().find(|d| d.uuid == s.uuid).unwrap();
        for m in &s.moves {
            let f = sym.fields.iter().find(|f| f.name == m.field).unwrap();
            assert!(!f.text.vertical, "{} {}", s.reference, m.field);
            assert_eq!(
                (f.text.at, f.text.halign, f.text.bbox),
                (m.at, m.halign, m.rect),
                "{} {}",
                s.reference,
                m.field
            );
        }
        assert!(!symbol_text(&after, &s.uuid).contains("fields_autoplaced"));
    }
    let r4 = "a0000000-0000-4000-8000-000000000004";
    assert_eq!(
        symbol_text(&after, r4),
        symbol_text(&before, r4),
        "R4 overlaps nothing and stays"
    );
    // Outside the moved symbols' properties nothing changed: same line count, the library and wire untouched
    assert_eq!(after.lines().count(), before.lines().count());
    assert!(after.contains(LIB) && after.contains("(wire (pts (xy 54.61 40.64)"));

    // Placed once, nothing is left to move
    let again = place(&file, &opts).unwrap();
    assert!(
        again.plan.symbols.is_empty() && again.written.is_empty(),
        "{:?}",
        again.plan
    );
}

/// Three horizontal resistors stacked one grid step apart, wired on both sides, their fields piled up: all three
/// are packed and get Reference before and Value after the body, on the line above their wire, with Power hidden
fn packed() -> String {
    let mut s = String::from(
        "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n\
         (uuid \"a0000000-0000-4000-8000-000000000000\")\n(paper \"A4\")\n",
    );
    s += LIB;
    for (n, y) in [(5, 50.8), (6, 53.34), (7, 55.88)] {
        let reference = ["R5", "R6", "R7"][n as usize - 5];
        let fields = [
            ("Reference", reference, 101.6, y, 0),
            ("Value", "100R", 101.6, y, 0),
            ("Power", "63mW", 101.6, y + 0.5, 0),
        ];
        s += &resistor(n, (101.6, y, 90), "", &fields);
        for (a, b) in [(91.44, 96.52), (106.68, 111.76)] {
            s += &format!(
                "(wire (pts (xy {a} {y}) (xy {b} {y})) (stroke (width 0) (type default)) \
                 (uuid \"c{n}000000-0000-4000-8000-0000000000{}\"))\n",
                (a as u32) % 100
            );
        }
    }
    s += "(sheet_instances (path \"/\" (page \"1\")))\n)\n";
    s
}

#[test]
fn packed_parts_lie_along() {
    let dir = scratch("place_fields_packed");
    let file = dir.join("packed.kicad_sch");
    std::fs::write(&file, packed()).unwrap();
    let opts = PlaceOptions {
        write: true,
        ..PlaceOptions::default()
    };
    let placed = place(&file, &opts).unwrap();
    let side = |r: &str| {
        let s = placed
            .plan
            .symbols
            .iter()
            .find(|s| s.reference == r)
            .unwrap();
        (s.side, s.hidden.clone())
    };
    for r in ["R5", "R6", "R7"] {
        assert_eq!(side(r), (Some(Side::Ends), vec!["Power".to_owned()]), "{r}");
    }
    assert!(
        placed.after.draw_9_fields < placed.before.draw_9_fields,
        "{placed:?}"
    );

    let drawn = symbols(&file);
    for s in placed
        .plan
        .symbols
        .iter()
        .filter(|s| s.side == Some(Side::Ends))
    {
        let sym = drawn.iter().find(|d| d.uuid == s.uuid).unwrap();
        let body = sym.body.unwrap();
        let field = |name: &str| sym.fields.iter().find(|f| f.name == name).unwrap();
        assert!(field("Power").hidden, "{}", s.reference);
        let (r, v) = (field("Reference"), field("Value"));
        assert_eq!((r.hidden, v.hidden), (false, false));
        assert_eq!(
            (r.text.halign, v.text.halign),
            (HAlign::Right, HAlign::Left)
        );
        assert!(r.text.bbox.max.x <= body.min.x && v.text.bbox.min.x >= body.max.x);
        // On the line above the part's axis, clear of its own wire
        let axis = body.center().y;
        assert_eq!(r.text.at.y, v.text.at.y);
        assert!(
            r.text.bbox.max.y < axis && r.text.bbox.max.y > axis - 1_270_000,
            "{:?}",
            r.text.bbox
        );
        for m in &s.moves {
            let f = field(&m.field);
            assert_eq!(
                (f.text.at, f.text.vertical, f.text.bbox),
                (m.at, false, m.rect)
            );
        }
    }
}

const GND: &str = r##"(symbol "power:GND" (power) (pin_numbers (hide yes)) (pin_names (hide yes)) (exclude_from_sim no) (in_bom yes) (on_board yes)
(property "Reference" "#PWR" (at 0 0 0) (effects (font (size 1.27 1.27))))
(property "Value" "GND" (at 0 0 0) (effects (font (size 1.27 1.27))))
(symbol "GND_0_1" (polyline (pts (xy 0 0) (xy 0 -1.27) (xy 1.27 -1.27) (xy 0 -2.54) (xy -1.27 -1.27) (xy 0 -1.27)) (stroke (width 0) (type default)) (fill (type none))))
(symbol "GND_1_1" (pin power_in line (at 0 0 270) (length 0) (name "GND" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))))
"##;

/// Two vertical capacitors 10.16 mm apart, each with a ground symbol on its lower pin and a column of four fields
/// on its right whose last line, the dielectric, comes too close to the ground symbol
fn caps_over_ground() -> String {
    let mut s = String::from(
        "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n\
         (uuid \"a0000000-0000-4000-8000-000000000000\")\n(paper \"A4\")\n",
    );
    s += &LIB.replace("(lib_symbols\n", &format!("(lib_symbols\n{GND}"));
    for (n, x) in [(8, 50.8), (9, 60.96)] {
        let reference = ["C8", "C9"][n as usize - 8];
        let fields = [
            ("Reference", reference, x + 2.54, 46.99, 0),
            ("Value", "100nF", x + 2.54, 49.53, 0),
            ("Voltage", "50V", x + 2.54, 52.07, 0),
            ("Dielectric", "X5R", x + 2.54, 55.88, 0),
        ];
        s += &resistor(n, (x, 50.8, 0), "", &fields);
        s += &format!(
            "(symbol (lib_id \"power:GND\") (at {x} 55.88 0) (unit 1) (exclude_from_sim no) (in_bom yes) (on_board yes) \
             (dnp no) (uuid \"d{n}000000-0000-4000-8000-000000000000\") \
             (property \"Reference\" \"#PWR0{n}\" (at {x} 62.23 0) (effects (font (size 1.27 1.27)) (hide yes))) \
             (property \"Value\" \"GND\" (at {x} 60.96 0) (effects (font (size 1.27 1.27)))) \
             (pin \"1\" (uuid \"e{n}000000-0000-4000-8000-000000000000\")) \
             (instances (project \"place\" (path \"/a0000000-0000-4000-8000-000000000000\" (reference \"#PWR0{n}\") (unit 1)))))\n"
        );
    }
    s += "(sheet_instances (path \"/\" (page \"1\")))\n)\n";
    s
}

/// Vertical parts whose columns fit beside them are not packed: the column stays, and only the rating field that
/// touches the ground symbol below is hidden; the rest of the file is as it was
#[test]
fn rating_on_power_symbol_is_hidden() {
    let dir = scratch("place_fields_ground");
    let file = dir.join("ground.kicad_sch");
    let before = caps_over_ground();
    std::fs::write(&file, &before).unwrap();
    let opts = PlaceOptions {
        write: true,
        ..PlaceOptions::default()
    };
    let placed = place(&file, &opts).unwrap();
    let plans: Vec<_> = placed
        .plan
        .symbols
        .iter()
        .map(|s| {
            (
                s.reference.as_str(),
                s.side,
                s.moves.len(),
                s.hidden.clone(),
            )
        })
        .collect();
    let hidden = vec!["Dielectric".to_owned()];
    assert_eq!(
        plans,
        [
            ("C8", Some(Side::InPlace), 0, hidden.clone()),
            ("C9", Some(Side::InPlace), 0, hidden)
        ]
    );
    // Nothing overlaps; the column beside C8 fits, only within the part margin of C9
    assert_eq!(placed.after.draw_9_fields, 0, "{:?}", placed.plan);
    let left: Vec<(&str, &str, bool)> = placed
        .plan
        .symbols
        .iter()
        .flat_map(|s| s.left.iter())
        .map(|r| (r.field.as_str(), r.other.as_str(), r.close))
        .collect();
    assert_eq!(
        left,
        [
            ("C8 Value", "C9 body", true),
            ("C8 Value", "C9.1 pin", true)
        ]
    );
    let after = std::fs::read_to_string(&file).unwrap();
    let drawn = symbols(&file);
    for (s, x) in [
        ("a0000000-0000-4000-8000-000000000008", 50.8),
        ("a0000000-0000-4000-8000-000000000009", 60.96),
    ] {
        let sym = drawn.iter().find(|d| d.uuid == s).unwrap();
        for f in &sym.fields {
            assert!(!f.text.vertical, "{}", f.name);
            assert_eq!(f.hidden, f.name == "Dielectric", "{}", f.name);
        }
        // Only the dielectric changed, and the autoplaced flag went
        let was = symbol_text(&before, s).replace(" (fields_autoplaced yes)", "");
        let want = was.replace(
            &format!("\"X5R\" (at {} 55.88 0) (effects (font (size 1.27 1.27)) (justify left))", x + 2.54),
            &format!("\"X5R\" (at {} 55.88 0) (effects (font (size 1.27 1.27)) (justify left) (hide yes))", x + 2.54),
        );
        assert_eq!(symbol_text(&after, s), want);
    }
}

/// One vertical resistor whose fields overlap nothing but stand 10 mm to its right, as if they were another
/// part's
fn detached() -> String {
    let mut s = String::from(
        "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n\
         (uuid \"a0000000-0000-4000-8000-000000000000\")\n(paper \"A4\")\n",
    );
    s += LIB;
    let fields = [
        ("Reference", "R8", 111.76, 49.53, 0),
        ("Value", "10k", 111.76, 52.07, 0),
    ];
    s += &resistor(8, (101.6, 50.8, 0), "", &fields);
    s += "(sheet_instances (path \"/\" (page \"1\")))\n)\n";
    s
}

#[test]
fn far_fields_come_back_beside_their_body() {
    let dir = scratch("place_fields_detached");
    let file = dir.join("detached.kicad_sch");
    std::fs::write(&file, detached()).unwrap();
    let opts = PlaceOptions {
        write: true,
        ..PlaceOptions::default()
    };
    let placed = place(&file, &opts).unwrap();
    assert_eq!(placed.plan.symbols.len(), 1, "{:?}", placed.plan);
    let drawn = symbols(&file);
    let sym = &drawn[0];
    let body = sym.body.unwrap();
    for f in sym
        .fields
        .iter()
        .filter(|f| !f.hidden && !f.text.text.is_empty())
    {
        // Within a millimetre of the body, the column's own gap
        assert!(
            f.text.bbox.gap(body) <= 1_000_000,
            "{} {:?} {body:?}",
            f.name,
            f.text.bbox
        );
    }
    let again = place(&file, &opts).unwrap();
    assert!(again.plan.symbols.is_empty(), "{:?}", again.plan);
}
