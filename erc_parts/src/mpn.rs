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

/// The `mx3:` datasheet link format (`mx3:${Mpn1} ${Mpn2}`): its one reader, shared with mx3's handler.
pub use mx3_api_core::datasheet_link;

#[cfg(test)]
mod tests {
    use super::mpn_field;

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
