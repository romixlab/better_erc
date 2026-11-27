//! The `mx3:` `Datasheet` link of a symbol's MPN fields (`mx3:${Mpn1} ${Mpn2}`), as the symbol library writes it:
//! one `${Field}` reference per MPN field the symbol has, in field order. `KiCad` has no wildcard over fields, so a
//! field added later goes unnamed, and a reference to a field the symbol lacks shows unresolved. [`stale`] tells
//! both, [`link_of`] writes the link the fields call for. A vendor URL or an empty `Datasheet` is no link.

use crate::mpn::{datasheet_link, mpn_field};

/// What is wrong with a symbol's `mx3:` link
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stale {
    /// MPN fields the link references and the symbol lacks (`Mpn5`), in link order
    pub unknown: Vec<String>,
    /// MPN fields the symbol has and the link does not reference, in field order
    pub missing: Vec<String>,
}

/// Whether a field is an MPN field a link can reference: an MPN field ([`mpn_field`]) whose name `KiCad` takes in a
/// `${...}` reference (no white space or braces)
pub fn link_field(name: &str) -> bool {
    mpn_field(name) && !name.contains(|c: char| c.is_whitespace() || c == '{' || c == '}')
}

/// The field names a text references as `${Name}`, in text order; an unclosed `${` ends the list
pub fn references(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("${") {
        let after = &rest[at + 2..];
        let Some(end) = after.find('}') else {
            break;
        };
        out.push(&after[..end]);
        rest = &after[end + 1..];
    }
    out
}

/// What is wrong with `datasheet`, a symbol's `Datasheet` field, given the names of the symbol's fields; `None`
/// when it is no `mx3:` link or references exactly the symbol's MPN fields (any case, any order).
pub fn stale<'a>(datasheet: &str, fields: impl IntoIterator<Item = &'a str>) -> Option<Stale> {
    if !datasheet_link::is_link(datasheet) {
        return None;
    }
    let mpns: Vec<&str> = fields.into_iter().filter(|n| link_field(n)).collect();
    let named = references(datasheet);
    let has = |list: &[&str], n: &str| list.iter().any(|m| m.eq_ignore_ascii_case(n));
    let mut s = Stale::default();
    for n in named.iter().filter(|n| link_field(n) && !has(&mpns, n)) {
        if !s.unknown.iter().any(|u| u.eq_ignore_ascii_case(n)) {
            s.unknown.push((*n).to_owned());
        }
    }
    for n in mpns.iter().filter(|n| !has(&named, n)) {
        if !s.missing.iter().any(|m| m.eq_ignore_ascii_case(n)) {
            s.missing.push((*n).to_owned());
        }
    }
    (!s.unknown.is_empty() || !s.missing.is_empty()).then_some(s)
}

/// The link a symbol with these fields carries: `mx3:` and a reference to each MPN field, in field order; `None`
/// when it has no MPN field
pub fn link_of<'a>(fields: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut mpns: Vec<&str> = Vec::new();
    for n in fields.into_iter().filter(|n| link_field(n)) {
        if !mpns.iter().any(|m| m.eq_ignore_ascii_case(n)) {
            mpns.push(n);
        }
    }
    if mpns.is_empty() {
        return None;
    }
    let refs: Vec<String> = mpns.iter().map(|n| format!("${{{n}}}")).collect();
    Some(format!("{}{}", datasheet_link::SCHEME, refs.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIELDS: [&str; 7] = [
        "Reference",
        "Value",
        "Datasheet",
        "Mpn1",
        "Mpn2",
        "Mfg1",
        "Mpn12",
    ];

    #[test]
    fn a_link_of_each_mpn_field() {
        assert_eq!(
            link_of(FIELDS).as_deref(),
            Some("mx3:${Mpn1} ${Mpn2} ${Mpn12}")
        );
        assert_eq!(link_of(["Reference", "Value"]), None);
        assert_eq!(link_of(["Mpn1", "MPN1"]).as_deref(), Some("mx3:${Mpn1}"));
    }

    #[test]
    fn a_right_link_is_not_stale() {
        assert_eq!(stale("mx3:${Mpn1} ${Mpn2} ${Mpn12}", FIELDS), None);
        assert_eq!(stale("MX3:${MPN12} ${mpn1} ${Mpn2}", FIELDS), None);
        assert_eq!(stale("mx3:${Mpn1}", ["Value", "Mpn1"]), None);
    }

    #[test]
    fn unknown_and_missing_fields() {
        assert_eq!(
            stale("mx3:${Mpn1} ${Mpn2} ${Mpn3} ${Mpn4}", FIELDS),
            Some(Stale {
                unknown: vec!["Mpn3".into(), "Mpn4".into()],
                missing: vec!["Mpn12".into()],
            })
        );
        // a literal part number names no field; other references are not MPN fields
        assert_eq!(
            stale(
                "mx3:TPS2116DRLR ${Value} ${Mpn1}",
                ["Value", "Mpn1", "Mpn2"]
            ),
            Some(Stale {
                unknown: vec![],
                missing: vec!["Mpn2".into()],
            })
        );
        assert_eq!(
            stale("mx3:${Mpn1}", ["Value"]),
            Some(Stale {
                unknown: vec!["Mpn1".into()],
                missing: vec![],
            })
        );
    }

    #[test]
    fn no_link_is_never_stale() {
        assert_eq!(stale("", FIELDS), None);
        assert_eq!(stale("~", FIELDS), None);
        assert_eq!(stale("https://www.ti.com/lit/ds/x.pdf", FIELDS), None);
    }

    #[test]
    fn references_in_text_order() {
        assert_eq!(references("mx3:${A} x ${B}${C} ${D"), ["A", "B", "C"]);
        assert!(references("mx3:A").is_empty());
    }
}
