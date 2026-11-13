//! The fields of one symbol of a KiCad symbol library (`.kicad_sym`), a derived symbol's (`extends`) on top of
//! its parent's, and [`set_property`], which writes one property of a symbol back into its file. Where the library
//! file is found: [`crate::libtable`].

use crate::edit::{Edits, Node, Tree, items, quote};
use crate::sexpr;
use std::fmt;

/// The fields of symbol `name` in a `.kicad_sym` file, as name and value: a derived symbol's on top of its parent's.
/// `None` when the file has no such symbol or is no symbol library.
pub fn symbol_fields(text: &str, name: &str) -> Option<Vec<(String, String)>> {
    resolve_fields(name, |_| Some(text.to_string()))
}

/// The names of the symbols a `.kicad_sym` file defines (its top-level symbols, not their units), in file order;
/// `None` when it is no symbol library
pub fn symbol_names(text: &str) -> Option<Vec<String>> {
    let parsed = sexpr::parse(text).ok()?;
    let root = parsed
        .iter()
        .find(|r| r.head() == Some("kicad_symbol_lib"))?;
    Some(
        root.children("symbol")
            .filter_map(|s| s.arg(0).map(str::to_string))
            .collect(),
    )
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

/// The oldest file version [`set_property`] writes: KiCad 8's (`20231120`, symbol libraries and schematics alike)
pub const OLDEST_WRITTEN: u32 = 20231120;

/// What [`set_property`] did
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyEdit {
    /// The file's new text; the same text when the property already had the value
    pub text: String,
    /// The property's value before, `None` when the symbol did not have it
    pub old: Option<String>,
    /// The text changed: the property was not there or had another value
    pub changed: bool,
}

/// Why [`set_property`] could not write the property
#[derive(Clone, Debug, PartialEq)]
pub enum EditError {
    Parse(sexpr::ParseError),
    /// Neither a symbol library (`kicad_symbol_lib`) nor a schematic (`kicad_sch`)
    NotSymbolFile,
    /// No symbol of that name: a library's own symbol, or a library id in a schematic's `lib_symbols`
    NoSymbol(String),
    /// Written by a KiCad older than 8: open and save it in KiCad 8 or newer first
    TooOld(u32),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditError::Parse(e) => e.fmt(f),
            EditError::NotSymbolFile => f.write_str("neither a symbol library nor a schematic"),
            EditError::NoSymbol(s) => write!(f, "no symbol {s}"),
            EditError::TooOld(v) => write!(
                f,
                "file version {v} is older than KiCad 8 ({OLDEST_WRITTEN}): open and save it in KiCad 8 or newer first"
            ),
        }
    }
}

impl std::error::Error for EditError {}

/// Sets property `name` of symbol `symbol` to `value` in `text`, a symbol library (`.kicad_sym`, a whole library or
/// one file of a `.kicad_symdir`) or a schematic, whose copy of library symbol `symbol` (a `Lib:Name` there) in
/// `lib_symbols` is edited. Only the value is written when the property is there; a new one goes after the
/// symbol's last property that is not KiCad's own `ki_...`, hidden, in the layout KiCad gives that file's version.
/// Every other byte stays as it was.
pub fn set_property(
    text: &str,
    symbol: &str,
    name: &str,
    value: &str,
) -> Result<PropertyEdit, EditError> {
    let tree = Tree::parse(text).map_err(EditError::Parse)?;
    let (sym, version) = find_symbol(&tree, symbol)?;
    let props: Vec<&Node> = tree.children(sym, "property").collect();
    let mut edits = Edits::new();
    let old = match props
        .iter()
        .find(|p| tree.arg(p, 0).as_deref() == Some(name))
    {
        Some(p) => {
            let old = tree.arg(p, 1).map(|v| v.into_owned());
            if old.as_deref() == Some(value) {
                return Ok(PropertyEdit {
                    text: text.to_string(),
                    old,
                    changed: false,
                });
            }
            match items(p).get(2) {
                Some(v) if !matches!(v.kind, crate::edit::Kind::List(_)) => {
                    edits.replace(v.span.clone(), quote(value))
                }
                // `(property "Name")` or `(property "Name" (at ...))`: the value goes after the name
                _ => edits.insert(items(p)[1].span.end, format!(" {}", quote(value))),
            }
            old
        }
        None => {
            insert_property(&tree, sym, &props, version, name, value, &mut edits);
            None
        }
    };
    let text = edits
        .apply(text)
        .expect("edits of one property don't overlap");
    Ok(PropertyEdit {
        text,
        old,
        changed: true,
    })
}

