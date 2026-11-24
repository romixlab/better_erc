//! `berc place-fields` on a schematic written here: moved fields end up clear of everything, drawn where the plan
//! put them, and symbols with nothing in the way stay byte for byte

use better_erc::Config;
use better_erc::Project;
use better_erc::model::geometry::DrawnSymbol;
use better_erc::model::geometry::HAlign;
use better_erc::place::{PlaceOptions, Placed, Side, place, render};
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

/// Four resistors: three with their fields piled on each other at several turns, one with its fields clear and in
/// the style
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
    // In the style: a column 0.635 mm past the pins, on 0.25 mm steps, 2 mm apart
    let clear = vec![
        ("Reference", "R4", 52.5, 100.5, 0),
        ("Value", "1k", 52.5, 102.5, 0),
    ];
    s += &resistor(4, (50.8, 101.6, 0), "", &clear);
    // A wire past R1's right side, so its column goes elsewhere
    s += "(wire (pts (xy 54.61 40.64) (xy 54.61 60.96)) (stroke (width 0) (type default)) (uuid \"c0000000-0000-4000-8000-000000000001\"))\n";
    s += "(sheet_instances (path \"/\" (page \"1\")))\n)\n";
    s
}

/// The project file beside `sheet`, with the drawing sheet's default ratings as text variables: the values the
/// placer may hide
fn defaults_pro(sheet: &Path) {
    let pro = r#"{"text_variables": {"DEFAULT_R_PACKAGE": "0402", "DEFAULT_R_POWER": "63mW",
        "DEFAULT_C_PACKAGE": "0402", "DEFAULT_C_DIELECTRIC": "X5R", "DEFAULT_C_VOLTAGE": "50V"}}"#;
    std::fs::write(sheet.with_extension("kicad_pro"), pro).unwrap();
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
        "R4 overlaps nothing, follows the style and stays"
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
    defaults_pro(&file);
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
    // Ends in the style stay, and a write with nothing to move says so, not "Dry run"
    let again = place(&file, &opts).unwrap();
    assert!(again.plan.symbols.is_empty(), "{:?}", again.plan);
    let report = render(&again);
    assert!(
        report.ends_with("Nothing to move: no file changed.\n"),
        "{report}"
    );
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
    defaults_pro(&file);
    // Sparse spacing: the column beside C8 fits, only within the part margin of C9, and is left so
    let mut config = Config::default();
    config
        .set_text("drawing.clearance.spacing", "sparse")
        .unwrap();
    let sparse = PlaceOptions {
        config,
        ..PlaceOptions::default()
    };
    let left = |placed: &Placed| -> Vec<(String, String, bool)> {
        placed
            .plan
            .symbols
            .iter()
            .flat_map(|s| s.left.iter())
            .map(|r| (r.field.clone(), r.other.clone(), r.close))
            .collect()
    };
    let close = |field: &str, other: &str| (field.to_owned(), other.to_owned(), true);
    assert_eq!(
        left(&place(&file, &sparse).unwrap()),
        [close("C8 Value", "C9 body"), close("C8 Value", "C9.1 pin")]
    );
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
    // Nothing overlaps; in tight spacing the column beside C8 keeps the packed margin to C9
    assert_eq!(placed.after.draw_9_fields, 0, "{:?}", placed.plan);
    assert_eq!(left(&placed), []);
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

/// A sheet of vertical resistors at `x`, each with its fields' font size and `fields` (name, value, x, y), and
/// nothing else
type Part<'f> = (u32, f64, f64, Vec<(&'f str, &'f str, f64, f64)>);

fn columns_sheet(parts: &[Part]) -> String {
    let mut s = String::from(
        "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n\
         (uuid \"a0000000-0000-4000-8000-000000000000\")\n(paper \"A4\")\n",
    );
    s += LIB;
    for (n, x, size, fields) in parts {
        let fields: Vec<_> = fields
            .iter()
            .map(|(k, v, fx, fy)| (*k, *v, *fx, *fy, 0))
            .collect();
        s += &resistor(*n, (*x, 50.8, 0), "", &fields)
            .replace("(size 1.27 1.27)", &format!("(size {size} {size})"));
    }
    s += "(sheet_instances (path \"/\" (page \"1\")))\n)\n";
    s
}

