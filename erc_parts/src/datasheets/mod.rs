//! A design's datasheet URLs by MPN (TOOL-18): the `Datasheet` field of every fitted part that is an http(s) URL,
//! on each of the part's MPNs, for `berc parts datasheets`. With the cargo feature `mx3`, [`mx3`] states them in
//! the mx3 part store as each MPN's datasheet fact.
//!
//! A URL is taken as the schematic writes it: a vendor's product page counts as much as a PDF. Field values are
//! only compared and passed on, never opened.

#[cfg(feature = "mx3")]
pub mod mx3;

use crate::board::views;
use crate::mpn::mpns;
use erc_model::{Design, natural_cmp};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// The field read
pub const DATASHEET_FIELD: &str = "Datasheet";

/// One MPN of the design with the datasheet URLs its parts' `Datasheet` fields give
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MpnDatasheets {
    /// As the board writes it
    pub mpn: String,
    /// Each URL with the references whose field gives it, in URL order; two or more URLs disagree
    pub urls: Vec<UrlRefs>,
}

/// A datasheet URL and the references that give it
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UrlRefs {
    pub url: String,
    /// In natural order
    pub references: Vec<String>,
}

/// Whether a field value is a web URL: `http://` or `https://`, letter case ignored
pub fn web_url(value: &str) -> bool {
    let lower = value.get(..8).unwrap_or(value).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Every MPN of the fitted parts of `design` (in the base design and each variant) whose part's `Datasheet` field is
/// a web URL, sorted by MPN; a field with anything else (empty, `~`, a file, an `mx3:` link) gives nothing
pub fn of(design: &Design) -> Vec<MpnDatasheets> {
    let mut found: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    for board in &design.boards {
        for view in views(board) {
            for p in view.fitted_components() {
                let Some(url) = view
                    .field(p, DATASHEET_FIELD)
                    .map(str::trim)
                    .filter(|u| web_url(u))
                else {
                    continue;
                };
                for (_, mpn) in mpns(view, p) {
                    found
                        .entry(mpn.to_owned())
                        .or_default()
                        .entry(url.to_owned())
                        .or_default()
                        .insert(board.part(p).designator.clone());
                }
            }
        }
    }
    found
        .into_iter()
        .map(|(mpn, urls)| MpnDatasheets {
            mpn,
            urls: urls
                .into_iter()
                .map(|(url, refs)| {
                    let mut references: Vec<String> = refs.into_iter().collect();
                    references.sort_by(|a, b| natural_cmp(a, b));
                    UrlRefs { url, references }
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::web_url;

    #[test]
    fn web_urls() {
        for u in ["https://x/a.pdf", "http://x", "HTTPS://X/A.PDF"] {
            assert!(web_url(u), "{u}");
        }
        for u in [
            "",
            "~",
            "mx3:${Mpn1}",
            "file:///a.pdf",
            "${KIPRJMOD}/a.pdf",
            "ftp://x",
            "https:/x",
        ] {
            assert!(!web_url(u), "{u}");
        }
    }
}
