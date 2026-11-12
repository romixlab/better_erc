//! KiCad symbol libraries as the footprint review reads them: a library table (`sym-lib-table`) to find the file
//! of a library nickname, and the fields of one symbol of a `.kicad_sym` file, a derived symbol's (`extends`) on
//! top of its parent's. A first slice: the full library tables with their nesting are a later task.

use crate::sexpr;

/// One library of a library table
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibEntry {
    /// The nickname symbols name it by (`Device` in `Device:R`)
    pub name: String,
    /// `KiCad`, `Legacy`, `Table`, ...
    pub kind: String,
    /// The file, with `${VAR}` references as written
    pub uri: String,
}

/// The libraries of a `sym-lib-table` (or `fp-lib-table`) file; `None` when it is not one
pub fn read_lib_table(text: &str) -> Option<Vec<LibEntry>> {
    let parsed = sexpr::parse(text).ok()?;
    let root = parsed
        .iter()
        .find(|r| matches!(r.head(), Some("sym_lib_table" | "fp_lib_table")))?;
    Some(
        root.children("lib")
            .filter_map(|l| {
                Some(LibEntry {
                    name: l.value("name")?.to_string(),
                    kind: l.value("type").unwrap_or("KiCad").to_string(),
                    uri: l.value("uri")?.to_string(),
                })
            })
            .collect(),
    )
}

/// `uri` with every `${VAR}` (and `$(VAR)`) replaced through `var`; `None` when one is not known
pub fn expand(uri: &str, var: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    let mut out = String::new();
    let mut rest = uri;
    while let Some(i) = rest.find('$') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let close = match after.chars().next() {
            Some('{') => '}',
            Some('(') => ')',
            _ => {
                out.push('$');
                rest = after;
                continue;
            }
        };
        let end = after.find(close)?;
        out.push_str(&var(&after[1..end])?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

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
    fn table_and_variables() {
        let t = r#"(sym_lib_table (version 7)
            (lib (name "Lib")(type "KiCad")(uri "${KIPRJMOD}/lib/Lib.kicad_sym")(options "")(descr ""))
            (lib (name "Device")(type "KiCad")(uri "${KICAD10_SYMBOL_DIR}/Device.kicad_sym")(options "")(descr "")))"#;
        let libs = read_lib_table(t).unwrap();
        assert_eq!(libs.len(), 2);
        assert_eq!(libs[0].name, "Lib");
        let var = |v: &str| (v == "KIPRJMOD").then(|| "/p".to_string());
        assert_eq!(
            expand(&libs[0].uri, &var).as_deref(),
            Some("/p/lib/Lib.kicad_sym")
        );
        assert_eq!(expand(&libs[1].uri, &var), None);
        assert_eq!(expand("$(KIPRJMOD)/a$b", &var).as_deref(), Some("/p/a$b"));
        assert!(read_lib_table("(kicad_sch)").is_none());
    }

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
