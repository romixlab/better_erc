//! Moving, hiding, writing and removing the fields of placed symbols in a schematic (`TOOL-13`): a field's position, angle and
//! justification written new, or the field hidden, every other byte kept as it was ([`crate::edit`]).

use crate::edit::{Edits, Kind, Node, Tree, items, quote};
use crate::sch::geom::upright_field;
use crate::sexpr::ParseError;
use erc_model::Pt;
use erc_model::geometry::HAlign;
use erc_model::layout::Nm;
use std::fmt;

/// What happens to one field of a placed symbol
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldPlace {
    /// The UUID of the placed symbol
    pub symbol: String,
    /// The field's name (`Reference`, `Value`, `Power`)
    pub field: String,
    pub change: FieldChange,
}

/// A field moved or hidden
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldChange {
    /// Drawn upright, vertically centred on `at`, with `halign` on the sheet
    Move { at: Pt, halign: HAlign },
    /// No longer shown
    Hide,
}

/// Why [`place_fields`] could not move a field
#[derive(Clone, Debug, PartialEq)]
pub enum PlaceError {
    Parse(ParseError),
    /// Not a schematic (`kicad_sch`)
    NotSchematic,
    /// No placed symbol with this UUID
    NoSymbol(String),
    /// The symbol has no such field, or the field has no `(at ...)` or `(effects ...)` to write
    NoField {
        symbol: String,
        field: String,
    },
    /// Two edits touch the same bytes of the file, at this byte
    Overlap(usize),
}

impl fmt::Display for PlaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaceError::Parse(e) => e.fmt(f),
            PlaceError::NotSchematic => f.write_str("not a schematic"),
            PlaceError::NoSymbol(s) => write!(f, "no placed symbol {s}"),
            PlaceError::NoField { symbol, field } => {
                write!(f, "symbol {symbol} has no field {field} to edit")
            }
            PlaceError::Overlap(at) => write!(f, "two edits touch byte {at}"),
        }
    }
}

impl std::error::Error for PlaceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PlaceError::Parse(e) => Some(e),
            _ => None,
        }
    }
}

/// `text`, a schematic, with each field of `places` moved or hidden. A moved field's `(at ...)` and the
/// justification in its `(effects ...)` are written, a hidden one gets `(hide yes)`; a symbol with a moved field loses its `(fields_autoplaced yes)`, as when a field is
/// moved by hand in KiCad, so KiCad does not place its fields again when the symbol moves.
pub fn place_fields(text: &str, places: &[FieldPlace]) -> Result<String, PlaceError> {
    let tree = Tree::parse(text).map_err(PlaceError::Parse)?;
    let sch = tree.root("kicad_sch").ok_or(PlaceError::NotSchematic)?;
    let mut edits = Edits::new();
    let mut unflagged: Vec<&str> = Vec::new();
    for p in places {
        let sym = tree
            .children(sch, "symbol")
            .find(|s| uuid(&tree, s).as_deref() == Some(p.symbol.as_str()))
            .ok_or_else(|| PlaceError::NoSymbol(p.symbol.clone()))?;
        let no_field = || PlaceError::NoField {
            symbol: p.symbol.clone(),
            field: p.field.clone(),
        };
        let prop = tree
            .children(sym, "property")
            .find(|f| tree.arg(f, 0).as_deref() == Some(p.field.as_str()))
            .ok_or_else(no_field)?;
        let at = tree.child(prop, "at").ok_or_else(no_field)?;
        let effects = tree.child(prop, "effects").ok_or_else(no_field)?;
        match &p.change {
            FieldChange::Move { at: to, halign } => {
                let rotation = tree
                    .child(sym, "at")
                    .and_then(|a| tree.arg(a, 2))
                    .and_then(|r| r.parse::<f64>().ok())
                    .map_or(0, |r| r.round() as i32);
                let mirror = tree
                    .child(sym, "mirror")
                    .and_then(|m| tree.arg(m, 0))
                    .and_then(|m| m.chars().next());
                let (angle, justify) = upright_field(rotation, mirror, *halign);
                edits.replace(
                    at.span.clone(),
                    format!("(at {} {} {angle})", Nm(to.x), Nm(to.y)),
                );
                justification(&tree, effects, justify, &mut edits);
            }
            FieldChange::Hide => hide(&tree, prop, at, effects, &mut edits),
        }
        if !unflagged.contains(&p.symbol.as_str())
            && let Some(flag) = tree.child(sym, "fields_autoplaced")
        {
            unflagged.push(&p.symbol);
            edits.replace(removal(&tree, sym, flag), "");
        }
    }
    Ok(edits
        .apply(text)
        .expect("each field and each flag is edited once"))
}

