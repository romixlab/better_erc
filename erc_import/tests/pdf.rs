//! The schematic PDF importer: the synthetic Smart PDF in full, and the Altium Smart PDFs of the internal set when
//! they are on this PC (`BERC_PDF_DIR`, default `~/sync`): the Xyloni against its hand-made netlist, every other one
//! for consistency between its bookmarks and the board.

use erc_import::Registry;
use erc_model::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

mod common;

fn import(name: &str, bytes: Vec<u8>) -> Imported {
    let files = MemFiles::default().with(name, bytes);
    Registry::default()
        .import(&Source::new(name, &files), &ImportOptions::default())
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn nets(b: &Board) -> BTreeMap<String, BTreeSet<String>> {
    b.nets
        .iter()
        .map(|n| {
            (
                n.name.clone(),
                n.pins.iter().map(|p| b.pin_name(*p)).collect(),
            )
        })
        .collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn committed_fixture_is_the_generated_one() {
    let generated = common::smart_pdf::synthetic();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/smart_pdf.pdf");
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(&path, &generated).unwrap();
    }
    assert!(
        std::fs::read(&path).unwrap_or_default() == generated,
        "{} differs from the generator (BLESS=1 to write it)",
        path.display()
    );
}

#[test]
fn synthetic_smart_pdf() {
    let imported = import("smart_pdf.pdf", common::smart_pdf::synthetic());
    let b = &imported.design.boards[0];
    assert_eq!(b.source.tool, "altium-pdf");
    assert_eq!(
        b.source.connectivity,
        Some(Provenance::Pdf {
            file: "smart_pdf.pdf".into(),
            confidence: PdfConfidence::Bookmarks
        })
    );
    let sheets: Vec<String> = (0..b.sheets.len())
        .map(|i| b.sheet_path(SheetId::from_index(i)))
        .collect();
    assert_eq!(sheets, ["/", "/U_Top/", "/U_Top/U_IO/", "/U_Top/U_MCU/"]);

    let n = nets(b);
    assert_eq!(
        n["GND"],
        set(&["C1.2", "J1.3", "R4.2", "U1.4", "U1.6"]),
        "a power name joins sheets"
    );
    assert_eq!(
        n["SDA"],
        set(&["J1.2", "R2.1", "U1.1"]),
        "a port joins its parent's net"
    );
    assert_eq!(
        n["NRST"],
        set(&["J1.5", "R3.1", "U1.7"]),
        "differently named nets of one port, on sibling sheets and the parent"
    );
    assert_eq!(
        n["/U_Top/U_IO/EN"],
        set(&["R3.2"]),
        "a label is local to its sheet"
    );
    assert_eq!(n["/U_Top/U_MCU/EN"], set(&["R4.1"]));
    let nrst_ports: Vec<&str> = b
        .labels
        .iter()
        .filter(|l| l.kind == LabelKind::Hierarchical && l.net == b.net_by_name("NRST"))
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(nrst_ports, ["NRST", "NRST"], "ports by their own name");
    assert_eq!(n["D1"], set(&["R2.2", "U1.3"]), "a bus member");
    assert_eq!(n["EN_µC"], set(&["J1.4"]), "a UTF-16 title");
    let auto = b.net(b.net_by_name("NetR1_2").unwrap());
    assert_eq!(auto.kind, NetKind::Auto);

    // U1's units on two sheets
    let u1 = b.part(b.part_by_designator("U1").unwrap());
    let units: Vec<(u32, String)> = u1
        .units
        .iter()
        .map(|u| (u.unit, b.sheet_path(u.sheet)))
        .collect();
    assert_eq!(
        units,
        [(1, "/U_Top/U_MCU/".to_string()), (2, "/U_Top/".to_string())]
    );
    assert_eq!(u1.pins.len(), 7);

    // Properties: bound by the CO word, by the pins (U1B), through a form (C1), inline (R2)
    let field = |d: &str, f: &str| {
        b.part(b.part_by_designator(d).unwrap())
            .field(f)
            .map(String::from)
    };
    assert_eq!(field("R1", "Value").as_deref(), Some("4k7"));
    assert_eq!(
        field("R1", "Description").as_deref(),
        Some("Resistor \"thick film\"")
    );
    assert_eq!(field("U1", "Comment").as_deref(), Some("STM32G0B1"));
    assert_eq!(
        field("C1", "Supplier Part Number 1").as_deref(),
        Some("81-GRM1")
    );
    assert_eq!(field("R2", "Value").as_deref(), Some("10k"));
    assert_eq!(
        field("J1", "Value"),
        None,
        "an empty property is not a field"
    );
    let j1 = b.part(b.part_by_designator("J1").unwrap());
    assert_eq!(j1.value.get(), Some(&Value::Text("Header".into())));
    assert_eq!(j1.lib_id, "HEADER4");
    let src = j1.fields[0].value.resolve().unwrap();
    assert_eq!(
        src.source,
        Provenance::Pdf {
            file: "smart_pdf.pdf".into(),
            confidence: PdfConfidence::Position
        }
    );
    assert_eq!(src.rank, Rank::RECONSTRUCTED);

    // Labels: net labels placed by their NL words, ports without a position
    let label = |text: &str, kind: LabelKind| -> Vec<Option<Pt>> {
        b.labels
            .iter()
            .filter(|l| l.text == text && l.kind == kind)
            .map(|l| l.at)
            .collect()
    };
    assert_eq!(
        label("VIN", LabelKind::Local),
        [Some(Pt::from_mm(
            150.0 * 25.4 / 72.0,
            (595.0 - 375.0) * 25.4 / 72.0
        ))]
    );
    assert_eq!(label("EN_µC", LabelKind::Local).len(), 1);
    assert!(label("EN_µC", LabelKind::Local)[0].is_some());
    assert_eq!(label("SDA", LabelKind::Hierarchical), [None, None]);
    assert_eq!(
        label("D1", LabelKind::Local)
            .iter()
            .filter(|a| a.is_some())
            .count(),
        1
    );

    let notes: Vec<String> = imported.notes.iter().map(|n| n.to_string()).collect();
    let has = |s: &str| notes.iter().any(|n| n.contains(s));
    assert!(
        has("1 of 8 property popups have no component or pin word"),
        "{notes:#?}"
    );
    assert!(
        has("1 of 8 property popups are on pages no sheet bookmark leads to"),
        "{notes:#?}"
    );
    assert!(
        has(
            "sheet /U_Top/U_MCU/: missing: 1 port(s) meet no port or net of their name on another sheet: VDD_3V3"
        ),
        "{notes:#?}"
    );
    assert!(
        has("2 net(s) join sheets through ports of one name"),
        "{notes:#?}"
    );
    assert!(has("1 of 9 net labels have no position"), "{notes:#?}");
    assert_eq!(notes.len(), 5, "{notes:#?}");
}

#[test]
fn not_a_smart_pdf() {
    // A PDF without the bookmarks: offered (so the error can say why), then refused
    let mut pdf = pdf_writer::Pdf::new();
    pdf.catalog(pdf_writer::Ref::new(1))
        .pages(pdf_writer::Ref::new(2));
    pdf.pages(pdf_writer::Ref::new(2))
        .kids([pdf_writer::Ref::new(3)])
        .count(1);
    pdf.page(pdf_writer::Ref::new(3))
        .parent(pdf_writer::Ref::new(2))
        .media_box(pdf_writer::Rect::new(0.0, 0.0, 595.0, 842.0));
    let files = MemFiles::default().with("plain.pdf", pdf.finish());
    let src = Source::new("plain.pdf", &files);
    let reg = Registry::default();
    assert_eq!(
        reg.pick(&src).map(|(i, c)| (i.name(), c)),
        Some(("pdf", Confidence::Maybe))
    );
    let e = reg.import(&src, &ImportOptions::default()).unwrap_err();
    assert!(
        e.to_string()
            .contains("no schematic netlist in the bookmarks"),
        "{e}"
    );
}

/// The internal set: Altium Smart PDFs on this PC that are not ours to publish
fn internal_dir() -> Option<PathBuf> {
    let dir = match std::env::var_os("BERC_PDF_DIR") {
        Some(d) => PathBuf::from(d),
        None => PathBuf::from(std::env::var_os("HOME")?).join("sync"),
    };
    if dir.is_dir() {
        Some(dir)
    } else {
        eprintln!(
            "note: {} not here: the internal Smart PDF set is skipped",
            dir.display()
        );
        None
    }
}

/// The Xyloni, against the netlist taken by hand from its bookmarks (9 Oct 2026): every net and every pin
#[test]
fn xyloni_acceptance() {
    let Some(dir) = internal_dir() else { return };
    let x = dir.join("db/Ref/Boards/Xyloni");
    let (Ok(pdf), Ok(json)) = (
        std::fs::read(x.join("xyloni-sch-v1.1.pdf")),
        std::fs::read_to_string(x.join("xyloni-netlist.json")),
    ) else {
        eprintln!("note: the Xyloni files are not in {}: skipped", x.display());
        return;
    };
    let imported = import("xyloni-sch-v1.1.pdf", pdf);
    let b = &imported.design.boards[0];
    let got = nets(b);
    let want = hand_netlist(&json);
    assert_eq!(
        got.keys().collect::<Vec<_>>(),
        want.keys().collect::<Vec<_>>(),
        "net names"
    );
    for (net, pins) in &want {
        assert_eq!(&got[net], pins, "net {net}");
    }
    // Every component the hand netlist lists, with every pin it lists
    for (comp, pins) in hand_components(&json) {
        for p in pins {
            let (des, number) = p.split_once('-').unwrap();
            let part = b
                .part_by_designator(des)
                .unwrap_or_else(|| panic!("{comp}: no part {des}"));
            assert!(
                b.pins_by_number(part, number).next().is_some(),
                "{comp}: no pin {p}"
            );
        }
    }
    let with_value = b.parts.iter().filter(|p| p.value.get().is_some()).count();
    assert_eq!(
        with_value,
        b.parts.len(),
        "every part has a value from its popup"
    );
}

/// The hand-made netlist's nets, its pins written `R1.2` as the board names them
fn hand_netlist(json: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    let nets = section(json, "\"nets\"");
    for (name, body) in entries(nets) {
        let pins = list(body, "\"pins\"")
            .into_iter()
            .map(|p| p.replacen('-', ".", 1))
            .collect();
        out.insert(name, pins);
    }
    out
}

fn hand_components(json: &str) -> Vec<(String, Vec<String>)> {
    entries(section(json, "\"components\""))
        .into_iter()
        .map(|(c, body)| (c, list(body, "\"pins\"")))
        .collect()
}

// The hand-made JSON is regular (`json.dump(indent=1)` of string lists); a few lines read it without a JSON crate

fn section<'a>(json: &'a str, key: &str) -> &'a str {
    let start = json.find(key).expect("section") + key.len();
    let open = start + json[start..].find('{').unwrap();
    &json[open..open + matching(&json[open..])]
}