/// Symbol `symbol` of a parsed symbol library or schematic (its `lib_symbols` copy), with the file's version;
/// files older than [`OLDEST_WRITTEN`] refused
fn find_symbol<'t>(tree: &'t Tree, symbol: &str) -> Result<(&'t Node, u32), EditError> {
    let (root, symbols) = if let Some(lib) = tree.root("kicad_symbol_lib") {
        (lib, Some(lib))
    } else if let Some(sch) = tree.root("kicad_sch") {
        (sch, tree.child(sch, "lib_symbols"))
    } else {
        return Err(EditError::NotSymbolFile);
    };
    let version = tree
        .child(root, "version")
        .and_then(|v| tree.arg(v, 0))
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    if version < OLDEST_WRITTEN {
        return Err(EditError::TooOld(version));
    }
    let sym = symbols
        .and_then(|l| {
            tree.children(l, "symbol")
                .find(|s| tree.arg(s, 0).as_deref() == Some(symbol))
        })
        .ok_or_else(|| EditError::NoSymbol(symbol.to_string()))?;
    Ok((sym, version))
}

/// The names of the properties symbol `symbol` has in `text` (a symbol library or a schematic, as [`set_property`]
/// reads them), in file order: its own, not a parent's (`extends`)
pub fn property_names(text: &str, symbol: &str) -> Result<Vec<String>, EditError> {
    let tree = Tree::parse(text).map_err(EditError::Parse)?;
    let (sym, _) = find_symbol(&tree, symbol)?;
    Ok(tree
        .children(sym, "property")
        .filter_map(|p| tree.arg(p, 0).map(|n| n.into_owned()))
        .collect())
}

/// Removes property `name` of symbol `symbol` from `text` (a symbol library or a schematic, as [`set_property`]
/// reads them): the property and the line it stood on, every other byte kept. The same text, unchanged, when the
/// symbol has no such property.
pub fn remove_property(text: &str, symbol: &str, name: &str) -> Result<PropertyEdit, EditError> {
    let tree = Tree::parse(text).map_err(EditError::Parse)?;
    let (sym, _) = find_symbol(&tree, symbol)?;
    let Some(p) = tree
        .children(sym, "property")
        .find(|p| tree.arg(p, 0).as_deref() == Some(name))
    else {
        return Ok(PropertyEdit {
            text: text.to_string(),
            old: None,
            changed: false,
        });
    };
    let old = tree.arg(p, 1).map(|v| v.into_owned());
    // The blanks before it, and the line break before them when it stood on a line of its own
    let bytes = text.as_bytes();
    let mut start = p.span.start;
    while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
        start -= 1;
    }
    if start > 0 && bytes[start - 1] == b'\n' {
        start -= 1;
        if start > 0 && bytes[start - 1] == b'\r' {
            start -= 1;
        }
    }
    let mut edits = Edits::new();
    edits.replace(start..p.span.end, "");
    Ok(PropertyEdit {
        text: edits.apply(text).expect("one edit"),
        old,
        changed: true,
    })
}

