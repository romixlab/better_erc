//! Packages: what a part's footprint says about how it is assembled. The footprint name gives the case size of a
//! chip part (`R_0402_1005Metric` is an 0402) and the mount type (`Resistor_SMD`, `PinHeader_1x04_P2.54mm`,
//! `TO-220-3_Vertical`); the board file, when it was read, says the mount type for sure (`attr smd` or
//! `through_hole`) and the side a footprint sits on.

use erc_model::layout::Side;
use erc_model::{Board, PartId};
use serde::{Deserialize, Serialize};

/// How a part is soldered
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Mount {
    Smd,
    Tht,
}

impl Mount {
    pub fn name(self) -> &'static str {
        match self {
            Mount::Smd => "SMD",
            Mount::Tht => "THT",
        }
    }
}

/// A part's package as its footprint tells it
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    /// The footprint library (`Resistor_SMD`), empty when the field has no `library:` prefix
    pub library: String,
    /// The footprint name without its library (`R_0402_1005Metric`)
    pub name: String,
    /// The imperial case code of a chip part (`0402`), from the name
    pub size: Option<&'static str>,
    /// SMD or through-hole: the board file's attribute when it was read, else the name's reading
    pub mount: Option<Mount>,
    /// The side of the board, when the board file was read
    pub side: Option<Side>,
}

/// Imperial chip case codes with their metric names, smallest first
const SIZES: &[(&str, &str)] = &[
    ("01005", "0402Metric"),
    ("0201", "0603Metric"),
    ("0402", "1005Metric"),
    ("0603", "1608Metric"),
    ("0805", "2012Metric"),
    ("1008", "2520Metric"),
    ("1206", "3216Metric"),
    ("1210", "3225Metric"),
    ("1806", "4516Metric"),
    ("1812", "4532Metric"),
    ("2010", "5025Metric"),
    ("2220", "5750Metric"),
    ("2512", "6332Metric"),
];

/// Name tokens of through-hole packages
const THT_WORDS: &[&str] = &[
    "THT",
    "DIP",
    "PDIP",
    "TO-92",
    "TO-220",
    "TO-247",
    "TO-3P",
    "Radial",
    "Axial",
    "PinHeader",
    "PinSocket",
    "IDC",
    "Terminal",
    "TerminalBlock",
    "DIN",
    "SIP",
    "ZIP",
];

/// Name tokens of surface-mount packages
const SMD_WORDS: &[&str] = &[
    "SMD", "SMT", "SOT", "SOIC", "SO", "SOP", "SSOP", "TSSOP", "MSOP", "VSSOP", "QFN", "VQFN",
    "WQFN", "UQFN", "DFN", "WDFN", "UDFN", "SON", "WSON", "VSON", "QFP", "LQFP", "TQFP", "BGA",
    "LGA", "WLCSP", "CSP", "SOD", "TO-252", "TO-263", "DPAK", "D2PAK", "PowerPAK", "SC-70",
    "SC-88", "MELF", "MiniMELF",
];

impl Package {
    /// Reads a footprint field (`Resistor_SMD:R_0402_1005Metric`); `None` for an empty one
    pub fn from_footprint(footprint: &str) -> Option<Package> {
        let footprint = footprint.trim();
        if footprint.is_empty() {
            return None;
        }
        let (library, name) = footprint.split_once(':').unwrap_or(("", footprint));
        let tokens: Vec<&str> = name.split(['_', ' ']).collect();
        let size = tokens.iter().find_map(|t| {
            SIZES
                .iter()
                .find(|(imperial, metric)| t == imperial || t == metric)
                .map(|(imperial, _)| *imperial)
        });
        let words = |list: &[&str]| {
            [library, name].iter().any(|s| {
                s.split(['_', ' ', ':']).any(|t| {
                    list.iter()
                        .any(|w| t.eq_ignore_ascii_case(w) || starts_with_word(t, w))
                })
            })
        };
        // An SMD header (`PinHeader_1x04_P1.27mm_Vertical_SMD`) is SMD: the explicit word wins
        let mount = if words(&["SMD", "SMT"]) {
            Some(Mount::Smd)
        } else if words(THT_WORDS) {
            Some(Mount::Tht)
        } else if size.is_some() || words(SMD_WORDS) {
            Some(Mount::Smd)
        } else {
            None
        };
        Some(Package {
            library: library.to_string(),
            name: name.to_string(),
            size,
            mount,
            side: None,
        })
    }

    /// A part's package: its Footprint field, with the board file's mount attribute and side when the layout was
    /// read and has the part's footprint
    pub fn of(board: &Board, part: PartId) -> Option<Package> {
        let field = board.part(part).field("Footprint").unwrap_or("");
        let footprint = board
            .layout
            .as_ref()
            .and_then(|l| l.footprints.iter().find(|f| f.part == Some(part)));
        let mut p = match Package::from_footprint(field) {
            Some(p) => p,
            None => Package::from_footprint(&footprint?.lib_id)?,
        };
        if let Some(f) = footprint {
            p.side = Some(f.side);
            match f.attrs.mount.as_deref() {
                Some("smd") => p.mount = Some(Mount::Smd),
                Some("through_hole") => p.mount = Some(Mount::Tht),
                _ => {}
            }
        }
        Some(p)
    }

    /// The case size when there is one, else the footprint name: what tells two packages of one value apart
    pub fn short(&self) -> &str {
        self.size.unwrap_or(&self.name)
    }
}

/// `TO-220-3` starts with the word `TO-220`, `SOT-23-5` with `SOT`, `QFN-32-1EP` with `QFN`: a word followed by
/// `-` and more
fn starts_with_word(token: &str, word: &str) -> bool {
    token.len() > word.len()
        && token.is_char_boundary(word.len())
        && token[..word.len()].eq_ignore_ascii_case(word)
        && token[word.len()..].starts_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(f: &str) -> (Option<&'static str>, Option<Mount>) {
        let p = Package::from_footprint(f).unwrap();
        (p.size, p.mount)
    }

    #[test]
    fn footprint_names() {
        assert_eq!(
            read("Resistor_SMD:R_0402_1005Metric"),
            (Some("0402"), Some(Mount::Smd))
        );
        assert_eq!(read("C_0603_1608Metric"), (Some("0603"), Some(Mount::Smd)));
        assert_eq!(read("Lib:C_1608Metric"), (Some("0603"), Some(Mount::Smd)));
        assert_eq!(
            read("Connector_PinHeader_2.54mm:PinHeader_1x04_P2.54mm_Vertical"),
            (None, Some(Mount::Tht))
        );
        assert_eq!(
            read("Connector_PinHeader_1.27mm:PinHeader_1x04_P1.27mm_Vertical_SMD"),
            (None, Some(Mount::Smd))
        );
        assert_eq!(
            read("Package_TO_SOT_THT:TO-220-3_Vertical"),
            (None, Some(Mount::Tht))
        );
        assert_eq!(
            read("Package_TO_SOT_SMD:SOT-23-5"),
            (None, Some(Mount::Smd))
        );
        assert_eq!(
            read("Capacitor_THT:CP_Radial_D5.0mm_P2.00mm"),
            (None, Some(Mount::Tht))
        );
        assert_eq!(read("MyLib:Odd"), (None, None));
        assert!(Package::from_footprint("  ").is_none());
    }
}
