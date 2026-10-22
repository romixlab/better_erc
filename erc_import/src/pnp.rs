//! Pick-and-place (component position) CSV files as KiCad, Altium and Allegro write them, read into a board with
//! a layout only: one footprint per row with its reference, position, rotation, side, value and package, and the
//! other columns as its fields. It has no schematic and no pads; it is the placement source of the PnP compare
//! ([`erc_model::placecmp`]) and of the test-point checks.
//!
//! The header row is the first row naming at least five of the known columns (`Ref`/`RefDes`/`Designator`,
//! `PosX`/`Center-X`/`Center-X(mm)`, the Y twins, `Rot`/`Rotation`, `Side`/`Layer`); the separator is the one of
//! `,`, tab and `;` the file uses most. Lines before the header (Altium's and Allegro's file information) are not
//! kept. Positions are millimetres unless the header or the file information says mil or inch, and are kept as the
//! file writes them (most PnP files count y upwards, a board file downwards).

use crate::notes::Notes;
use crate::{file_name, text};
use erc_model::layout::{Footprint, FootprintAttrs, Nm, Pt, Side};
use erc_model::*;

/// The pick-and-place CSV importer
pub struct PnpImporter;

const DESIGNATOR: &[&str] = &["Ref", "RefDes", "Designator"];
const X: &[&str] = &["PosX", "Center-X", "Center-X(mm)", "Center-X(mil)"];
const Y: &[&str] = &["PosY", "Center-Y", "Center-Y(mm)", "Center-Y(mil)"];
const ROTATION: &[&str] = &["Rot", "Rotation"];
const SIDE: &[&str] = &["Side", "Layer"];
const VALUE: &[&str] = &["Val", "Value", "Comment"];
const PACKAGE: &[&str] = &["Package", "Footprint"];

/// Designator, X, Y, rotation, side
const MINIMUM_COLUMNS: usize = 5;

fn known(column: &str) -> bool {
    [DESIGNATOR, X, Y, ROTATION, SIDE]
        .iter()
        .any(|names| names.contains(&column))
}

/// The separator the file uses most
fn separator(text: &str) -> u8 {
    let count = |c: char| text.chars().filter(|x| *x == c).count();
    let mut counts = [(count(','), b','), (count('\t'), b'\t'), (count(';'), b';')];
    counts.sort_by_key(|c| c.0);
    counts[2].1
}

fn rows(text: &str) -> Vec<Vec<String>> {
    csv::ReaderBuilder::new()
        .delimiter(separator(text))
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes())
        .into_records()
        .map_while(Result::ok)
        .map(|r| r.iter().map(|c| c.trim().to_string()).collect())
        .collect()
}

/// The index of the header row
fn header(rows: &[Vec<String>]) -> Option<usize> {
    rows.iter()
        .position(|r| r.iter().filter(|c| known(c)).count() >= MINIMUM_COLUMNS)
}

