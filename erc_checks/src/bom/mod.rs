//! `BOM` - BOM hygiene

mod bom_10;
mod bom_5;
mod bom_6;
mod bom_9;

/// A value that is a decision not taken: starts or ends with `?` (`?`, `??k`, `4k7?`), or is `TBD` or `TODO`
/// as a word
pub(crate) fn placeholder(value: &str) -> bool {
    let v = value.trim();
    v.starts_with('?')
        || v.ends_with('?')
        || v.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w.eq_ignore_ascii_case("TBD") || w.eq_ignore_ascii_case("TODO"))
}

#[cfg(test)]
mod tests {
    use super::placeholder;

    #[test]
    fn placeholders() {
        for v in ["?", "??k", "4k7?", "TBD", "tbd", "10k TODO"] {
            assert!(placeholder(v), "{v}");
        }
        for v in ["", "4k7", "DNP", "TBDX", "STUDIO"] {
            assert!(!placeholder(v), "{v}");
        }
    }
}
