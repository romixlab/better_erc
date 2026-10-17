//! `.kicad_pro` (JSON project settings) as sorted, line-oriented text: one setting per line as a dotted path
//! (`board.design_settings.defaults.zones.min_clearance = 0.3`), so opening a project in a newer KiCad - which
//! can silently add dozens of new-but-default settings on first save - shows as a small, readable change
//! instead of a wall of JSON noise. `uuid` fields are identity, not content, and dropped like everywhere else
//! in berc; the project's own `meta.filename` is the file's own name, not content, and dropped too.
//!
//! A leaf folds into a per-section summary line when it matches `pro_defaults`'s table of KiCad 10.0.6's own
//! defaults (transcribed from source - see that module for why not a generated fresh project), or, for
//! anything not in that table, when its value is JSON's own "nothing here" (`false`, `0`, `0.0`, `""`, `[]`,
//! `{}`, `null`) - a cheap approximation for the rest of the schema, not a real default check: an untabled
//! setting that is genuinely and meaningfully `0` (a net class `priority`, say) folds too. `--all` always
//! shows every line, folded or not.

use crate::common::{Options, q, sort_natural};
use crate::pro_defaults;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Write;

/// `Some(json)` when `text` looks like a `.kicad_pro`: a JSON object with a `meta.filename`. Checked by content,
/// not extension, so detection still works when git hands berc a temporary path without `.kicad_pro` on it.
pub fn detect(text: &str) -> Option<Value> {
    let v: Value = serde_json::from_str(text.trim()).ok()?;
    v.as_object()?
        .get("meta")?
        .as_object()?
        .get("filename")?
        .as_str()?;
    Some(v)
}

pub fn render(root: &Value, opts: Options) -> String {
    let Some(map) = root.as_object() else {
        return format!("# berc: .kicad_pro top level is not a JSON object, shown as is\n{root}");
    };
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    let defaults = pro_defaults::table();
    let mut out = String::new();
    for k in keys {
        let v = &map[k];
        // The file's own metadata: filename is the project's own name (noise), version is a real schema signal
        if k == "meta" {
            if let Some(ver) = v.as_object().and_then(|m| m.get("version")) {
                let _ = writeln!(out, "meta.version = {}", compact_scalar(ver));
            }
            continue;
        }
        let mut lines = Vec::new();
        let mut folded = 0usize;
        walk(k, v, opts, &defaults, &mut lines, &mut folded);
        sort_natural(&mut lines);
        for l in &lines {
            out.push_str(l);
            out.push('\n');
        }
        if !opts.show_defaults && folded > 0 {
            let s = if folded == 1 { "" } else { "s" };
            let _ = writeln!(
                out,
                "# {folded} setting{s} at KiCad's default or empty, folded: {k}.* (--all shows them)"
            );
        }
    }
    out
}

fn walk(
    path: &str,
    v: &Value,
    opts: Options,
    defaults: &HashMap<String, Value>,
    lines: &mut Vec<String>,
    folded: &mut usize,
) {
    // A path with a known KiCad default is judged only against that default, even when the actual value also
    // happens to look like JSON's own "nothing here" (reuse_designators: false is real content, not noise,
    // even though its default is true, not false)
    if !opts.show_defaults {
        if let Some(default) = defaults.get(path) {
            if pro_defaults::value_eq(v, default) {
                *folded += 1;
                return;
            }
        }
    }
    let known_not_default = !opts.show_defaults && defaults.contains_key(path);
    match v {
        Value::Object(map) if map.is_empty() => push_leaf(path, "{}", true, opts, lines, folded),
        Value::Object(map) => {
            for (k, v) in map {
                if k == "uuid" {
                    continue;
                }
                walk(&format!("{path}.{k}"), v, opts, defaults, lines, folded);
            }
        }
        Value::Array(items) if items.is_empty() => push_leaf(path, "[]", true, opts, lines, folded),
        Value::Array(items) => walk_array(path, items, opts, defaults, lines, folded),
        _ => push_leaf(
            path,
            &compact_scalar(v),
            !known_not_default && is_zero_scalar(v),
            opts,
            lines,
            folded,
        ),
    }
}

fn walk_array(
    path: &str,
    items: &[Value],
    opts: Options,
    defaults: &HashMap<String, Value>,
    lines: &mut Vec<String>,
    folded: &mut usize,
) {
    // Array of objects that each have a name: a map keyed by that name instead of by index (net classes, BOM
    // field order, sheet lists), so adding or removing one entry shows as one line, not a renumbering of the rest
    let named: Option<Vec<(&str, &serde_json::Map<String, Value>)>> = items
        .iter()
        .map(|v| {
            let obj = v.as_object()?;
            let name = obj.get("name")?.as_str()?;
            (!name.is_empty()).then_some((name, obj))
        })
        .collect();
    if let Some(named) = named {
        for (name, obj) in named {
            let child = format!("{path}.{name}");
            for (k, v) in obj {
                if k == "name" || k == "uuid" {
                    continue;
                }
                walk(&format!("{child}.{k}"), v, opts, defaults, lines, folded);
            }
        }
        return;
    }

    // `[uuid, name]` pairs (the project's top-level sheet list): the uuid is identity, keep only the names
    let name_pairs: Option<Vec<&str>> = items
        .iter()
        .map(|v| {
            let [a, b] = v.as_array()?.as_slice() else {
                return None;
            };
            let (a, b) = (a.as_str()?, b.as_str()?);
            Some(if is_uuid(a) { b } else { a })
        })
        .collect();
    if let Some(mut names) = name_pairs {
        names.sort();
        push_leaf(
            path,
            &format!("[{}]", names.join(", ")),
            false,
            opts,
            lines,
            folded,
        );
        return;
    }

    // Plain values in file order (track widths, the ERC conflict matrix row): position is the content here
    if items.iter().all(is_scalar) {
        let rendered = items
            .iter()
            .map(compact_scalar)
            .collect::<Vec<_>>()
            .join(", ");
        push_leaf(path, &format!("[{rendered}]"), false, opts, lines, folded);
        return;
    }

    // Nothing to key by: index it, same as everything else that has no name
    for (i, item) in items.iter().enumerate() {
        walk(&format!("{path}.{i}"), item, opts, defaults, lines, folded);
    }
}