/// The distances between the anchors of each moved column, by reference
fn pitches(placed: &Placed) -> Vec<(String, Vec<i64>)> {
    placed
        .plan
        .symbols
        .iter()
        .map(|s| {
            let ys: Vec<i64> = s.moves.iter().map(|m| m.at.y).collect();
            (
                s.reference.clone(),
                ys.windows(2).map(|w| w[1] - w[0]).collect(),
            )
        })
        .collect()
}

/// Two resistors with their fields piled up, one in 1.27 mm text and one in 2 mm text
fn two_sizes() -> String {
    let piled = |r: &'static str, x: f64| {
        vec![
            ("Reference", r, x, 50.8),
            ("Value", "10k", x, 50.8),
            ("Power", "63mW", x, 51.3),
        ]
    };
    columns_sheet(&[
        (1, 50.8, 1.27, piled("R1", 52.07)),
        (2, 101.6, 2.0, piled("R2", 102.87)),
    ])
}

#[test]
fn every_column_has_the_same_pitch() {
    let dir = scratch("place_fields_pitch");
    let file = dir.join("pitch.kicad_sch");
    std::fs::write(&file, two_sizes()).unwrap();
    let placed = place(&file, &PlaceOptions::default()).unwrap();
    let want = vec![2_000_000, 2_000_000];
    assert_eq!(
        pitches(&placed),
        [("R1".to_owned(), want.clone()), ("R2".to_owned(), want)],
        "{:?}",
        placed.plan
    );
    let height = |s: usize| placed.plan.symbols[s].moves[0].rect.height();
    assert_eq!((height(0), height(1)), (1_270_000, 2_000_000));

    let mut config = Config::default();
    config.set("drawing.fields.pitch", 2.54).unwrap();
    let opts = PlaceOptions {
        write: true,
        config,
        ..PlaceOptions::default()
    };
    let placed = place(&file, &opts).unwrap();
    let want = vec![2_540_000, 2_540_000];
    assert_eq!(
        pitches(&placed),
        [("R1".to_owned(), want.clone()), ("R2".to_owned(), want)]
    );
    // Read back at 2.54 mm; run again with the same setting, it is in style and stays
    let drawn = symbols(&file);
    let ys: Vec<i64> = drawn[0]
        .fields
        .iter()
        .filter(|f| !f.hidden && !f.text.text.is_empty())
        .map(|f| f.text.at.y)
        .collect();
    assert_eq!(ys[1] - ys[0], 2_540_000, "{ys:?}");
    let again = place(&file, &opts).unwrap();
    assert!(again.plan.symbols.is_empty(), "{:?}", again.plan);
    // With the default pitch the same columns stray from the style and are placed again
    let back = place(&file, &PlaceOptions::default()).unwrap();
    assert_eq!(back.plan.symbols.len(), 2, "{:?}", back.plan);
}

#[test]
fn a_bad_setting_is_an_error() {
    let dir = scratch("place_fields_bad_setting");
    let file = dir.join("bad.kicad_sch");
    std::fs::write(&file, two_sizes()).unwrap();
    let mut config = Config::default();
    config.set("drawing.fields.step", 0.0).unwrap();
    let opts = PlaceOptions {
        config,
        ..PlaceOptions::default()
    };
    let e = place(&file, &opts).unwrap_err().to_string();
    assert_eq!(
        e,
        "drawing.fields.step = 0: must be a number of at least 0.05 mm"
    );
}

/// Two resistors whose columns overlap nothing: R1's 2.54 mm apart (KiCad's own spacing), R2's in the style
fn odd_and_even() -> String {
    columns_sheet(&[
        (
            1,
            50.8,
            1.27,
            vec![
                ("Reference", "R1", 53.34, 49.53),
                ("Value", "10k", 53.34, 52.07),
            ],
        ),
        (
            2,
            101.6,
            1.27,
            vec![
                ("Reference", "R2", 103.5, 48.75),
                ("Value", "10k", 103.5, 50.75),
                ("Power", "63mW", 103.5, 52.75),
            ],
        ),
    ])
}

