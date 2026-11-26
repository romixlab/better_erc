//! The MPN fields of a part: which fields hold a manufacturer part number, the maker field beside each (`Mfg`
//! beside `Mpn`, `Mfg1` beside `Mpn1`), and their values in a variant.

use erc_model::{PartId, View};

/// A field name with case, spaces and punctuation dropped
fn normal(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Whether a field holds a manufacturer part number: `MPN`, `Mpn1`, `MPN_2`, `MPN_Alt`, `Mfr. Part`, `Manufacturer_Part_Number`,
/// `Manufacturer Part Number`, `MFR_PN` (case, spaces and punctuation ignored)
pub fn mpn_field(name: &str) -> bool {
    let n = normal(name);
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
        .filter_map(|f| Some((f.name.as_str(), value(view, part, &f.name)?)))
        .collect()
}

/// The suffix that pairs an MPN field with its maker field: what follows `mpn` (`1` of `Mpn1`, `alt` of
/// `MPN_Alt`), empty for `MPN` and the long names (`Manufacturer_Part_Number`)
fn mpn_suffix(name: &str) -> String {
    normal(name)
        .strip_prefix("mpn")
        .map(str::to_owned)
        .unwrap_or_default()
}

/// Whether field `name` names the maker of the MPN in field `mpn`: `Mfg` and the MPN field's suffix (`Mfg1` beside
/// `Mpn1`, `MFG_2` beside `MPN_2`, `Mfg` beside `MPN`), case, spaces and punctuation ignored
pub fn maker_field_of(name: &str, mpn: &str) -> bool {
    normal(name)
        .strip_prefix("mfg")
        .is_some_and(|s| s == mpn_suffix(mpn))
}

/// Whether a field names a maker (`Mfg`, `Mfg1`, `MFG_Alt`)
pub fn maker_field(name: &str) -> bool {
    normal(name).starts_with("mfg")
}

/// One MPN of a part with the maker named beside it
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MpnEntry<'a> {
    /// The MPN field (`Mpn1`)
    pub field: &'a str,
    pub mpn: &'a str,
    /// The maker field and its value, when the part fills one
    pub maker: Option<(&'a str, &'a str)>,
}

impl MpnEntry<'_> {
    /// The MPN and maker, as the lock keys them
    pub fn key(&self) -> crate::lock::PartKey {
        crate::lock::PartKey::new(self.mpn, self.maker.map(|(_, m)| m))
    }
}

/// A field's value in the view, trimmed; `None` when absent, empty or `~`
fn value<'a>(view: View<'a>, part: PartId, name: &str) -> Option<&'a str> {
    let v = view.field(part, name)?.trim();
    (!v.is_empty() && v != "~").then_some(v)
}

/// A part's MPN fields as [`mpns`] gives them, each with the maker field beside it when filled
pub fn mpn_entries<'a>(view: View<'a>, part: PartId) -> Vec<MpnEntry<'a>> {
    let fields = &view.board.part(part).fields;
    mpns(view, part)
        .into_iter()
        .map(|(field, mpn)| MpnEntry {
            field,
            mpn,
            maker: fields
                .iter()
                .filter(|f| maker_field_of(&f.name, field))
                .find_map(|f| Some((f.name.as_str(), value(view, part, &f.name)?))),
        })
        .collect()
}

/// A part's filled maker fields with no filled MPN field beside them: name and value, in field order
pub fn lone_makers<'a>(view: View<'a>, part: PartId) -> Vec<(&'a str, &'a str)> {
    let fields = &view.board.part(part).fields;
    let filled = mpns(view, part);
    fields
        .iter()
        .filter(|f| maker_field(&f.name))
        .filter(|f| !filled.iter().any(|(m, _)| maker_field_of(&f.name, m)))
        .filter_map(|f| Some((f.name.as_str(), value(view, part, &f.name)?)))
        .collect()
}

/// The `mx3:` datasheet link format (`mx3:${Mpn1} ${Mpn2}`): its one reader, shared with mx3's handler.
pub use mx3_api_core::datasheet_link;

#[cfg(test)]
mod tests {
    use super::{maker_field, maker_field_of, mpn_field};

    #[test]
    fn maker_fields() {
        for (mfg, mpn) in [
            ("Mfg", "Mpn"),
            ("MFG", "MPN"),
            ("Mfg1", "Mpn1"),
            ("MFG_2", "MPN_2"),
            ("Mfg_Alt", "MPN_Alt"),
            ("Mfg", "Manufacturer_Part_Number"),
        ] {
            assert!(maker_field_of(mfg, mpn), "{mfg} beside {mpn}");
        }
        for (mfg, mpn) in [("Mfg1", "Mpn"), ("Mfg", "Mpn1"), ("Manufacturer", "MPN")] {
            assert!(!maker_field_of(mfg, mpn), "{mfg} not beside {mpn}");
        }
        assert!(maker_field("Mfg2"));
        assert!(!maker_field("Manufacturer"));
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
