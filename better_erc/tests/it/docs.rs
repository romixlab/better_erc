//! The block documentation checks (`DRAW-48` to `DRAW-51`) through `Project::check`, on schematics written here:
//! each case that must fire next to one that must not

use better_erc::{CheckError, CheckOptions, Config, Project, RunOptions};
use std::path::{Path, PathBuf};

const FONT: &str = "(effects (font (size 1.27 1.27)))";
const HIDDEN: &str = "(effects (font (size 1.27 1.27)) (hide yes))";

/// A two-pin library symbol drawn as a box: `Device:R`, `Device:L`, `Regulator_Linear:LDO`
fn lib(lib_id: &str, reference: &str) -> String {
    let short = lib_id.split(':').nth(1).expect("a library and a name");
    format!(
        "(symbol \"{lib_id}\" (pin_numbers (hide yes)) (pin_names (hide yes)) (in_bom yes) (on_board yes)\n\
         (property \"Reference\" \"{reference}\" (at 0 0 0) {FONT})\n(property \"Value\" \"{short}\" (at 0 0 0) {FONT})\n\
         (symbol \"{short}_0_1\" (rectangle (start -1.016 -2.54) (end 1.016 2.54) (stroke (width 0.254) (type default)) (fill (type none))))\n\
         (symbol \"{short}_1_1\" (pin passive line (at 0 5.08 270) (length 2.54) (name \"~\" {FONT}) (number \"1\" {FONT})) \
         (pin passive line (at 0 -5.08 90) (length 2.54) (name \"~\" {FONT}) (number \"2\" {FONT}))))\n"
    )
}

/// A schematic being written: its items numbered for their UUIDs
struct Sheet {
    text: String,
    n: u32,
}

impl Sheet {
    fn new(libs: &[(&str, &str)]) -> Sheet {
        let mut text = String::from(
            "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n\
             (uuid \"e0000000-0000-4000-8000-000000000000\")\n(paper \"A4\")\n(lib_symbols\n",
        );
        for (id, reference) in libs {
            text += &lib(id, reference);
        }
        text += ")\n";
        Sheet { text, n: 0 }
    }

    fn uuid(&mut self) -> String {
        self.n += 1;
        format!("e0000000-0000-4000-8000-{:012}", self.n)
    }

    /// A part at `x`, `y` with extra (hidden) fields
    fn part(&mut self, lib_id: &str, reference: &str, at: (f64, f64), fields: &[(&str, &str)]) {
        let (x, y) = at;
        let (u, p1, p2) = (self.uuid(), self.uuid(), self.uuid());
        let mut s = format!(
            "(symbol (lib_id \"{lib_id}\") (at {x} {y} 0) (unit 1) (in_bom yes) (on_board yes) (dnp no) (uuid \"{u}\")\n\
             (property \"Reference\" \"{reference}\" (at {} {} 0) {FONT})\n(property \"Value\" \"x\" (at {} {} 0) {HIDDEN})\n",
            x + 2.54,
            y,
            x,
            y
        );
        for (name, value) in fields {
            s += &format!("(property \"{name}\" \"{value}\" (at {x} {y} 0) {HIDDEN})\n");
        }
        s += &format!(
            "(pin \"1\" (uuid \"{p1}\")) (pin \"2\" (uuid \"{p2}\"))\n(instances (project \"docs\" \
             (path \"/e0000000-0000-4000-8000-000000000000\" (reference \"{reference}\") (unit 1)))))\n"
        );
        self.text += &s;
    }

    /// A free text of character height `size`
    fn text(&mut self, text: &str, at: (f64, f64), size: f64) {
        let u = self.uuid();
        self.text += &format!(
            "(text \"{text}\" (at {} {} 0) (effects (font (size {size} {size})) (justify left)) (uuid \"{u}\"))\n",
            at.0, at.1
        );
    }

    /// A dashed grey note box
    fn note(&mut self, text: &str, at: (f64, f64)) {
        let u = self.uuid();
        self.text += &format!(
            "(text_box \"{text}\" (at {} {} 0) (size 50.8 10.16) (stroke (width 0.1524) (type dash) \
             (color 110 110 110 1)) (fill (type none)) (effects (font (size 1.27 1.27)) (justify left top)) (uuid \"{u}\"))\n",
            at.0, at.1
        );
    }

    /// A block's heading and cost line
    fn heading(&mut self, name: &str, at: (f64, f64)) {
        self.text(name, at, 3.0);
        self.text("$0.19 @25 · $0.12 @1k", (at.0, at.1 + 3.3), 1.5);
    }

    /// The schematic written as `docs.kicad_sch` in a fresh directory named `name`
    fn write(mut self, name: &str) -> PathBuf {
        self.text += "(sheet_instances (path \"/\" (page \"1\")))\n)\n";
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("docs.kicad_sch"), self.text).unwrap();
        dir
    }
}

/// The fingerprints of one check's findings on the schematic in `dir`, sorted, with these settings
fn findings(dir: &Path, check: &str, sets: &[(&str, &str)]) -> Result<Vec<String>, CheckError> {
    let mut config = Config::default();
    for (k, v) in sets {
        config.set_text(k, v).unwrap();
    }
    let opts = CheckOptions {
        run: RunOptions {
            only: vec![check.to_owned()],
            config,
            ..RunOptions::default()
        },
        ..CheckOptions::default()
    };
    let project = Project::open(&dir.join("docs.kicad_sch")).unwrap();
    let run = project.check(&opts)?;
    let mut f: Vec<String> = run
        .report
        .findings
        .iter()
        .map(|f| f.fingerprint.clone())
        .collect();
    f.sort();
    Ok(f)
}

