use super::*;
use crate::finding::input_hash;

const FULL: &str = r#"
[nets."+3V3"]
voltage = 3.3
[nets.VBAT]
voltage = { min = 3.0, max = 4.2, nominal = 3.7 }
[nets.VIN]
voltage = { min = 9, max = 36 }

[derating]
ceramic_capacitor_v = 2.0
resistor_p = 3

[checks]
disabled = ["NAME-5"]
severity = { "I2C-3" = "warning" }

[[waiver]]
fingerprint = "I2C-1:net /SDA|part R5"
input_hash = "in1:0123456789abcdef"
who = "Jane Doe"
date = 2026-11-25
reason = "the pull-up sits on the mating board"
expires = "2027-06-01"

[[waiver]]
fingerprint = "BOM-14:part R9"
board = "b"
who = "Jane Doe"
date = 2026-11-20
reason = "DNP in every variant"
"#;

fn err(text: &str) -> String {
    ProjectFile::parse("b.berc.toml", text)
        .expect_err("refused")
        .to_string()
}

#[test]
fn reads_every_section() {
    let p = ProjectFile::parse("b.berc.toml", FULL).unwrap();
    assert_eq!(p.warnings, Vec::<String>::new());
    let c = &p.config;
    assert_eq!(c.file.as_deref(), Some("b.berc.toml"));
    assert_eq!(c.nets["+3V3"].voltage, Some(NetVoltage::fixed(3.3)));
    let vbat = c.nets["VBAT"].voltage.unwrap();
    assert_eq!((vbat.min, vbat.max, vbat.nominal), (3.0, 4.2, Some(3.7)));
    let vin = c.nets["VIN"].voltage.unwrap();
    assert_eq!((vin.nominal, vin.peak()), (None, 36.0));
    assert_eq!(c.derating.ceramic_capacitor_v, 2.0);
    assert_eq!(c.derating.resistor_p, 3.0);
    assert_eq!(
        c.derating.inductor_isat, 1.3,
        "unset keys keep their default"
    );
    assert!(c.disabled.contains("NAME-5"));
    assert_eq!(c.severity["I2C-3"], Severity::Warning);
    assert_eq!(p.waivers.len(), 2);
    let w = &p.waivers[0];
    assert_eq!(w.fingerprint, "I2C-1:net /SDA|part R5");
    assert_eq!(w.input_hash.as_deref(), Some("in1:0123456789abcdef"));
    assert_eq!(w.date, Time::parse("2026-11-25").unwrap());
    assert_eq!(w.expires, Time::parse("2027-06-01"));
    assert_eq!(p.waivers[1].board.as_deref(), Some("b"));
}

#[test]
fn check_settings_and_drawing_settings() {
    let p = ProjectFile::parse(
        "berc.toml",
        "[drawing.fields]\nhide_defaults = \"always\"\n",
    )
    .unwrap();
    assert_eq!(p.warnings, Vec::<String>::new());
    assert!(p.config.params.contains_key("drawing.fields.hide_defaults"));
    assert!(
        err("[drawing.fields]\nhide_defaults = \"sometimes\"\n")
            .starts_with("b.berc.toml: drawing.fields.hide_defaults: ")
    );
}

#[test]
fn unknown_keys_warn() {
    let p = ProjectFile::parse(
        "b.berc.toml",
        "[intent]\nvibration = true\n[derating]\nresistr_p = 2\n[nets.X]\ncurrent = 1\n[drawing]\nnope = 1\n",
    )
    .unwrap();
    assert_eq!(
        p.warnings,
        [
            "b.berc.toml: unknown key `derating.resistr_p`, ignored",
            "b.berc.toml: unknown key `drawing.nope`, ignored",
            "b.berc.toml: unknown key `intent`, ignored",
            "b.berc.toml: unknown key `nets.X.current`, ignored",
        ]
    );
}