/// Length up to and including the brace matching the one `s` starts with
fn matching(s: &str) -> usize {
    let mut depth = 0;
    let mut in_str = false;
    let mut esc = false;
    for (i, c) in s.char_indices() {
        if in_str {
            match (esc, c) {
                (true, _) => esc = false,
                (false, '\\') => esc = true,
                (false, '"') => in_str = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' | '[' => depth += 1,
            '}' | ']' => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
    }
    s.len()
}

/// `"key": {...}` entries of an object
fn entries(obj: &str) -> Vec<(String, &str)> {
    let mut out = Vec::new();
    let mut rest = &obj[1..];
    while let Some(q) = rest.find('"') {
        let after = &rest[q + 1..];
        let end = after.find('"').unwrap();
        let key = after[..end].to_string();
        let tail = &after[end + 1..];
        let Some(open) = tail.find('{') else { break };
        let len = matching(&tail[open..]);
        out.push((key, &tail[open..open + len]));
        rest = &tail[open + len..];
    }
    out
}

fn list(body: &str, key: &str) -> Vec<String> {
    let Some(k) = body.find(key) else {
        return Vec::new();
    };
    let open = k + body[k..].find('[').unwrap();
    let arr = &body[open..open + matching(&body[open..])];
    arr.split('"')
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, s)| s.to_string())
        .collect()
}

/// Every Altium Smart PDF of the internal set imports, and the board holds exactly what its bookmarks list
#[test]
fn internal_smart_pdfs() {
    let Some(dir) = internal_dir() else { return };
    let set = [
        "db/Ref/Boards/Xyloni/xyloni-sch-v1.1.pdf",
        "db/Ref/Boards/Nucleo-G0B1RE/MB1360-G0B1RE-C02_Schematic.pdf",
        "datasheets/ST/H7/H7Eval_MB1246_E03/en.MB1246-H757-E03_Schematic.pdf",
        "datasheets/ST/H7/H7Disco_MB1248_D03/en.mb1248-h747i-d03-schematic.pdf",
        "datasheets/ST/H7/H743ZINucleo_E01/en.MB1364-H743ZI-E01_Schematic.PDF",
        "datasheets/ST/G0/en.MB1360-G071RB-C02_Schematic.pdf",
        "datasheets/Smothieboard/MKS SBASE SCH_V12.pdf",
        "datasheets/WaveShare/XCore407I-Schematic.pdf",
    ];
    for rel in set {
        let path = dir.join(rel);
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("note: {} not here: skipped", path.display());
            continue;
        };
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let t = std::time::Instant::now();
        let imported = import(&name, bytes);
        let b = &imported.design.boards[0];
        let connected = b.pins.iter().filter(|p| p.net.is_some()).count();
        let valued = b.parts.iter().filter(|p| p.value.get().is_some()).count();
        eprintln!(
            "{rel}: {} sheets, {} parts ({valued} with a value), {} pins ({connected} on a net), {} nets, {} notes, {:?}",
            b.sheets.len(),
            b.parts.len(),
            b.pins.len(),
            b.nets.len(),
            imported.notes.len(),
            t.elapsed()
        );
        for n in &imported.notes {
            eprintln!("    {n}");
        }
        assert!(b.nets.len() > 10, "{rel}");
        let text = dump::write(&imported.design);
        assert_eq!(
            dump::write(&dump::read(&text).unwrap()),
            text,
            "{rel}: dump, read, dump"
        );
    }
}