/// A new hidden property `name` in symbol `sym`
fn insert_property(
    tree: &Tree,
    sym: &Node,
    props: &[&Node],
    version: u32,
    name: &str,
    value: &str,
    edits: &mut Edits,
) {
    let nl = tree.newline();
    let unit = tree.indent_unit();
    // KiCad 10 writes show_name, do_not_autoplace and hide on the property, KiCad 8 and 9 hide in its effects. The
    // file's own properties tell which, its version where it has none.
    let kicad10 = match props.first() {
        Some(_) => props.iter().any(|p| {
            ["show_name", "do_not_autoplace", "hide"]
                .iter()
                .any(|h| tree.child(p, h).is_some())
        }),
        None => version >= 20251024,
    };
    let sym_indent = tree.indent(sym).unwrap_or("");
    let lines: Vec<(usize, String)> = if kicad10 {
        vec![
            (0, format!("(property {} {}", quote(name), quote(value))),
            (1, "(at 0 0 0)".into()),
            (1, "(show_name no)".into()),
            (1, "(do_not_autoplace no)".into()),
            (1, "(hide yes)".into()),
            (1, "(effects".into()),
            (2, "(font".into()),
            (3, "(size 1.27 1.27)".into()),
            (2, ")".into()),
            (1, ")".into()),
            (0, ")".into()),
        ]
    } else {
        vec![
            (0, format!("(property {} {}", quote(name), quote(value))),
            (1, "(at 0 0 0)".into()),
            (1, "(effects".into()),
            (2, "(font".into()),
            (3, "(size 1.27 1.27)".into()),
            (2, ")".into()),
            (2, "(hide yes)".into()),
            (1, ")".into()),
            (0, ")".into()),
        ]
    };
    let block = |indent: &str| {
        lines
            .iter()
            .map(|(level, l)| format!("{indent}{}{l}", unit.repeat(*level)))
            .collect::<Vec<_>>()
            .join(nl)
    };
    let own = |p: &Node| !tree.arg(p, 0).is_some_and(|n| n.starts_with("ki_"));
    if let Some(after) = props.iter().rev().find(|p| own(p)) {
        // After the symbol's last own property, before KiCad's ki_keywords and ki_fp_filters
        let indent = tree.indent(after).unwrap_or(sym_indent);
        edits.insert(after.span.end, format!("{nl}{}", block(indent)));
    } else if let Some(first) = props.first() {
        // Only ki_ properties: before the first of them
        let indent = tree.indent(first).unwrap_or(sym_indent);
        edits.insert(
            first.span.start,
            format!("{}{nl}{indent}", block(indent).trim_start()),
        );
    } else {
        // No property at all: after the symbol's settings, before its units
        let it = items(sym);
        let units = it
            .iter()
            .position(|c| tree.head(c) == Some("symbol"))
            .unwrap_or(it.len());
        let after = &it[units.saturating_sub(1).max(1)];
        let indent = format!("{sym_indent}{unit}");
        edits.insert(after.span.end, format!("{nl}{}", block(&indent)));
    }
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
        assert_eq!(symbol_names(t).unwrap(), ["Base", "Derived"]);
        assert!(symbol_names("(kicad_sch)").is_none());
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

    /// A property removed with the line it stood on, every other byte kept; one not there changes nothing
    #[test]
    fn property_removed_with_its_line() {
        let t = "(kicad_symbol_lib (version 20241209)\n  (symbol \"R\"\n    (property \"Reference\" \"R\")\n    (property \"Footprint_Review2\" \"fp1:bb\")\n    (property \"Value\" \"R\") (property \"Inline\" \"x\")\n  )\n)\n";
        let e = remove_property(t, "R", "Footprint_Review2").unwrap();
        assert!(e.changed);
        assert_eq!(e.old.as_deref(), Some("fp1:bb"));
        assert_eq!(
            e.text,
            "(kicad_symbol_lib (version 20241209)\n  (symbol \"R\"\n    (property \"Reference\" \"R\")\n    (property \"Value\" \"R\") (property \"Inline\" \"x\")\n  )\n)\n"
        );
        let e = remove_property(&e.text, "R", "Inline").unwrap();
        assert!(
            e.text.contains("(property \"Value\" \"R\")\n  )"),
            "{}",
            e.text
        );
        assert_eq!(
            property_names(t, "R").unwrap(),
            ["Reference", "Footprint_Review2", "Value", "Inline"]
        );
        let same = remove_property(t, "R", "Missing").unwrap();
        assert!(!same.changed);
        assert_eq!(same.text, t);
        assert_eq!(
            remove_property(t, "Nope", "Value").unwrap_err(),
            EditError::NoSymbol("Nope".into())
        );
    }
}
