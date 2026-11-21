//! `berc place-fields` on a schematic written here: moved fields end up clear of everything, drawn where the plan
//! put them, and symbols with nothing in the way stay byte for byte

use better_erc::Project;
use better_erc::model::geometry::DrawnSymbol;
use better_erc::place::{PlaceOptions, place};
use std::path::{Path, PathBuf};

const LIB: &str = r#"(lib_symbols
(symbol "Device:R" (pin_numbers (hide yes)) (pin_names (hide yes)) (exclude_from_sim no) (in_bom yes) (on_board yes)
(property "Reference" "R" (at 0 0 0) (effects (font (size 1.27 1.27))))
(property "Value" "R" (at 0 0 0) (effects (font (size 1.27 1.27))))
(symbol "R_0_1" (rectangle (start -1.016 -2.54) (end 1.016 2.54) (stroke (width 0.254) (type default)) (fill (type none))))
(symbol "R_1_1" (pin passive line (at 0 5.08 270) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27))))) (pin passive line (at 0 -5.08 90) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27))))))))
"#;

/// A resistor at `x`, `y` turned and mirrored, with its fields where `fields` puts them: name, value, x, y, angle
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
    s += &format!(
        "  (pin \"1\" (uuid \"b{n}000000-0000-4000-8000-000000000001\")) (pin \"2\" (uuid \"b{n}000000-0000-4000-8000-000000000002\"))\n  \
         (instances (project \"place\" (path \"/a0000000-0000-4000-8000-000000000000\" (reference \"R{n}\") (unit 1)))))\n"
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

fn scratch() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("place_fields");
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
    let dir = scratch();
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
