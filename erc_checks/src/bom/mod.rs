//! `BOM` - BOM hygiene

mod bom_1;
mod bom_10;
mod bom_11;
mod bom_12;
mod bom_14;
mod bom_17;
mod bom_20;
mod bom_3;
mod bom_4;
mod bom_5;
mod bom_6;
mod bom_8;
mod bom_9;

pub(crate) use erc_parts::mpn::mpns;

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
    use super::placeholder;

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
}
