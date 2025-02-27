use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub const POSSIBLE_PNP_COLUMN_NAMES: [&str; 13] = [
    "RefDes",
    "Ref",
    "Designator",
    "Center-X",
    "PosX",
    "Center-X(mm)",
    "Center-Y",
    "PosY",
    "Center-Y(mm)",
    "Rotation",
    "Rot",
    "Layer",
    "Side",
];

/// Designator, X, Y, A, Side
pub const MINIMUM_PNP_COLUMNS_REQUIRED: usize = 5;

// TODO: Switch to BufReader or String
pub fn determine_separator(path: &Path) -> Option<u8> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_e) => {
            return None;
        }
    };
    let reader = BufReader::new(file);
    let mut counts: [(usize, u8); 3] = [(0, b','), (1, b'\t'), (2, b';')];
    for b in reader.bytes() {
        let Ok(b) = b else {
            break;
        };
        match b {
            b',' => counts[0].0 += 1,
            b'\t' => counts[1].0 += 1,
            b';' => counts[2].0 += 1,
            _ => {}
        }
    }
    counts.sort_by(|a, b| a.0.cmp(&b.0));
    // debug!("{counts:?}");
    Some(counts[2].1)
}

/// Find index of the first row that contains threshold or more names from possible_columns.
/// Empty rows are discarded and not counted.
pub fn find_header_row(
    threshold: usize,
    possible_columns: &[&str],
    path: &Path,
) -> Option<(usize, Vec<String>)> {
    let separator = determine_separator(path)?;
    let Ok(reader) = csv::ReaderBuilder::new()
        .delimiter(separator)
        .has_headers(false)
        .flexible(true)
        .from_path(path)
    else {
        return None;
    };

    let records = reader.into_records();
    for (idx, record) in records.enumerate() {
        let Ok(record) = record else {
            return None;
        };
        let column_names = record.iter().collect::<Vec<&str>>();
        let mut count = 0;
        for column_name in &column_names {
            if possible_columns.contains(&column_name) {
                count += 1;
            }
        }
        if count >= threshold {
            return Some((idx, column_names.iter().map(|s| s.to_string()).collect()));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_find_header_row_kicad() {
        let header_row = find_header_row(
            MINIMUM_PNP_COLUMNS_REQUIRED,
            &POSSIBLE_PNP_COLUMN_NAMES,
            Path::new("test_input/pnp_kicad.csv"),
        )
        .unwrap();
        assert_eq!(header_row.0, 0);
        assert_eq!(
            header_row.1,
            &["Ref", "Val", "Package", "PosX", "PosY", "Rot", "Side"]
        );
    }

    #[test]
    fn can_find_header_row_altium() {
        let header_row = find_header_row(
            MINIMUM_PNP_COLUMNS_REQUIRED,
            &POSSIBLE_PNP_COLUMN_NAMES,
            Path::new("test_input/pnp_altium_no_units.csv"),
        )
        .unwrap();
        assert_eq!(header_row.0, 9);
    }

    #[test]
    fn can_find_header_row_allegro() {
        let header_row = find_header_row(
            MINIMUM_PNP_COLUMNS_REQUIRED,
            &POSSIBLE_PNP_COLUMN_NAMES,
            &Path::new("test_input/pnp_allegro.csv"),
        )
        .unwrap();
        assert_eq!(header_row.0, 6);
    }
}
