use crate::Designator;
use crate::csv_util::{
    MINIMUM_PNP_COLUMNS_REQUIRED, POSSIBLE_PNP_COLUMN_NAMES, determine_separator, find_header_row,
};
use anyhow::{Error, Result};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug)]
pub struct ComponentPosition {
    pub x: f32,
    pub x_str: String,
    pub y: f32,
    pub y_str: String,
    pub rotation: f32,
    pub rotation_str: String,
    pub side: Side,
    pub value: Option<String>,
    pub package: Option<String>,
}

impl PartialEq for ComponentPosition {
    fn eq(&self, other: &Self) -> bool {
        self.x_str == other.x_str
            && self.y_str == other.y_str
            && self.rotation_str == other.rotation_str
            && self.side == other.side
            && self.value == other.value
            && self.package == other.package
    }
}

impl Eq for ComponentPosition {}

#[derive(Debug, Eq, PartialEq)]
pub enum Side {
    Top,
    Bottom,
}

pub fn load_component_positions(path: &Path) -> Result<HashMap<Designator, ComponentPosition>> {
    let (header_idx, header) = find_header_row(
        MINIMUM_PNP_COLUMNS_REQUIRED,
        &POSSIBLE_PNP_COLUMN_NAMES,
        path,
    )
    .unwrap();
    let separator = determine_separator(path).unwrap();
    let Ok(reader) = csv::ReaderBuilder::new()
        .delimiter(separator)
        .has_headers(false)
        .flexible(true)
        .from_path(path)
    else {
        return Err(Error::msg("Column header not found"));
    };

    let designator_idx = find_column_idx(&header, &["Ref", "RefDes", "Designator"])
        .ok_or(Error::msg("Required column not found (designator)"))?;
    let x_idx = find_column_idx(&header, &["Center-X", "Center-X(mm)", "PosX"])
        .ok_or(Error::msg("Required column not found (X position)"))?;
    let y_idx = find_column_idx(&header, &["Center-Y", "Center-Y(mm)", "PosY"])
        .ok_or(Error::msg("Required column not found (Y position)"))?;
    let a_idx = find_column_idx(&header, &["Rotation", "Rot"])
        .ok_or(Error::msg("Required column not found (Rotation)"))?;
    let side_idx = find_column_idx(&header, &["Side", "Layer"])
        .ok_or(Error::msg("Required column not found (Side)"))?;
    let value_idx = find_column_idx(&header, &["Value", "Val", "Comment"]);
    let package_idx = find_column_idx(&header, &["Package", "Footprint"]);

    let mut positions = HashMap::new();
    let records = reader.into_records().skip(header_idx + 1);
    for record in records {
        let Ok(record) = record else { continue };
        let values = record.iter().collect::<Vec<&str>>();
        let side = match values[side_idx] {
            "Top" | "TOP" | "top" | "TopLayer" => Side::Top,
            "Bottom" | "BOTTOM" | "bottom" | "BottomLayer" => Side::Bottom,
            _ => {
                return Err(Error::msg(format!(
                    "Unknown board side: {}",
                    values[side_idx].to_string()
                )));
            }
        };
        let x_str = values[x_idx].strip_suffix("mm").unwrap_or(values[x_idx]);
        let y_str = values[y_idx].strip_suffix("mm").unwrap_or(values[y_idx]);
        let position = ComponentPosition {
            x: x_str.parse::<f32>()?,
            x_str: x_str.to_string(),
            y: y_str.parse::<f32>()?,
            y_str: y_str.to_string(),
            rotation: values[a_idx].parse::<f32>()?,
            rotation_str: values[a_idx].to_string(),
            side,
            value: value_idx.map(|idx| values[idx].to_string()),
            package: package_idx.map(|idx| values[idx].to_string()),
        };
        let designator = Designator(values[designator_idx].to_string());
        positions.insert(designator, position);
    }
    Ok(positions)
}

fn find_column_idx(columns: &[String], synonyms: &[&str]) -> Option<usize> {
    columns
        .iter()
        .enumerate()
        .find(|(_idx, c)| synonyms.contains(&c.as_str()))
        .map(|(idx, _)| idx)
}

#[cfg(test)]
mod tests {
    use crate::pnp::{Designator, Side, load_component_positions};
    use std::path::Path;

    #[test]
    fn can_read_pnp_kicad() {
        let positions = load_component_positions(Path::new("test_input/pnp_kicad.csv")).unwrap();
        let pos_r1 = positions.get(&Designator("R1".to_string())).unwrap();
        assert_eq!(pos_r1.x_str, "56.600000");
        assert_eq!(pos_r1.side, Side::Bottom);
        let pos_c1 = positions.get(&Designator("C1".to_string())).unwrap();
        assert_eq!(pos_c1.y_str, "5.850000");
        assert_eq!(pos_c1.side, Side::Top);
    }

    #[test]
    fn can_read_pnp_altium() {
        let positions =
            load_component_positions(Path::new("test_input/pnp_altium_no_units.csv")).unwrap();
        let pos_j1 = positions.get(&Designator("J1".to_string())).unwrap();
        assert_eq!(pos_j1.x_str, "1.3943");
        assert_eq!(pos_j1.side, Side::Top);
        let pos_r1 = positions.get(&Designator("R1".to_string())).unwrap();
        assert_eq!(pos_r1.y_str, "15.1000");
        assert_eq!(pos_r1.side, Side::Bottom);
    }

    #[test]
    fn can_read_pnp_altium_with_units() {
        let positions =
            load_component_positions(Path::new("test_input/pnp_altium_with_units.csv")).unwrap();
        let pos_r1 = positions.get(&Designator("J1".to_string())).unwrap();
        assert_eq!(pos_r1.x_str, "1.3943");
        assert_eq!(pos_r1.side, Side::Top);
        let pos_c1 = positions.get(&Designator("R1".to_string())).unwrap();
        assert_eq!(pos_c1.y_str, "15.1000");
        assert_eq!(pos_c1.side, Side::Bottom);
    }

    #[test]
    fn can_read_pnp_allegro() {
        let positions = load_component_positions(Path::new("test_input/pnp_allegro.csv")).unwrap();
        let pos_r1 = positions.get(&Designator("R1".to_string())).unwrap();
        assert_eq!(pos_r1.x_str, "30.0200");
        assert_eq!(pos_r1.side, Side::Bottom);
        let pos_c1 = positions.get(&Designator("C1".to_string())).unwrap();
        assert_eq!(pos_c1.y_str, "-7.8000");
        assert_eq!(pos_c1.side, Side::Top);
    }
}