#[test]
fn block_without_a_note() {
    let mut s = Sheet::new(&[("Device:R", "R")]);
    s.heading("Divider", (20.32, 25.4));
    s.part("Device:R", "R1", (25.4, 38.1), &[("Block", "Divider")]);
    s.note("Information:\\n--\\nGain = 0.32", (33.02, 35.56));
    s.heading("Filter", (120.65, 25.4));
    s.part("Device:R", "R2", (125.73, 38.1), &[("Block", "Filter")]);
    // Far from its block, but it names R3
    s.heading("Pull-up", (20.32, 101.6));
    s.part("Device:R", "R3", (25.4, 114.3), &[("Block", "Pull-up")]);
    s.note("R3 holds the line high at reset", (101.6, 165.1));
    let dir = s.write("docs_block_notes");
    assert_eq!(
        findings(&dir, "DRAW-48", &[]).unwrap(),
        ["DRAW-48:item block Filter|sheet /"]
    );
}

#[test]
fn placeholders_in_notes_and_fields() {
    let mut s = Sheet::new(&[("Device:R", "R")]);
    s.note(
        "Information:\\n--\\nUnregulated, x V .. y V\\nMax current:",
        (20.32, 25.4),
    );
    s.note(
        "Information:\\n--\\nUnregulated, 5 V .. 6 V\\nMax current: 30 mA",
        (20.32, 50.8),
    );
    s.part("Device:R", "R1", (101.6, 63.5), &[("Mpn1", "TBD")]);
    s.part(
        "Device:R",
        "R2",
        (127.0, 63.5),
        &[("Mpn1", "RC0402FR-0710KL")],
    );
    let dir = s.write("docs_placeholders");
    assert_eq!(
        findings(&dir, "DRAW-49", &[]).unwrap(),
        [
            "DRAW-49:item text box \"Information:\"|sheet /",
            "DRAW-49:part R1"
        ]
    );
}

#[test]
fn note_references_against_the_notes_file() {
    let mut s = Sheet::new(&[]);
    s.note("See N1, N2", (20.32, 25.4));
    s.note("See N3", (20.32, 50.8));
    let dir = s.write("docs_note_references");
    std::fs::create_dir_all(dir.join("doc")).unwrap();
    let notes = "# Notes\n\n## N1 · Power\n\nText.\n\n## N2 · Reset\n\nText.\n";
    std::fs::write(dir.join("doc/notes.md"), notes).unwrap();
    let set = [("drawing.notes", "doc/notes.md")];
    assert_eq!(
        findings(&dir, "DRAW-50", &set).unwrap(),
        ["DRAW-50:item text box \"See N3\"|sheet /"]
    );
    // Unset: the check does not run
    let project = Project::open(&dir.join("docs.kicad_sch")).unwrap();
    let opts = CheckOptions {
        run: RunOptions {
            only: vec!["DRAW-50".to_owned()],
            ..RunOptions::default()
        },
        ..CheckOptions::default()
    };
    let run = project.check(&opts).unwrap();
    let not_run: Vec<&str> = run.report.not_run().map(|n| n.check.as_str()).collect();
    assert_eq!(not_run, ["DRAW-50"]);
    // A file that is not there stops the run
    let missing = findings(&dir, "DRAW-50", &[("drawing.notes", "doc/gone.md")]);
    assert!(
        matches!(missing, Err(CheckError::Setting(_))),
        "{missing:?}"
    );
}

#[test]
fn notes_file_from_the_project_file() {
    let mut s = Sheet::new(&[]);
    s.note("See N1", (20.32, 25.4));
    s.note("See N2", (20.32, 50.8));
    let dir = s.write("docs_notes_project_file");
    std::fs::create_dir_all(dir.join("doc")).unwrap();
    std::fs::write(dir.join("doc/notes.md"), "## N1 · Power\n").unwrap();
    std::fs::write(dir.join("doc/other.md"), "## N2 · Reset\n").unwrap();
    std::fs::write(
        dir.join("docs.berc.toml"),
        "[drawing]\nnotes = \"doc/notes.md\"\n",
    )
    .unwrap();
    assert_eq!(
        findings(&dir, "DRAW-50", &[]).unwrap(),
        ["DRAW-50:item text box \"See N2\"|sheet /"]
    );
    // --set wins over the file
    assert_eq!(
        findings(&dir, "DRAW-50", &[("drawing.notes", "doc/other.md")]).unwrap(),
        ["DRAW-50:item text box \"See N1\"|sheet /"]
    );
}

#[test]
fn block_name_against_its_parts() {
    let libs = [
        ("Device:L", "L"),
        ("Regulator_Linear:LDO", "U"),
        ("Regulator_Switching:BUCK", "U"),
    ];
    let mut s = Sheet::new(&libs);
    s.heading("3V8 DC-DC 500mA", (20.32, 25.4));
    s.part(
        "Regulator_Linear:LDO",
        "U1",
        (25.4, 38.1),
        &[("Block", "3V8 DC-DC 500mA")],
    );
    s.heading("5V buck", (101.6, 25.4));
    s.part(
        "Regulator_Switching:BUCK",
        "U2",
        (106.68, 38.1),
        &[("Block", "5V buck")],
    );
    s.part("Device:L", "L1", (119.38, 38.1), &[("Block", "5V buck")]);
    s.heading("3V3 LDO", (20.32, 101.6));
    s.part(
        "Regulator_Linear:LDO",
        "U3",
        (25.4, 114.3),
        &[("Block", "3V3 LDO")],
    );
    let dir = s.write("docs_block_names");
    assert_eq!(
        findings(&dir, "DRAW-51", &[]).unwrap(),
        ["DRAW-51:item block 3V8 DC-DC 500mA|part U1|sheet /"]
    );
}