fn is_scalar(v: &Value) -> bool {
    !matches!(v, Value::Array(_) | Value::Object(_))
}

fn is_uuid(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

fn is_zero_scalar(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !b,
        Value::Number(n) => n.as_f64() == Some(0.0),
        Value::String(s) => s.is_empty(),
        Value::Array(_) | Value::Object(_) => false,
    }
}

fn compact_scalar(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => q(s),
        Value::Array(_) | Value::Object(_) => v.to_string(),
    }
}

fn push_leaf(
    path: &str,
    rendered: &str,
    zero_like: bool,
    opts: Options,
    lines: &mut Vec<String>,
    folded: &mut usize,
) {
    if zero_like && !opts.show_defaults {
        *folded += 1;
        return;
    }
    lines.push(format!("{path} = {rendered}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_by_content_not_extension() {
        assert!(detect(r#"{"meta": {"filename": "x.kicad_pro", "version": 1}}"#).is_some());
        assert!(detect(r#"{"meta": {"version": 1}}"#).is_none());
        assert!(detect("(kicad_sch)").is_none());
    }

    #[test]
    fn folds_zero_like_leaves() {
        let v: Value = serde_json::from_str(
            r#"{"meta": {"filename": "x", "version": 1}, "board": {"a": false, "b": 1}}"#,
        )
        .unwrap();
        let out = render(&v, Options::default());
        assert_eq!(
            out,
            "board.b = 1\n# 1 setting at KiCad's default or empty, folded: board.* (--all shows them)\nmeta.version = 1\n"
        );
        let all = render(
            &v,
            Options {
                show_defaults: true,
                ..Options::default()
            },
        );
        assert_eq!(all, "board.a = false\nboard.b = 1\nmeta.version = 1\n");
    }

    #[test]
    fn keys_arrays_by_name() {
        let v: Value = serde_json::from_str(
            r#"{"meta": {"filename": "x", "version": 1}, "net_settings": {"classes": [{"name": "Default", "clearance": 0.1}, {"name": "D90", "clearance": 0.15}]}}"#,
        )
        .unwrap();
        let out = render(
            &v,
            Options {
                show_defaults: true,
                ..Options::default()
            },
        );
        assert_eq!(
            out,
            "meta.version = 1\nnet_settings.classes.D90.clearance = 0.15\nnet_settings.classes.Default.clearance = 0.1\n"
        );
    }

    #[test]
    fn sheets_keep_only_the_name() {
        let v: Value = serde_json::from_str(
            r#"{"meta": {"filename": "x", "version": 1}, "sheets": [["28257c90-4356-40df-bb03-6745be772667", "Root"]]}"#,
        )
        .unwrap();
        let out = render(
            &v,
            Options {
                show_defaults: true,
                ..Options::default()
            },
        );
        assert_eq!(out, "meta.version = 1\nsheets = [Root]\n");
    }

    #[test]
    fn folds_kicad_defaults_not_just_zero() {
        let v: Value = serde_json::from_str(
            r#"{"meta": {"filename": "x", "version": 1}, "schematic": {"reuse_designators": true}}"#,
        )
        .unwrap();
        let out = render(&v, Options::default());
        assert_eq!(
            out,
            "meta.version = 1\n# 1 setting at KiCad's default or empty, folded: schematic.* (--all shows them)\n"
        );

        // A project that disabled it is real content, not folded
        let v: Value = serde_json::from_str(
            r#"{"meta": {"filename": "x", "version": 1}, "schematic": {"reuse_designators": false}}"#,
        )
        .unwrap();
        let out = render(&v, Options::default());
        assert_eq!(
            out,
            "meta.version = 1\nschematic.reuse_designators = false\n"
        );
    }

    #[test]
    fn folds_the_whole_default_pin_map_as_one_setting() {
        let default_pin_map = pro_defaults::table()["erc.pin_map"].clone();
        let v: Value = serde_json::json!({
            "meta": {"filename": "x", "version": 1},
            "erc": {"pin_map": default_pin_map},
        });
        let out = render(&v, Options::default());
        assert_eq!(
            out,
            "# 1 setting at KiCad's default or empty, folded: erc.* (--all shows them)\nmeta.version = 1\n"
        );
    }
}
