//! The MPN fields of a part: which fields hold a manufacturer part number, and their values in a variant.

use erc_model::{PartId, View};

/// Whether a field holds a manufacturer part number: `MPN`, `Mpn1`, `MPN_2`, `MPN_Alt`, `Mfr. Part`, `Manufacturer_Part_Number`,
/// `Manufacturer Part Number`, `MFR_PN` (case, spaces and punctuation ignored)
pub fn mpn_field(name: &str) -> bool {
    let n: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    n.starts_with("mpn")
        || matches!(
            n.as_str(),
            "mfrpart" | "mfrpartnumber" | "mfrpn" | "manufacturerpartnumber" | "manufacturerpn"
        )
}

/// A part's MPN fields as the view resolves them (the variant's override first): name and value, empty values left
/// out, in field order
pub fn mpns<'a>(view: View<'a>, part: PartId) -> Vec<(&'a str, &'a str)> {
    view.board
        .part(part)
        .fields
        .iter()
        .filter(|f| mpn_field(&f.name))
        .filter_map(|f| {
            let v = view.field(part, &f.name)?.trim();
            (!v.is_empty() && v != "~").then_some((f.name.as_str(), v))
        })
        .collect()
}

/// How a `Datasheet` field naming part numbers for mx3's document library starts (any case):
/// `mx3:${Mpn1} ${Mpn2}`. KiCad's Show Datasheet hands it to mx3's handler (mx3 `docs/kicad-datasheet.md`).
pub const MX3_SCHEME: &str = "mx3:";
/// A longer `Datasheet` field is no `mx3:` link.
const MX3_LINK_BYTES_MAX: usize = 4096;

/// Whether a `Datasheet` field is an `mx3:` link.
pub fn is_mx3_link(datasheet: &str) -> bool {
    datasheet
        .trim()
        .get(..MX3_SCHEME.len())
        .is_some_and(|s| s.eq_ignore_ascii_case(MX3_SCHEME))
}

/// An `mx3:` `Datasheet` field with its `${Field}` references filled from `field` (a name, any case), as KiCad's
/// schematic editor fills them; a reference with no such field stays. `None` when the field is no `mx3:` link.
pub fn mx3_link_resolved<'a>(
    datasheet: &str,
    field: impl Fn(&str) -> Option<&'a str>,
) -> Option<String> {
    let text = datasheet.trim();
    if !is_mx3_link(text) || text.len() > MX3_LINK_BYTES_MAX {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("${") {
        out.push_str(&rest[..at]);
        let after = &rest[at + 2..];
        match after.find('}') {
            Some(end) => {
                let name = &after[..end];
                match field(name) {
                    Some(v) => out.push_str(v.trim()),
                    None => out.push_str(&rest[at..at + 2 + end + 1]),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[at..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    Some(out)
}

/// The part numbers an `mx3:` `Datasheet` field names, references filled as [`mx3_link_resolved`] fills them, by
/// the rules of mx3's handler: an optional `//` after the scheme, percent escapes decoded, part numbers apart by
/// white space, a reference left unresolved (a word holding `${`) and `~` dropped, each once. `None` when the
/// field is no `mx3:` link.
pub fn mx3_link_mpns<'a>(
    datasheet: &str,
    field: impl Fn(&str) -> Option<&'a str>,
) -> Option<Vec<String>> {
    let resolved = mx3_link_resolved(datasheet, field)?;
    let rest = &resolved[MX3_SCHEME.len()..];
    let rest = rest.strip_prefix("//").unwrap_or(rest);
    let mut out: Vec<String> = Vec::new();
    for w in percent_decode(rest).split_whitespace() {
        if !w.contains("${") && w != "~" && !out.iter().any(|o| o.eq_ignore_ascii_case(w)) {
            out.push(w.to_owned());
        }
    }
    Some(out)
}

/// `%XX` escapes as their bytes; a `%` without two hex digits after it stays.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while let Some(&c) = b.get(i) {
        let hex = b
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (c, hex) {
            (b'%', Some(v)) => {
                out.push(v);
                i += 3;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::{is_mx3_link, mpn_field, mx3_link_mpns, mx3_link_resolved};

    fn fields(name: &str) -> Option<&'static str> {
        match name.to_ascii_lowercase().as_str() {
            "mpn1" => Some("LM358/NOPB"),
            "mpn2" => Some(" 74HC00#PBF "),
            "empty" => Some(""),
            _ => None,
        }
    }

    #[test]
    fn mx3_links_name_part_numbers() {
        assert!(is_mx3_link(" MX3:x"));
        assert!(!is_mx3_link("https://ti.com/lit/ds/x.pdf"));
        assert!(!is_mx3_link("docs/mx3:x.pdf"));
        assert_eq!(
            mx3_link_resolved("mx3:${Mpn1} ${MPN2} ${Mpn3}", fields).as_deref(),
            Some("mx3:LM358/NOPB 74HC00#PBF ${Mpn3}")
        );
        assert_eq!(
            mx3_link_mpns("mx3:${Mpn1} ${MPN2} ${Mpn3} ${Empty} ~", fields).unwrap(),
            ["LM358/NOPB", "74HC00#PBF"]
        );
        assert_eq!(
            mx3_link_mpns("mx3://A%20B a X,REEL", |_| None).unwrap(),
            ["A", "B", "X,REEL"]
        );
        assert_eq!(
            mx3_link_mpns("mx3:${Mpn1", |_| None).unwrap(),
            Vec::<String>::new()
        );
        assert!(mx3_link_mpns("~", fields).is_none());
        assert!(mx3_link_mpns("/lib/x.pdf", fields).is_none());
    }

    #[test]
    fn mpn_fields() {
        for n in [
            "MPN",
            "Mpn1",
            "MPN_2",
            "MPN_Alt",
            "Mfr. Part",
            "Manufacturer_Part_Number",
            "MFR_PN",
        ] {
            assert!(mpn_field(n), "{n}");
        }
        for n in ["Value", "Manufacturer", "Datasheet", "LCSC"] {
            assert!(!mpn_field(n), "{n}");
        }
    }
}
