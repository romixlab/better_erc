//! The drawing geometry (`NET-26`) against KiCad's own output: `kicad-cli sch export svg` draws every text as strokes
//! with its string beside them, so each text's box here is compared with the box of the strokes KiCad plots.
//! Skipped with a message when kicad-cli is not installed.

use erc_model::geometry::{Dir, Rect, SheetGeometry};
use erc_model::{DirFiles, ImportOptions, Pt};
use std::path::{Path, PathBuf};
use std::process::Command;

/// A text drawn by KiCad: its string and the box of its strokes
#[derive(Debug)]
struct Ink {
    text: String,
    bbox: Rect,
}

fn numbers(s: &str) -> Vec<f64> {
    s.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter_map(|t| t.parse().ok())
        .collect()
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Every stroked text of an SVG export
fn inks(svg: &str) -> Vec<Ink> {
    let mut out = Vec::new();
    for part in svg.split("<g class=\"stroked-text\"><desc>").skip(1) {
        let Some((text, rest)) = part.split_once("</desc>") else {
            continue;
        };
        let Some(end) = rest.find("</g>") else {
            continue;
        };
        let mut points = Vec::new();
        for path in rest[..end].split("<path d=\"").skip(1) {
            let d = path.split('"').next().unwrap_or("");
            points.extend(numbers(d).chunks_exact(2).map(|c| Pt::from_mm(c[0], c[1])));
        }
        if let Some(bbox) = Rect::around(points) {
            out.push(Ink {
                text: unescape(text),
                bbox,
            });
        }
    }
    out
}

/// Every text box of a sheet as this crate computes it, with what it is
fn boxes(s: &SheetGeometry) -> Vec<(String, Rect, String)> {
    let mut out: Vec<(String, Rect, String)> = Vec::new();
    for sym in &s.symbols {
        for f in sym.fields.iter().filter(|f| !f.hidden) {
            out.push((
                f.text.text.clone(),
                f.text.bbox,
                format!("{} {}", sym.reference, f.name),
            ));
        }
        for p in &sym.pins {
            if let Some(b) = p.name_box {
                out.push((
                    p.name.clone(),
                    b,
                    format!("{} pin name {}", sym.reference, p.name),
                ));
            }
            if let Some(b) = p.number_box {
                out.push((
                    p.number.clone(),
                    b,
                    format!("{} pin {}", sym.reference, p.number),
                ));
            }
        }
    }
    for l in &s.labels {
        out.push((
            l.text.text.clone(),
            l.text.bbox,
            format!("{:?} label", l.kind),
        ));
    }
    for t in &s.texts {
        out.push((t.text.text.clone(), t.text.bbox, "text".into()));
    }
    for t in s
        .text_boxes
        .iter()
        .chain(s.tables.iter().flat_map(|t| &t.cells))
    {
        out.push((t.text.text.clone(), t.text.bbox, "text box".into()));
    }
    out
}

/// The largest difference of the two boxes' edges
fn apart(a: Rect, b: Rect) -> i64 {
    [
        (a.min.x - b.min.x).abs(),
        (a.max.x - b.max.x).abs(),
        (a.min.y - b.min.y).abs(),
        (a.max.y - b.max.y).abs(),
    ]
    .into_iter()
    .max()
    .unwrap()
}

/// The worst texts of a sheet: (edge difference in nm, the text, what it is, ours, KiCad's); only single-line ones
fn compare(dir: &Path, file: &str, out: &Path) -> Option<Vec<(i64, String, String, Rect, Rect)>> {
    let export = Command::new("kicad-cli")
        .args(["sch", "export", "svg", "-e", "-n", "-o"])
        .arg(out)
        .arg(dir.join(file))
        .output();
    let Ok(export) = export else {
        eprintln!("kicad-cli not installed: geometry not compared with KiCad");
        return None;
    };
    assert!(export.status.success(), "{export:?}");
    let name = file.trim_end_matches(".kicad_sch");
    let svg = std::fs::read_to_string(out.join(format!("{name}.svg"))).unwrap();
    let imported = erc_kicad::project::read(
        &DirFiles(dir.to_path_buf()),
        file,
        &ImportOptions::default(),
    )
    .unwrap();
    let geometry = imported.design.boards[0].geometry.as_ref().unwrap();
    let sheet = geometry.sheets.iter().find(|s| s.file == file).unwrap();
    let ours = boxes(sheet);
    let mut rows = Vec::new();
    for ink in inks(&svg) {
        let best = ours
            .iter()
            .filter(|(t, _, _)| *t == ink.text)
            .min_by_key(|(_, b, _)| apart(*b, ink.bbox));
        if let Some((t, b, what)) = best {
            rows.push((apart(*b, ink.bbox), t.clone(), what.clone(), *b, ink.bbox));
        }
    }
    rows.sort_by_key(|r| std::cmp::Reverse(r.0));
    Some(rows)
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Glyph side bearings and descenders are the rest of the difference between the advance box and the ink
const TOLERANCE: i64 = 1_000_000;

#[test]
fn text_boxes_match_kicad_output() {
    let out = std::env::temp_dir().join(format!("berc-geometry-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    let rows = compare(&fixtures(), "geometry.kicad_sch", &out);
    let _ = std::fs::remove_dir_all(&out);
    let Some(rows) = rows else { return };
    assert!(rows.len() >= 300, "only {} texts compared", rows.len());
    let off: Vec<_> = rows.iter().filter(|r| r.0 > TOLERANCE).collect();
    assert!(off.is_empty(), "{off:#?}");
}

fn fixture() -> SheetGeometry {
    let imported = erc_kicad::project::read(
        &DirFiles(fixtures()),
        "geometry.kicad_sch",
        &ImportOptions::default(),
    )
    .unwrap();
    let mut geometry = imported.design.boards[0].geometry.clone().unwrap();
    geometry.sheets.remove(0)
}

#[test]
fn sheet_items() {
    let s = fixture();
    assert_eq!(s.paper, "A3");
    assert_eq!(s.paper_size, (420_000_000, 297_000_000));
    assert_eq!(s.title_block.rev, "B");
    assert_eq!(s.wires.len(), 1);
    assert_eq!(s.junctions.len(), 1);
    assert_eq!(s.text_boxes.len(), 1);
    assert_eq!(s.graphics.len(), 2);
    assert_eq!(s.graphics[0].stroke.style, "dot");
    assert_eq!(s.text_boxes[0].stroke.colour.unwrap().r, 110);
}

#[test]
fn pins_face_out_of_the_turned_body() {
    let s = fixture();
    let by_ref = |r: &str| s.symbols.iter().find(|x| x.reference == r).unwrap();
    let u1 = by_ref("U1");
    assert_eq!(u1.body.unwrap().width(), 10_160_000);
    let out = |sym: &erc_model::geometry::DrawnSymbol, n: &str| {
        sym.pins.iter().find(|p| p.number == n).unwrap().outward
    };
    assert_eq!(out(u1, "1"), Dir::Left);
    assert_eq!(out(u1, "3"), Dir::Up);
    // Turned by 90 degrees counter-clockwise, pin 1 (left) points down
    let turned = s
        .symbols
        .iter()
        .find(|x| x.rotation == 90 && x.mirror.is_none())
        .unwrap();
    assert_eq!(out(turned, "1"), Dir::Down);
    assert_eq!(out(turned, "3"), Dir::Left);
    for sym in &s.symbols {
        for p in &sym.pins {
            let body = sym.body.unwrap();
            assert!(
                body.contains(p.root()),
                "{} pin {}",
                sym.reference,
                p.number
            );
            assert!(!body.contains(p.at), "{} pin {}", sym.reference, p.number);
        }
    }
}
