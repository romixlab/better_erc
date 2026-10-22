//! `BOM` - BOM hygiene

mod bom_1;
mod bom_10;
mod bom_11;
mod bom_14;
mod bom_17;
mod bom_3;
mod bom_4;
mod bom_5;
mod bom_6;
mod bom_8;
mod bom_9;

use erc_model::{PartId, View};

/// Whether a field holds a manufacturer part number: `MPN`, `Mpn1`, `MPN_2`, `MPN_Alt`, `Mfr. Part`, `Manufacturer_Part_Number`,
/// `Manufacturer Part Number`, `MFR_PN` (case, spaces and punctuation ignored)
pub(crate) fn mpn_field(name: &str) -> bool {
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
pub(crate) fn mpns<'a>(view: View<'a>, part: PartId) -> Vec<(&'a str, &'a str)> {
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

/// A value that is a decision not taken: starts or ends with `?` (`?`, `??k`, `4k7?`), or has `TBD`, `TODO` or
/// `calc` as a word
pub(crate) fn placeholder(value: &str) -> bool {
    let v = value.trim();
    v.starts_with('?')
        || v.ends_with('?')
        || v.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| {
            ["TBD", "TODO", "CALC"]
                .iter()
                .any(|x| w.eq_ignore_ascii_case(x))
        })
}

#[cfg(test)]
mod tests {
    use super::{mpn_field, placeholder};

    #[test]
    fn placeholders() {
        for v in [
            "?",
            "??k",
            "4k7?",
            "TBD",
            "tbd",
            "10k TODO",
            "calc",
            "CALC later",
        ] {
            assert!(placeholder(v), "{v}");
        }
        for v in ["", "4k7", "DNP", "TBDX", "STUDIO", "CALCULATOR"] {
            assert!(!placeholder(v), "{v}");
        }
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