/// What happens to the text of one field of a placed symbol
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueChange {
    /// The field gets this text; a symbol without the field gets one, copied from its Reference field's layout
    Put(String),
    /// The field is taken off the symbol
    Remove,
}

/// One field's text written or removed
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueEdit {
    /// The UUID of the placed symbol
    pub symbol: String,
    pub field: String,
    pub change: ValueChange,
}

/// `text`, a schematic, with the fields of `edits` written or removed: a field's text replaced in place, a new
/// field added after the symbol's last one, laid out like its Reference field (shown, at the Reference's place, for
/// `place_fields` to move), a removed field's lines gone. Every other byte is kept.
pub fn edit_values(text: &str, edits: &[ValueEdit]) -> Result<String, PlaceError> {
    let tree = Tree::parse(text).map_err(PlaceError::Parse)?;
    let sch = tree.root("kicad_sch").ok_or(PlaceError::NotSchematic)?;
    let mut out = Edits::new();
    // New fields of one symbol go in together, in the order given
    let mut added: Vec<(&Node, Vec<String>)> = Vec::new();
    for e in edits {
        let sym = tree
            .children(sch, "symbol")
            .find(|s| uuid(&tree, s).as_deref() == Some(e.symbol.as_str()))
            .ok_or_else(|| PlaceError::NoSymbol(e.symbol.clone()))?;
        let no_field = || PlaceError::NoField {
            symbol: e.symbol.clone(),
            field: e.field.clone(),
        };
        let prop = tree.children(sym, "property").find(|f| {
            tree.arg(f, 0)
                .is_some_and(|n| n.eq_ignore_ascii_case(&e.field))
        });
        match (&e.change, prop) {
            (ValueChange::Put(v), Some(prop)) => {
                let value = items(prop).get(2).ok_or_else(no_field)?;
                out.replace(value.span.clone(), quote(v));
            }
            (ValueChange::Remove, Some(prop)) => out.replace(removal(&tree, sym, prop), ""),
            (ValueChange::Remove, None) => return Err(no_field()),
            (ValueChange::Put(v), None) => {
                let reference = tree
                    .children(sym, "property")
                    .find(|f| tree.arg(f, 0).as_deref() == Some("Reference"))
                    .ok_or_else(no_field)?;
                let (name, value) = (
                    items(reference).get(1).ok_or_else(no_field)?,
                    items(reference).get(2).ok_or_else(no_field)?,
                );
                let src = tree.text();
                let copy = format!(
                    "{}{}{}{}{}",
                    &src[reference.span.start..name.span.start],
                    quote(&e.field),
                    &src[name.span.end..value.span.start],
                    quote(v),
                    &src[value.span.end..reference.span.end],
                )
                .replace("(hide yes)", "(hide no)");
                match added.iter_mut().find(|(s, _)| s.span == sym.span) {
                    Some((_, l)) => l.push(copy),
                    None => added.push((sym, vec![copy])),
                }
            }
        }
    }
    for (sym, copies) in added {
        let last = tree
            .children(sym, "property")
            .last()
            .expect("a symbol with a Reference has a property");
        let at = last.span.end;
        for c in copies {
            let lead = match tree.indent(last) {
                Some(i) => format!("{}{i}", tree.newline()),
                None => " ".to_owned(),
            };
            out.insert(at, format!("{lead}{c}"));
        }
    }
    out.apply(text).map_err(|o| PlaceError::Overlap(o.at))
}

/// A placed symbol as its sheet file holds it
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedSymbol {
    pub uuid: String,
    /// The library id (`Device:R`)
    pub lib_id: String,
    /// The name of its copy in `lib_symbols`: the `lib_name` when the symbol has one, else the library id
    pub lib_symbol: String,
    /// Its properties, name and text, in file order
    pub properties: Vec<(String, String)>,
}

/// The placed symbols of `text`, a schematic, in file order
pub fn placed_symbols(text: &str) -> Result<Vec<PlacedSymbol>, PlaceError> {
    let tree = Tree::parse(text).map_err(PlaceError::Parse)?;
    let sch = tree.root("kicad_sch").ok_or(PlaceError::NotSchematic)?;
    let value = |n: &Node, head: &str| {
        tree.child(n, head)
            .and_then(|c| tree.arg(c, 0))
            .map(|v| v.into_owned())
    };
    Ok(tree
        .children(sch, "symbol")
        .filter_map(|s| {
            let lib_id = value(s, "lib_id")?;
            Some(PlacedSymbol {
                uuid: uuid(&tree, s)?,
                lib_symbol: value(s, "lib_name").unwrap_or_else(|| lib_id.clone()),
                lib_id,
                properties: tree
                    .children(s, "property")
                    .filter_map(|p| {
                        let name = tree.arg(p, 0)?.into_owned();
                        let text = tree.arg(p, 1).map(|v| v.into_owned()).unwrap_or_default();
                        Some((name, text))
                    })
                    .collect(),
            })
        })
        .collect())
}

