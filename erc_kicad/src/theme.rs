//! A KiCad colour theme (`colors/*.json`): the schematic colours that a sheet's items take when they set none.

use erc_model::geometry::{Colour, Theme};
use serde_json::Value;

/// Reads a theme file's text; the keys it leaves out stay `None`
pub fn read(text: &str) -> Result<Theme, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let schematic = root
        .get("schematic")
        .ok_or("not a KiCad colour theme: no `schematic` colours")?;
    let colour = |key: &str| {
        schematic
            .get(key)
            .and_then(Value::as_str)
            .and_then(Colour::parse)
    };
    let name = root.pointer("/meta/name").and_then(Value::as_str);
    Ok(Theme {
        name: name.unwrap_or("unnamed theme").to_string(),
        wire: colour("wire"),
        pin: colour("pin"),
        label_local: colour("label_local"),
        label_global: colour("label_global"),
        label_hier: colour("label_hier"),
        component_outline: colour("component_outline"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_schematic_colours() {
        let t = read(
            r#"{"meta": {"name": "T", "version": 5}, "schematic":
                {"wire": "rgb(74, 74, 74)", "pin": "rgba(150, 108, 0, 0.500)", "label_hier": "rgba(0, 0, 0, 0.000)"}}"#,
        )
        .unwrap();
        assert_eq!(t.name, "T");
        let (wire, pin) = (t.wire.unwrap(), t.pin.unwrap());
        assert_eq!((wire.r, wire.g, wire.b, wire.a), (74, 74, 74, 1.0));
        assert_eq!((pin.r, pin.g, pin.b, pin.a), (150, 108, 0, 0.5));
        assert_eq!(t.label_hier, None);
        assert_eq!(t.label_local, None);
    }

    #[test]
    fn refuses_another_file() {
        assert!(read("{}").is_err());
        assert!(read("not json").is_err());
    }
}