#[test]
fn odd_columns_are_placed_again() {
    let dir = scratch("place_fields_odd");
    let file = dir.join("odd.kicad_sch");
    let before = odd_and_even();
    std::fs::write(&file, &before).unwrap();
    let opts = PlaceOptions {
        write: true,
        ..PlaceOptions::default()
    };
    let placed = place(&file, &opts).unwrap();
    assert_eq!(placed.before.draw_9_fields, 0, "{:?}", placed.before);
    assert_eq!(
        pitches(&placed),
        [("R1".to_owned(), vec![2_000_000])],
        "{:?}",
        placed.plan
    );
    // R1 keeps its side, next to its pins
    let r1 = &placed.plan.symbols[0];
    assert_eq!(r1.side, Some(Side::Right));
    assert_eq!(r1.moves[0].at.x, 52_500_000);
    let after = std::fs::read_to_string(&file).unwrap();
    let r2 = "a0000000-0000-4000-8000-000000000002";
    assert_eq!(symbol_text(&after, r2), symbol_text(&before, r2));
    assert_eq!(after.lines().count(), before.lines().count());
    let again = place(&file, &opts).unwrap();
    assert!(again.plan.symbols.is_empty(), "{:?}", again.plan);
}

/// A column in style but slid off the middle of its part, as hiding its last lines leaves it, is centred again
#[test]
fn slid_columns_are_centred_again() {
    let file = scratch("place_fields_slid").join("slid.kicad_sch");
    let slid = columns_sheet(&[(
        1,
        50.8,
        1.27,
        vec![
            ("Reference", "R1", 52.5, 48.75),
            ("Value", "10k", 52.5, 50.75),
        ],
    )]);
    std::fs::write(&file, slid).unwrap();
    let opts = PlaceOptions {
        write: true,
        ..PlaceOptions::default()
    };
    let placed = place(&file, &opts).unwrap();
    let ys: Vec<(String, i64)> = placed.plan.symbols[0]
        .moves
        .iter()
        .map(|m| (m.field.clone(), m.at.y))
        .collect();
    assert_eq!(
        ys,
        [
            ("Reference".to_owned(), 49_750_000),
            ("Value".to_owned(), 51_750_000)
        ]
    );
    let again = place(&file, &opts).unwrap();
    assert!(again.plan.symbols.is_empty(), "{:?}", again.plan);
}

/// A horizontal resistor R1 wired on both sides with its fields piled up, a vertical one R2 below it whose fields
/// stand in R1's column below: R1 is not packed, so it takes a column above, centred on its body
fn horizontal_free_above() -> String {
    let mut s = String::from(
        "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n\
         (uuid \"a0000000-0000-4000-8000-000000000000\")\n(paper \"A4\")\n",
    );
    s += LIB;
    let piled = [
        ("Reference", "R1", 101.6, 50.8, 0),
        ("Value", "10k", 101.6, 50.8, 0),
    ];
    s += &resistor(1, (101.6, 50.8, 90), "", &piled);
    for (a, b) in [(91.44, 96.52), (106.68, 111.76)] {
        s += &format!(
            "(wire (pts (xy {a} 50.8) (xy {b} 50.8)) (stroke (width 0) (type default)) \
             (uuid \"c1000000-0000-4000-8000-0000000000{}\"))\n",
            (a as u32) % 100
        );
    }
    // Clear of everything, so settled when R1 is placed
    let below = [
        ("Reference", "R2", 100.5, 54.5, 0),
        ("Value", "1k", 100.5, 63.5, 0),
    ];
    s += &resistor(2, (99.06, 63.5, 0), "", &below);
    s += "(sheet_instances (path \"/\" (page \"1\")))\n)\n";
    s
}