#[test]
fn errors_name_file_and_key() {
    assert_eq!(
        err("[derating]\nresistor_p = \"2\"\n"),
        "b.berc.toml: derating.resistor_p: a number, not a string"
    );
    assert_eq!(
        err("[derating]\nresistor_p = 0.8\n"),
        "b.berc.toml: derating.resistor_p: a factor from 1 to 10, not 0.8"
    );
    assert_eq!(
        err("[nets.\"+5V\"]\nvoltage = \"5V\"\n"),
        "b.berc.toml: nets.\"+5V\".voltage: volts as a number, or { min, max, nominal }, not a string"
    );
    assert_eq!(
        err("[nets.VIN]\nvoltage = { min = 12, max = 9 }\n"),
        "b.berc.toml: nets.VIN.voltage: min 12 V is above max 9 V"
    );
    assert_eq!(
        err("[nets.VIN]\nvoltage = { min = 9 }\n"),
        "b.berc.toml: nets.VIN.voltage: a range needs both min and max"
    );
    assert_eq!(
        err("[checks.severity]\n\"I2C-3\" = \"loud\"\n"),
        "b.berc.toml: checks.severity.I2C-3: error, warning, suggestion or info, not \"loud\""
    );
    assert_eq!(
        err("[[waiver]]\nfingerprint = \"X-1:part R1\"\nwho = \"a\"\ndate = 2026-11-25\n"),
        "b.berc.toml: waiver[0].reason: missing: a waiver needs fingerprint, who, date and reason"
    );
    assert_eq!(
        err(
            "[[waiver]]\nfingerprint = \"X-1:part R1\"\nwho = \"a\"\nreason = \"r\"\ndate = \"25.11.2026\"\n"
        ),
        "b.berc.toml: waiver[0].date: a date like 2026-11-25, not \"25.11.2026\""
    );
    assert!(err("[nets\n").starts_with("b.berc.toml: not TOML: "));
}

#[test]
fn candidate_names_prefer_the_projects_own() {
    assert_eq!(
        candidate_names("board.kicad_pro"),
        ["board.berc.toml".to_owned(), "berc.toml".to_owned()]
    );
}

#[test]
fn waiver_entry_reads_back() {
    let w = Waiver {
        fingerprint: "RATE-1:part C4".into(),
        board: None,
        input_hash: Some(input_hash(&[("C4.Vworking", "3.3")])),
        who: "Jane \"JD\" Doe".into(),
        date: Time::parse("2026-11-25").unwrap(),
        reason: "derated by the\nsupplier".into(),
        expires: Time::parse("2027-01-31"),
    };
    let text = waiver::entry_text(&w);
    let p = ProjectFile::parse("b.berc.toml", &text).unwrap();
    assert_eq!(p.waivers, vec![w]);
}

#[test]
fn input_hash_is_order_free() {
    let a = input_hash(&[("a", "1"), ("b", "2")]);
    assert_eq!(a, input_hash(&[("b", "2"), ("a", "1")]));
    assert_ne!(a, input_hash(&[("a", "1"), ("b", "3")]));
    assert!(a.starts_with("in1:") && a.len() == 20, "{a}");
}

/// docs/project-file.md: its example reads without warnings, and its derating table has every key with its default
#[test]
fn the_docs_page_matches() {
    let page = include_str!("../../../docs/project-file.md");
    let example = page
        .split("## An example\n\n```toml\n")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .expect("an example block");
    let p = ProjectFile::parse("board.berc.toml", example).unwrap();
    assert_eq!(p.warnings, Vec::<String>::new());
    for k in derating::KEYS {
        let row = format!(
            "| `{}` | {} | {} |",
            k.name,
            show_factor(k.default),
            k.doc.trim()
        );
        assert!(
            page.contains(&row),
            "docs/project-file.md lacks the row {row}"
        );
    }
}

/// A factor as the docs write it: `2.0`, `1.25`
fn show_factor(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.1}")
    } else {
        v.to_string()
    }
}

#[test]
fn parts_lock_path() {
    let f = ProjectFile::parse("b.berc.toml", "[parts]\nlock = \"data/b.lock.toml\"\n").unwrap();
    assert_eq!(f.parts.lock_path(), "data/b.lock.toml");
    assert_eq!(ProjectFile::default().parts.lock_path(), "parts.lock.toml");
    assert_eq!(
        err("[parts]\nlock = \"../other/parts.lock.toml\"\n"),
        "b.berc.toml: parts.lock: a path inside the project directory, not \"../other/parts.lock.toml\""
    );
    assert_eq!(
        err("[parts]\nlock = \"/etc/parts.lock.toml\"\n"),
        "b.berc.toml: parts.lock: a path inside the project directory, not \"/etc/parts.lock.toml\""
    );
}

#[test]
fn report_units_are_read() {
    let f = ProjectFile::parse("b.berc.toml", "[report]\nunits = \"mil\"\n").expect("parses");
    assert_eq!(f.config.units, Some(Units::Mil));
    assert!(f.warnings.is_empty(), "{:?}", f.warnings);
    let none = ProjectFile::parse("b.berc.toml", "").expect("parses");
    assert_eq!(none.config.units, None);
}

#[test]
fn unknown_report_units_are_refused() {
    let e = err("[report]\nunits = \"inch\"\n");
    assert!(e.contains("report.units"), "{e}");
    assert!(e.contains("mm, mil or in"), "{e}");
}