impl Importer for PnpImporter {
    fn name(&self) -> &'static str {
        "pnp"
    }

    fn sniff(&self, src: &Source) -> Confidence {
        let lower = src.main.to_ascii_lowercase();
        if !(lower.ends_with(".csv") || lower.ends_with(".txt")) {
            return Confidence::No;
        }
        match src.files.read(&src.main) {
            Some(b) if header(&rows(&text::decode(&b))).is_some() => Confidence::Yes,
            Some(_) if lower.ends_with(".csv") => Confidence::Maybe,
            _ => Confidence::No,
        }
    }

    fn import(&self, src: &Source, _opts: &ImportOptions) -> Result<Imported, ImportError> {
        let file = src.main.as_str();
        let text = text::read(src, file)?;
        let rows = rows(&text);
        let invalid = |message: String| ImportError::Invalid {
            file: file.to_string(),
            message,
        };
        let h = header(&rows).ok_or_else(|| {
            invalid(format!(
                "no header row with {MINIMUM_COLUMNS} of the columns designator, X, Y, rotation and side"
            ))
        })?;
        let head = &rows[h];
        let col = |names: &[&str]| head.iter().position(|c| names.contains(&c.as_str()));
        let need = |names: &[&str], what: &str| {
            col(names).ok_or_else(|| invalid(format!("no {what} column")))
        };
        let (ci, cx, cy, cr, cs) = (
            need(DESIGNATOR, "designator")?,
            need(X, "X position")?,
            need(Y, "Y position")?,
            need(ROTATION, "rotation")?,
            need(SIDE, "side")?,
        );
        let (cv, cp) = (col(VALUE), col(PACKAGE));
        let mut notes = Notes::default();
        if h > 0 {
            notes.ignored(
                file,
                "",
                format!("{h} lines of file information before the header are not kept"),
            );
        }
        // The unit: a header that says it, else the file information, else millimetres
        let info = rows[..h].concat().join(" ").to_ascii_lowercase();
        let nm_per_unit: f64 = if head[cx].contains("(mil)") || info.contains(" mil") {
            25_400.0
        } else if info.contains(" inch") {
            25_400_000.0
        } else {
            1_000_000.0
        };
        let src_fact = Provenance::Placement { file: file.into() };
        let mut layout = Layout {
            file: file.to_string(),
            ..Layout::default()
        };
        for (i, row) in rows.iter().enumerate().skip(h + 1) {
            let line = format!("row {}", i + 1);
            if row.iter().all(String::is_empty) {
                continue;
            }
            let cell = |c: usize| row.get(c).map(String::as_str).unwrap_or("");
            let reference = cell(ci);
            if reference.is_empty() {
                notes.invalid(file, &line, "no designator; row not read");
                continue;
            }
            let at = format!("{line} {reference}");
            let coord = |c: usize| length(cell(c), nm_per_unit);
            let (Some(x), Some(y)) = (coord(cx), coord(cy)) else {
                notes.invalid(
                    file,
                    &at,
                    format!("position `{}`, `{}`; row not read", cell(cx), cell(cy)),
                );
                continue;
            };
            let Ok(rotation) = cell(cr).parse::<f64>() else {
                notes.invalid(file, &at, format!("rotation `{}`; row not read", cell(cr)));
                continue;
            };
            let side = match cell(cs).to_ascii_lowercase().as_str() {
                "top" | "toplayer" | "t" | "front" => Side::Front,
                "bottom" | "bottomlayer" | "b" | "back" => Side::Back,
                s => {
                    notes.invalid(file, &at, format!("board side `{s}`; row not read"));
                    continue;
                }
            };
            let mut fields = Vec::new();
            for (c, name) in head.iter().enumerate() {
                let used = [Some(ci), Some(cx), Some(cy), Some(cr), Some(cs), cv, cp];
                if name.is_empty() || used.contains(&Some(c)) || cell(c).is_empty() {
                    continue;
                }
                fields.push(Field::new(
                    name.clone(),
                    Fact::new(cell(c).to_string(), src_fact.clone()),
                ));
            }
            if row.len() > head.len() && row[head.len()..].iter().any(|c| !c.is_empty()) {
                notes.invalid(file, &at, "cells beyond the header's columns are not kept");
            }
            layout.footprints.push(Footprint {
                reference: reference.to_string(),
                part: None,
                schematic_path: None,
                lib_id: cp.map(cell).unwrap_or("").to_string(),
                value: cv.map(cell).unwrap_or("").to_string(),
                side,
                at: Pt { x, y },
                rotation,
                attrs: FootprintAttrs::default(),
                fields,
                pads: Vec::new(),
                graphics: Vec::new(),
                texts: Vec::new(),
                net_tie_pad_groups: Vec::new(),
                models: Vec::new(),
            });
        }
        let name = file_name(file);
        let mut board = Board {
            name: name.rsplit_once('.').map_or(name, |(s, _)| s).to_string(),
            source: SourceInfo {
                tool: "pnp".into(),
                version: None,
                files: vec![file.to_string()],
            },
            layout: Some(layout),
            ..Board::default()
        };
        board.finish();
        Ok(Imported {
            design: Design::single(board),
            notes: notes.into_vec(),
        })
    }
}

/// A coordinate in the file's unit (a `mm` or `mil` suffix allowed), in nanometres: exact for millimetres
fn length(text: &str, nm_per_unit: f64) -> Option<i64> {
    let (t, per) = if let Some(t) = text.strip_suffix("mm") {
        (t, 1_000_000.0)
    } else if let Some(t) = text.strip_suffix("mil") {
        (t, 25_400.0)
    } else {
        (text, nm_per_unit)
    };
    let t = t.trim();
    if per == 1_000_000.0
        && let Some(nm) = Nm::parse(t.strip_prefix('+').unwrap_or(t))
    {
        return Some(nm);
    }
    let v: f64 = t.parse().ok()?;
    v.is_finite()
        .then(|| (v * per).round())
        .filter(|n| n.abs() < 9.0e15)
        .map(|n| n as i64)
}