/// The placed symbol's UUID
fn uuid(tree: &Tree, sym: &Node) -> Option<String> {
    tree.child(sym, "uuid")
        .and_then(|u| tree.arg(u, 0))
        .map(|u| u.into_owned())
}

/// `(hide yes)` where the file keeps it: on the property (KiCad 9 and newer, which write `show_name` and
/// `do_not_autoplace` there too) or in its effects (KiCad 8); an old `(hide no)` is turned
fn hide(tree: &Tree, prop: &Node, at: &Node, effects: &Node, edits: &mut Edits) {
    let old = tree
        .child(prop, "hide")
        .or_else(|| tree.child(effects, "hide"));
    if let Some(old) = old {
        edits.replace(old.span.clone(), "(hide yes)");
    } else if tree.child(prop, "show_name").is_some()
        || tree.child(prop, "do_not_autoplace").is_some()
    {
        edits.insert(at.span.end, after(tree, at, "(hide yes)"));
    } else {
        let last = items(effects)
            .last()
            .expect("a list with a head has an item");
        edits.insert(last.span.end, after(tree, last, "(hide yes)"));
    }
}

/// `new` to go after `item`: on a line of its own when `item` is a list on one, else after a space
fn after(tree: &Tree, item: &Node, new: &str) -> String {
    match tree
        .indent(item)
        .filter(|_| matches!(item.kind, Kind::List(_)))
    {
        Some(indent) => format!("{}{indent}{new}", tree.newline()),
        None => format!(" {new}"),
    }
}

/// `(justify left)` or `(justify right)` in place of the old one, or after the font; no justification for centred
/// text
fn justification(tree: &Tree, effects: &Node, halign: HAlign, edits: &mut Edits) {
    let word = match halign {
        HAlign::Left => Some("(justify left)"),
        HAlign::Right => Some("(justify right)"),
        HAlign::Center => None,
    };
    match (tree.child(effects, "justify"), word) {
        (Some(old), Some(new)) => edits.replace(old.span.clone(), new),
        (Some(old), None) => edits.replace(removal(tree, effects, old), ""),
        (None, Some(new)) => {
            let last = items(effects)
                .last()
                .expect("a list with a head has an item");
            edits.insert(last.span.end, after(tree, last, new));
        }
        (None, None) => {}
    }
}

