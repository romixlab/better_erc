//! The fact keys a refresh asks a part source for: what the component checks (BOM-12, LIB-6, RATE-1, RATE-2,
//! RATE-9, RATE-15) and the footprint review compare. The names are mx3's curated parameter keys, the ones MPN
//! schemes set (`mpn_schemes` keys), so a decoded value and a supplier's land on the same key.
//!
//! A key is added here in the commit of the first check that reads it.

/// One key and what reads it
#[derive(Clone, Copy, Debug)]
pub struct FactKey {
    pub key: &'static str,
    /// The unit of a number (`F`), empty for a word or a tolerance
    pub unit: &'static str,
    /// The checks that read it
    pub used_by: &'static [&'static str],
}

/// Every key a refresh asks for, sorted by key
pub const FACT_KEYS: &[FactKey] = &[
    FactKey {
        key: "capacitance",
        unit: "F",
        used_by: &["BOM-12"],
    },
    FactKey {
        key: "case_eia",
        unit: "",
        used_by: &["LIB-6"],
    },
    FactKey {
        key: "dielectric",
        unit: "",
        used_by: &["BOM-12", "RATE-1"],
    },
    FactKey {
        key: "inductance",
        unit: "H",
        used_by: &["BOM-12"],
    },
    FactKey {
        key: "power",
        unit: "W",
        used_by: &["RATE-2"],
    },
    FactKey {
        key: "rated_voltage",
        unit: "V",
        used_by: &["RATE-1", "RATE-9"],
    },
    FactKey {
        key: "resistance",
        unit: "Ω",
        used_by: &["BOM-12"],
    },
    FactKey {
        key: "saturation_current",
        unit: "A",
        used_by: &["RATE-15"],
    },
    FactKey {
        key: "technology",
        unit: "",
        used_by: &["RATE-1", "RATE-2"],
    },
    FactKey {
        key: "tolerance",
        unit: "",
        used_by: &["BOM-12"],
    },
];

/// The key names, in [`FACT_KEYS`] order
pub fn names() -> Vec<String> {
    FACT_KEYS.iter().map(|k| k.key.to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::FACT_KEYS;

    #[test]
    fn sorted_and_unique() {
        for w in FACT_KEYS.windows(2) {
            assert!(w[0].key < w[1].key, "{} before {}", w[0].key, w[1].key);
        }
    }
}