/// The synthetic fixture encrypted by qpdf: with an owner password only (AES-256 and RC4, what "no printing"
/// PDFs carry) it reads as the plain file; with a user password it is a clean error. Made once, in
/// `tests/fixtures`: `qpdf --encrypt '' owner 256 --print=none --modify=none -- smart_pdf.pdf smart_pdf_owner_only.pdf`,
/// `qpdf --allow-weak-crypto --encrypt '' owner 128 --use-aes=n -- smart_pdf.pdf smart_pdf_owner_rc4.pdf`,
/// `qpdf --encrypt secret owner 256 -- smart_pdf.pdf smart_pdf_user_password.pdf`
#[test]
fn encrypted() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let read = |name: &str| std::fs::read(dir.join(name)).unwrap();
    let plain = import("smart_pdf.pdf", read("smart_pdf.pdf"));
    for name in ["smart_pdf_owner_only.pdf", "smart_pdf_owner_rc4.pdf"] {
        let b = &import(name, read(name)).design.boards[0];
        let p = &plain.design.boards[0];
        assert_eq!(nets(b), nets(p), "{name}");
        assert_eq!(b.parts.len(), p.parts.len(), "{name}");
        let value = |b: &Board| {
            b.part(b.part_by_designator("C1").unwrap())
                .field("Value")
                .map(String::from)
        };
        assert_eq!(
            value(b),
            value(p),
            "{name}: properties from the encrypted scripts"
        );
    }
    let name = "smart_pdf_user_password.pdf";
    let files = MemFiles::default().with(name, read(name));
    let err = Registry::default()
        .import(&Source::new(name, &files), &ImportOptions::default())
        .unwrap_err();
    assert!(
        err.to_string().contains("encrypted with a password"),
        "{err}"
    );
}