/// The bytes that remove `item` from `list`: its line when it stands on one, else the space before it
fn removal(tree: &Tree, list: &Node, item: &Node) -> std::ops::Range<usize> {
    let all = items(list);
    let i = all
        .iter()
        .position(|n| n.span == item.span)
        .expect("the item is in its list");
    let before = all[i - 1].span.end;
    match tree.indent(item) {
        // From the end of the line before: its line break and indent go with it
        Some(_) => {
            tree.text()[..item.span.start]
                .rfind('\n')
                .map_or(before, |nl| nl.max(before))
                .min(item.span.start)..item.span.end
        }
        None => before..item.span.end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCH: &str = "(kicad_sch\n\t(version 20250114)\n\t(symbol\n\t\t(lib_id \"Device:R\")\n\t\t(at 10 20 90)\n\
        \t\t(fields_autoplaced yes)\n\t\t(uuid \"u1\")\n\t\t(property \"Reference\" \"R1\"\n\t\t\t(at 1 2 0)\n\
        \t\t\t(effects\n\t\t\t\t(font\n\t\t\t\t\t(size 1.27 1.27)\n\t\t\t\t)\n\t\t\t)\n\t\t)\n\
        \t\t(property \"Value\" \"1k\"\n\t\t\t(at 3 4 90)\n\t\t\t(effects\n\t\t\t\t(font\n\t\t\t\t\t(size 1.27 1.27)\n\
        \t\t\t\t)\n\t\t\t\t(justify left bottom)\n\t\t\t)\n\t\t)\n\t)\n)\n";

    fn place(field: &str, x: f64, halign: HAlign) -> FieldPlace {
        FieldPlace {
            symbol: "u1".into(),
            field: field.into(),
            change: FieldChange::Move {
                at: Pt::new((x * 1e6) as i64, 21_270_000),
                halign,
            },
        }
    }

    /// Only the moved fields' position and justification change, and the autoplaced flag goes
    #[test]
    fn writes_only_the_field() {
        let out = place_fields(
            SCH,
            &[
                place("Reference", 12.7, HAlign::Left),
                place("Value", 7.62, HAlign::Center),
            ],
        )
        .unwrap();
        let want = SCH
            .replace("\t\t(fields_autoplaced yes)\n", "")
            .replace("(at 1 2 0)", "(at 12.7 21.27 90)")
            .replace("(at 3 4 90)", "(at 7.62 21.27 90)")
            .replace("\n\t\t\t\t(justify left bottom)", "");
        // On a symbol turned by 90 degrees the field's own axis turns twice: drawn left, it is written right
        let want = want.replacen(
            "\t\t\t\t\t(size 1.27 1.27)\n\t\t\t\t)\n",
            "\t\t\t\t\t(size 1.27 1.27)\n\t\t\t\t)\n\t\t\t\t(justify right)\n",
            1,
        );
        assert_eq!(out, want);
    }

    fn hidden(field: &str) -> FieldPlace {
        FieldPlace {
            symbol: "u1".into(),
            field: field.into(),
            change: FieldChange::Hide,
        }
    }

    /// KiCad 8 hides a field in its effects; KiCad 9 and newer on the property, beside show_name
    #[test]
    fn hides_where_the_file_does() {
        let out = place_fields(SCH, &[hidden("Value")]).unwrap();
        let want = SCH.replace("\t\t(fields_autoplaced yes)\n", "").replace(
            "(justify left bottom)",
            "(justify left bottom)\n\t\t\t\t(hide yes)",
        );
        assert_eq!(out, want);

        let nine = SCH.replace("(at 3 4 90)", "(at 3 4 90)\n\t\t\t(show_name no)");
        let out = place_fields(&nine, &[hidden("Value")]).unwrap();
        let want = nine
            .replace("\t\t(fields_autoplaced yes)\n", "")
            .replace("(at 3 4 90)", "(at 3 4 90)\n\t\t\t(hide yes)");
        assert_eq!(out, want);

        let shown = nine.replace("(show_name no)", "(hide no)");
        let out = place_fields(&shown, &[hidden("Value")]).unwrap();
        assert!(out.contains("(at 3 4 90)\n\t\t\t(hide yes)\n"), "{out}");
    }

    fn put(field: &str, v: &str) -> ValueEdit {
        ValueEdit {
            symbol: "u1".into(),
            field: field.into(),
            change: ValueChange::Put(v.into()),
        }
    }

    /// A field's text is replaced, a new field copies the Reference's layout after the last one, a removed field's
    /// line goes; the rest stays byte for byte
    #[test]
    fn writes_and_removes_values() {
        let with = SCH.to_owned();
        let out = edit_values(&with, &[put("Value", "2k2")]).unwrap();
        assert_eq!(out, with.replace("\"1k\"", "\"2k2\""));

        let out = edit_values(&with, &[put("Voltage", "25V"), put("Power", "63mW")]).unwrap();
        let reference = "\t\t(property \"Reference\" \"R1\"\n\t\t\t(at 1 2 0)\n\t\t\t(effects\n\t\t\t\t(font\n\t\t\t\t\t(size 1.27 1.27)\n\t\t\t\t)\n\t\t\t)\n\t\t)";
        let copy = |name: &str, v: &str| {
            reference.replace("\"Reference\" \"R1\"", &format!("\"{name}\" \"{v}\""))
        };
        let last = "\t\t\t\t(justify left bottom)\n\t\t\t)\n\t\t)";
        assert!(
            out.contains(&format!(
                "{last}\n{}\n{}\n\t)",
                copy("Voltage", "25V"),
                copy("Power", "63mW")
            )),
            "{out}"
        );

        let two = with.replace("(at 1 2 0)", "(at 1 2 0)\n\t\t\t(hide yes)");
        let out = edit_values(&two, &[put("Voltage", "25V")]).unwrap();
        assert!(
            out.contains("(property \"Voltage\" \"25V\"\n\t\t\t(at 1 2 0)\n\t\t\t(hide no)"),
            "{out}"
        );

        let gone = edit_values(
            &with,
            &[ValueEdit {
                symbol: "u1".into(),
                field: "value".into(),
                change: ValueChange::Remove,
            }],
        )
        .unwrap();
        assert!(!gone.contains("\"Value\""), "{gone}");
        assert!(gone.ends_with("\t\t)\n\t)\n)\n"), "{gone}");
        assert_eq!(
            edit_values(
                &with,
                &[put("Voltage", "5V")].map(|mut e| {
                    e.symbol = "nope".into();
                    e
                })
            ),
            Err(PlaceError::NoSymbol("nope".into()))
        );
    }

    #[test]
    fn errors() {
        assert_eq!(
            place_fields(SCH, &[place("Footprint", 1.0, HAlign::Left)]),
            Err(PlaceError::NoField {
                symbol: "u1".into(),
                field: "Footprint".into()
            })
        );
        assert_eq!(
            place_fields("(kicad_pcb)", &[]),
            Err(PlaceError::NotSchematic)
        );
    }
}
