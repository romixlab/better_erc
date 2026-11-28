//! `DRAW-9` and `DRAW-34` on sheets written here: what the clearance margins hold apart and what they leave alone

use better_erc::{CheckOptions, Config, Project};
use std::path::{Path, PathBuf};

const LIB: &str = r#"(lib_symbols
(symbol "Device:R" (pin_numbers (hide yes)) (pin_names (hide yes)) (exclude_from_sim no) (in_bom yes) (on_board yes)
(property "Reference" "R" (at 0 0 0) (effects (font (size 1.27 1.27))))
(property "Value" "R" (at 0 0 0) (effects (font (size 1.27 1.27))))
(symbol "R_0_1" (rectangle (start -1.016 -2.54) (end 1.016 2.54) (stroke (width 0.254) (type default)) (fill (type none))))
(symbol "R_1_1" (pin passive line (at 0 5.08 270) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27))))) (pin passive line (at 0 -5.08 90) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27))))))))
"#;

const ROOT_UUID: &str = "c0000000-0000-4000-8000-000000000000";

/// A fresh scratch directory per test
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("clearance_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A sheet file with the resistor library and these items
fn sheet(uuid: &str, items: &str) -> String {
    format!(
        "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n\
         (uuid \"{uuid}\")\n(paper \"A4\")\n{LIB}{items}\n(sheet_instances (path \"/\" (page \"1\")))\n)\n"
    )
}

/// A resistor `reference` at `x`, `y` turned by `angle`, with a shown Value field at `value` (x, y) when given
fn resistor(n: u32, reference: &str, at: (f64, f64, u32), value: Option<(f64, f64)>) -> String {
    let (x, y, r) = at;
    let field = |name: &str, text: &str, p: Option<(f64, f64)>| {
        let (fx, fy) = p.unwrap_or((x, y));
        let hide = if p.is_some() { "" } else { " (hide yes)" };
        format!(
            "(property \"{name}\" \"{text}\" (at {fx} {fy} 0) (effects (font (size 1.27 1.27)) (justify left){hide}))"
        )
    };
    format!(
        "(symbol (lib_id \"Device:R\") (at {x} {y} {r}) (unit 1) (exclude_from_sim no) (in_bom yes) (on_board yes) \
         (dnp no) (uuid \"c1000000-0000-4000-8000-0000000000{n:02}\")\n{}\n{}\n\
         (pin \"1\" (uuid \"c2{n:02}0000-0000-4000-8000-000000000001\")) (pin \"2\" (uuid \"c2{n:02}0000-0000-4000-8000-000000000002\"))\n\
         (instances (project \"t\" (path \"/{ROOT_UUID}\" (reference \"{reference}\") (unit 1)))))\n",
        field("Reference", reference, None),
        field("Value", "10k", value),
    )
}

/// A wire from `a` to `b`
fn wire(n: u32, a: (f64, f64), b: (f64, f64)) -> String {
    format!(
        "(wire (pts (xy {} {}) (xy {} {})) (stroke (width 0) (type default)) (uuid \"c3000000-0000-4000-8000-0000000000{n:02}\"))\n",
        a.0, a.1, b.0, b.1
    )
}

/// A text box with its corner at `x`, `y`, its text at the top left
fn text_box(n: u32, text: &str, at: (f64, f64), size: (f64, f64)) -> String {
    format!(
        "(text_box \"{text}\" (exclude_from_sim no) (at {} {} 0) (size {} {}) (stroke (width 0.1524) (type dot)) \
         (fill (type none)) (effects (font (size 1.27 1.27)) (justify left top)) (uuid \"c4000000-0000-4000-8000-0000000000{n:02}\"))\n",
        at.0, at.1, size.0, size.1
    )
}

/// The fingerprints of `DRAW-9` and `DRAW-34` on the root sheet `items`, with the child sheet `child.kicad_sch`
/// beside it, under `config`
fn findings(name: &str, items: &str, config: Config) -> Vec<String> {
    let dir = scratch(name);
    let root = dir.join("t.kicad_sch");
    std::fs::write(&root, sheet(ROOT_UUID, items)).unwrap();
    let child = sheet("c0000000-0000-4000-8000-0000000000ff", "");
    std::fs::write(dir.join("child.kicad_sch"), child).unwrap();
    let project = Project::open(&root).unwrap();
    let mut opts = CheckOptions::default();
    opts.run.only = vec!["DRAW-9".into(), "DRAW-34".into()];
    opts.run.config = config;
    let run = project.check(&opts).unwrap();
    let mut out: Vec<String> = run
        .report
        .findings
        .iter()
        .map(|f| f.fingerprint_names().to_owned())
        .collect();
    out.sort();
    out
}

/// A sheet box `A` at 60, 20, 20 by 12.7 mm, with an input pin `IN` on its left side at y 22.54
fn sheet_box() -> String {
    "(sheet (at 60 20) (size 20 12.7) (fields_autoplaced yes) (stroke (width 0.1524) (type solid)) \
     (fill (color 0 0 0 0.0000)) (uuid \"c5000000-0000-4000-8000-000000000001\")\n\
     (property \"Sheetname\" \"A\" (at 60 19.3 0) (effects (font (size 1.27 1.27)) (justify left bottom)))\n\
     (property \"Sheetfile\" \"child.kicad_sch\" (at 60 33.3 0) (effects (font (size 1.27 1.27)) (justify left top)))\n\
     (pin \"IN\" input (at 60 22.54 180) (effects (font (size 1.27 1.27)) (justify left)) \
     (uuid \"c5000000-0000-4000-8000-000000000002\"))\n\
     (instances (project \"t\" (path \"/c0000000-0000-4000-8000-000000000000\" (page \"2\")))))\n"
        .to_owned()
}

#[test]
fn a_wire_on_a_sheets_own_pin_is_not_near_its_box() {
    let on_pin = sheet_box() + &wire(1, (40.0, 22.54), (60.0, 22.54));
    assert_eq!(
        findings("sheet_pin", &on_pin, Config::default()),
        Vec::<String>::new()
    );
    // A wire running down 0.5 mm left of the box, tied to nothing of it
    let passing = on_pin + &wire(2, (59.5, 25.0), (59.5, 40.0));
    assert_eq!(
        findings("sheet_pass", &passing, Config::default()),
        ["DRAW-34:item sheet A|item wire of x 59.5 y 25|sheet /"]
    );
}

#[test]
fn a_text_box_border_over_a_wire_is_not_a_finding() {
    // The border crosses a wire at x 60 and runs 0.3 mm beside one at y 79.7; the text sits at the top left
    let wires = wire(1, (60.0, 40.0), (60.0, 90.0)) + &wire(2, (45.0, 79.7), (58.0, 79.7));
    let around = wires.clone() + &text_box(1, "Note", (50.0, 50.0), (20.0, 30.0));
    assert_eq!(
        findings("box_border", &around, Config::default()),
        Vec::<String>::new()
    );
    // The text itself across the wire, and 0.3 mm beside it
    let over = wires.clone() + &text_box(1, "A long note", (55.0, 50.0), (20.0, 10.0));
    assert_eq!(
        findings("box_text_over", &over, Config::default()),
        ["DRAW-9:item text box \"A long note\"|item wire of x 60 y 40|sheet /"]
    );
    let beside = wires + &text_box(1, "Note", (60.3, 60.0), (10.0, 10.0));
    assert_eq!(
        findings("box_text_beside", &beside, Config::default()),
        ["DRAW-34:item text box \"Note\"|item wire of x 60 y 40|sheet /"]
    );
}

/// Config in this spacing
fn spacing(word: &str) -> Config {
    let mut c = Config::default();
    c.set_text("drawing.clearance.spacing", word).unwrap();
    c
}

#[test]
fn tight_spacing_lets_parts_pack() {
    // Two horizontal resistors one grid step apart, bodies 0.508 mm apart; R2's Value just right of R1's pin end
    let packed = resistor(1, "R1", (50.0, 50.0, 90), None)
        + &resistor(2, "R2", (50.0, 52.54, 90), Some((56.2, 50.0)));
    assert_eq!(
        findings("packed_tight", &packed, Config::default()),
        Vec::<String>::new()
    );
    assert_eq!(
        findings("packed_tight_set", &packed, spacing("tight")),
        Vec::<String>::new()
    );
    assert_eq!(
        findings("packed_sparse", &packed, spacing("sparse")),
        [
            "DRAW-34:item R1 body|item R2 body|sheet /",
            "DRAW-34:item R1 body|item R2.1 pin|sheet /",
            "DRAW-34:item R1 body|item R2.2 pin|sheet /",
            "DRAW-34:item R1.1 pin|item R2 body|sheet /",
            "DRAW-34:item R1.2 pin|item R2 Value|sheet /",
            "DRAW-34:item R1.2 pin|item R2 body|sheet /",
        ]
    );
}

#[test]
fn tight_spacing_still_finds_overlaps_and_fields_on_wires() {
    // Bodies sharing area: a real overlap in both spacings
    let onto =
        resistor(1, "R1", (50.0, 50.0, 90), None) + &resistor(2, "R2", (50.0, 51.5, 90), None);
    for word in ["tight", "sparse"] {
        let found = findings(&format!("onto_{word}"), &onto, spacing(word));
        assert!(
            found.contains(&"DRAW-9:item R1 body|item R2 body|sheet /".to_owned()),
            "{word}: {found:?}"
        );
    }
    // A field just left of a wire it is not on: too close in both spacings
    let near_wire = resistor(1, "R1", (50.0, 50.0, 90), Some((60.0, 50.0)))
        + &wire(1, (61.0, 45.0), (61.0, 52.0));
    for word in ["tight", "sparse"] {
        let found = findings(&format!("near_wire_{word}"), &near_wire, spacing(word));
        assert!(
            found
                .iter()
                .any(|f| f.starts_with("DRAW-34:item R1 Value|item wire")),
            "{word}: {found:?}"
        );
    }
}

#[test]
fn spacing_takes_only_its_words() {
    let mut c = Config::default();
    assert_eq!(
        c.set_text("drawing.clearance.spacing", "loose"),
        Err("drawing.clearance.spacing takes tight or sparse, not loose".to_owned())
    );
    assert!(c.set("drawing.clearance.spacing", 2.0).is_err());
    assert!(c.set_text("drawing.clearance.packed", "0.2").is_ok());
}
