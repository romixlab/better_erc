//! The fields of one symbol of a KiCad symbol library (`.kicad_sym`), a derived symbol's (`extends`) on top of
//! its parent's. Where the library file is found: [`crate::libtable`].

use crate::sexpr;

/// The fields of symbol `name` in a `.kicad_sym` file, as name and value: a derived symbol's on top of its parent's.
/// `None` when the file has no such symbol or is no symbol library.
pub fn symbol_fields(text: &str, name: &str) -> Option<Vec<(String, String)>> {
    resolve_fields(name, |_| Some(text.to_string()))
}

/// The fields of symbol `name`, climbing `extends` chains across files: `read(n)` the text of a `.kicad_sym` file
/// that may define symbol `n` (a whole flat library, or one symbol directory's `n.kicad_sym`), `None` when there
/// is none to read. `None` overall when `name` itself can't be read or found; a parent that can't be is just the
/// end of the chain.
pub fn resolve_fields(
    name: &str,
    mut read: impl FnMut(&str) -> Option<String>,
) -> Option<Vec<(String, String)>> {
    let mut chain: Vec<Vec<(String, String)>> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut current = name.to_string();
    loop {
        if !seen.insert(current.clone()) || chain.len() >= 16 {
            break;
        }
        let found = read(&current).and_then(|text| {
            let parsed = sexpr::parse(&text).ok()?;
            let root = parsed
                .iter()
                .find(|r| r.head() == Some("kicad_symbol_lib"))?;
            let s = root
                .children("symbol")
                .find(|s| s.arg(0) == Some(current.as_str()))?;
            let props: Vec<(String, String)> = s
                .children("property")
                .filter_map(|p| Some((p.arg(0)?.to_string(), p.arg(1).unwrap_or("").to_string())))
                .collect();
            Some((props, s.value("extends").map(str::to_string)))
        });
        let Some((props, extends)) = found else {
            if chain.is_empty() {
                return None;
            }
            break;
        };
        chain.push(props);
        match extends {
            Some(parent) => current = parent,
            None => break,
        }
    }
    let mut fields: Vec<(String, String)> = Vec::new();
    for props in chain.into_iter().rev() {
        for (k, v) in props {
            match fields.iter_mut().find(|(fk, _)| *fk == k) {
                Some(f) => f.1 = v,
                None => fields.push((k, v)),
            }
        }
    }
    Some(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_symbol_fields() {
        let t = r#"(kicad_symbol_lib (version 20241209)
            (symbol "Base" (property "Reference" "U") (property "Footprint" "Pkg:SOT-23")
              (property "Footprint_Review" "fp1:aa") (symbol "Base_1_1"))
            (symbol "Derived" (extends "Base") (property "Value" "X") (property "Footprint_Review" "fp1:bb")))"#;
        let f = symbol_fields(t, "Derived").unwrap();
        let get = |k: &str| f.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        assert_eq!(get("Footprint_Review"), Some("fp1:bb"));
        assert_eq!(get("Footprint"), Some("Pkg:SOT-23"));
        assert_eq!(get("Value"), Some("X"));
        assert!(symbol_fields(t, "Missing").is_none());
    }

    #[test]
    fn resolve_fields_across_files() {
        // A symbol directory library: one `.kicad_sym` per symbol, a derived one's parent in another file
        let base = r#"(kicad_symbol_lib (version 20241209)
            (symbol "Base" (property "Footprint_Review" "fp1:aa")))"#;
        let derived = r#"(kicad_symbol_lib (version 20241209)
            (symbol "Derived" (extends "Base") (property "Value" "X")))"#;
        let read = |n: &str| match n {
            "Derived" => Some(derived.to_string()),
            "Base" => Some(base.to_string()),
            _ => None,
        };
        let f = resolve_fields("Derived", read).unwrap();
        let get = |k: &str| f.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        assert_eq!(get("Footprint_Review"), Some("fp1:aa"));
        assert_eq!(get("Value"), Some("X"));
        assert!(resolve_fields("Missing", read).is_none());
    }
}