#[test]
fn horizontal_part_takes_a_column_above() {
    let dir = scratch("place_fields_above");
    let file = dir.join("above.kicad_sch");
    std::fs::write(&file, horizontal_free_above()).unwrap();
    let opts = PlaceOptions {
        write: true,
        ..PlaceOptions::default()
    };
    let placed = place(&file, &opts).unwrap();
    let r1 = placed
        .plan
        .symbols
        .iter()
        .find(|s| s.reference == "R1")
        .unwrap();
    assert_eq!(r1.side, Some(Side::Above), "{:?}", placed.plan);
    assert!(r1.hidden.is_empty() && r1.left.is_empty(), "{r1:?}");
    let drawn = symbols(&file);
    let body = drawn
        .iter()
        .find(|d| d.reference == "R1")
        .unwrap()
        .outline();
    for m in &r1.moves {
        assert_eq!(m.halign, HAlign::Center);
        assert_eq!(m.at.x, body.center().x, "{} centred on the body", m.field);
        assert!(m.rect.max.y < body.min.y);
    }
}

const TP: &str = r#"(symbol "Connector:TestPoint" (pin_numbers (hide yes)) (pin_names (offset 0.762) (hide yes)) (exclude_from_sim no) (in_bom yes) (on_board yes)
(property "Reference" "TP" (at 0 0 0) (effects (font (size 1.27 1.27))))
(property "Value" "TestPoint" (at 0 0 0) (effects (font (size 1.27 1.27))))
(symbol "TestPoint_0_1" (circle (center 0 3.302) (radius 0.762) (stroke (width 0) (type default)) (fill (type none))))
(symbol "TestPoint_1_1" (pin passive line (at 0 0 90) (length 2.54) (name "1" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))))
"#;

/// A test point whose Reference and Value stand above it, clear of everything but 2.54 mm apart and far from its
/// circle
fn test_point() -> String {
    let mut s = String::from(
        "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n\
         (uuid \"a0000000-0000-4000-8000-000000000000\")\n(paper \"A4\")\n",
    );
    s += &LIB.replace("(lib_symbols\n", &format!("(lib_symbols\n{TP}"));
    s += "(symbol (lib_id \"Connector:TestPoint\") (at 101.6 50.8 0) (unit 1) (exclude_from_sim no) (in_bom yes) \
          (on_board yes) (dnp no) (uuid \"a0000000-0000-4000-8000-000000000001\")\n\
          (property \"Reference\" \"TP1\" (at 101.6 41.91 0) (effects (font (size 1.27 1.27))))\n\
          (property \"Value\" \"TestPoint\" (at 101.6 44.45 0) (effects (font (size 1.27 1.27))))\n\
          (pin \"1\" (uuid \"b1000000-0000-4000-8000-000000000001\"))\n\
          (instances (project \"place\" (path \"/a0000000-0000-4000-8000-000000000000\" (reference \"TP1\") (unit 1)))))\n";
    s += "(sheet_instances (path \"/\" (page \"1\")))\n)\n";
    s
}

#[test]
fn test_point_fields_go_right() {
    let dir = scratch("place_fields_test_point");
    let file = dir.join("tp.kicad_sch");
    std::fs::write(&file, test_point()).unwrap();
    let opts = PlaceOptions {
        write: true,
        ..PlaceOptions::default()
    };
    let placed = place(&file, &opts).unwrap();
    let plans: Vec<_> = placed
        .plan
        .symbols
        .iter()
        .map(|s| (s.reference.as_str(), s.side))
        .collect();
    assert_eq!(plans, [("TP1", Some(Side::Right))], "{:?}", placed.plan);
    assert_eq!(pitches(&placed), [("TP1".to_owned(), vec![2_000_000])]);
    let again = place(&file, &opts).unwrap();
    assert!(again.plan.symbols.is_empty(), "{:?}", again.plan);
}

/// The plans of `file`'s symbols after a run with `config`: reference and the fields hidden
fn hidden_by_place(name: &str, text: &str, config: Config) -> Vec<(String, Vec<String>)> {
    let file = scratch(name).join("defaults.kicad_sch");
    std::fs::write(&file, text).unwrap();
    defaults_pro(&file);
    let opts = PlaceOptions {
        config,
        ..PlaceOptions::default()
    };
    let placed = place(&file, &opts).unwrap();
    placed
        .plan
        .symbols
        .iter()
        .map(|s| (s.reference.clone(), s.hidden.clone()))
        .collect()
}

/// A rating off its default is a choice: packed resistors rated 250mW keep their Power shown, a C0G capacitor its
/// Dielectric, a 16V one its Voltage, and a project without the DEFAULT_* variables hides nothing
#[test]
fn only_default_ratings_are_hidden() {
    let shown = |plans: &[(String, Vec<String>)], field: &str| {
        plans
            .iter()
            .all(|(_, hidden)| !hidden.iter().any(|h| h == field))
    };
    let power = hidden_by_place(
        "place_fields_power",
        &packed().replace("\"63mW\"", "\"250mW\""),
        Config::default(),
    );
    assert_eq!(power.len(), 3, "{power:?}");
    assert!(shown(&power, "Power"), "{power:?}");

    let c0g = hidden_by_place(
        "place_fields_c0g",
        &caps_over_ground().replace("\"X5R\"", "\"C0G\""),
        Config::default(),
    );
    assert!(shown(&c0g, "Dielectric"), "{c0g:?}");

    // The voltage as the line on the ground symbol: the 50V default goes, a 16V rating stays
    let volts_last = caps_over_ground()
        .replace("\"Voltage\" \"50V\"", "\"Dielectric\" \"C0G\"")
        .replace("\"Dielectric\" \"X5R\"", "\"Voltage\" \"50V\"");
    let at_default = hidden_by_place("place_fields_50v", &volts_last, Config::default());
    let voltage = vec!["Voltage".to_owned()];
    assert_eq!(
        at_default,
        [
            ("C8".to_owned(), voltage.clone()),
            ("C9".to_owned(), voltage)
        ]
    );
    let other = hidden_by_place(
        "place_fields_16v",
        &volts_last.replace("\"50V\"", "\"16V\""),
        Config::default(),
    );
    assert!(shown(&other, "Voltage"), "{other:?}");

    // A project without the variables has no defaults: nothing is hidden
    let file = scratch("place_fields_no_defaults").join("bare.kicad_sch");
    std::fs::write(&file, caps_over_ground()).unwrap();
    let bare = place(&file, &PlaceOptions::default()).unwrap();
    assert!(
        bare.plan.symbols.iter().all(|s| s.hidden.is_empty()),
        "{:?}",
        bare.plan
    );
}

/// `drawing.fields.hide_defaults = always`: a rating at its default goes even on a part whose fields are clear of
/// everything, and the placer works on the fields left; a rating off its default stays
#[test]
fn always_hides_every_default() {
    let value = "\"1k\" (at 52.5 102.5 0) (effects (font (size 1.27 1.27)) (justify left)))\n";
    let with_power = |power: &str| {
        schematic().replace(
            value,
            &format!(
                "{value}  (property \"Power\" \"{power}\" (at 52.5 104.5 0) (effects (font (size 1.27 1.27)) (justify left)))\n"
            ),
        )
    };
    let mut config = Config::default();
    config
        .set_text("drawing.fields.hide_defaults", "always")
        .unwrap();
    let hides = |plans: &[(String, Vec<String>)], r: &str| {
        plans
            .iter()
            .find(|(reference, _)| reference == r)
            .is_some_and(|(_, hidden)| hidden.iter().any(|h| h == "Power"))
    };
    let at_default = hidden_by_place("place_fields_always", &with_power("63mW"), config.clone());
    for r in ["R1", "R2", "R3", "R4"] {
        assert!(hides(&at_default, r), "{r} {at_default:?}");
    }
    let other = hidden_by_place("place_fields_always_250", &with_power("250mW"), config);
    assert!(!hides(&other, "R4"), "{other:?}");
    // In the way only, the default: R4's clear column keeps its Power
    let in_the_way = hidden_by_place(
        "place_fields_in_the_way",
        &with_power("63mW"),
        Config::default(),
    );
    assert!(!hides(&in_the_way, "R4"), "{in_the_way:?}");
}
