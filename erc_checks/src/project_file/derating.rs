//! `[derating]`: how far below its rating a part works, per part class and technology. A factor `k` asks for a
//! rating of at least `k` times the working value (`k = 1.5`: a 3.3 V rail needs a 5 V part).

use erc_model::Provenance;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The largest factor a project may set: anything above is a typo, not a margin
pub const FACTOR_MAX: f64 = 10.0;

/// One derating key: its name, default and what it covers
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeratingKey {
    /// The key under `[derating]` (`ceramic_capacitor_v`)
    pub name: &'static str,
    pub default: f64,
    /// One line: which parts and which rating
    pub doc: &'static str,
}

macro_rules! derating {
    ($($(#[doc = $doc:literal])+ $name:ident = $default:literal,)+) => {
        /// The derating factors of a project: each a rating over the working value, the project's or the default
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        pub struct Derating {
            $($(#[doc = $doc])+ pub $name: f64,)+
            /// The keys the project file sets (the rest hold their default)
            #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
            pub set: BTreeSet<String>,
            /// The project file, for the provenance of a set factor
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub file: Option<String>,
        }

        impl Default for Derating {
            fn default() -> Derating {
                Derating {
                    $($name: $default,)+
                    set: BTreeSet::new(),
                    file: None,
                }
            }
        }

        /// Every derating key with its default, in the order the docs list them
        pub const KEYS: &[DeratingKey] = &[
            $(DeratingKey { name: stringify!($name), default: $default, doc: concat!($($doc),+) },)+
        ];

        impl Derating {
            /// The factor of a key, `None` for a key that is not one
            pub fn get(&self, name: &str) -> Option<f64> {
                match name {
                    $(stringify!($name) => Some(self.$name),)+
                    _ => None,
                }
            }

            /// Sets a key; false for a key that is not one
            fn put(&mut self, name: &str, value: f64) -> bool {
                match name {
                    $(stringify!($name) => self.$name = value,)+
                    _ => return false,
                }
                self.set.insert(name.to_owned());
                true
            }
        }
    };
}

derating! {
    /// Ceramic capacitors (class I and II), rated voltage over working voltage
    ceramic_capacitor_v = 1.5,
    /// Aluminium electrolytic capacitors, rated voltage over working voltage
    electrolytic_capacitor_v = 1.25,
    /// Aluminium polymer capacitors, rated voltage over working voltage
    polymer_capacitor_v = 1.25,
    /// Tantalum capacitors with an MnO2 cathode, rated voltage over working voltage
    tantalum_capacitor_v = 2.0,
    /// Tantalum polymer capacitors, rated voltage over working voltage
    tantalum_polymer_capacitor_v = 1.25,
    /// Film capacitors, rated voltage over working voltage
    film_capacitor_v = 1.25,
    /// Resistors, rated power over dissipated power
    resistor_p = 2.0,
    /// Resistors, limiting element voltage over working voltage
    resistor_v = 1.25,
    /// Inductors, saturation current over peak current
    inductor_isat = 1.3,
    /// Inductors, rated (heating) current over RMS current
    inductor_irms = 1.25,
    /// Ferrite beads, rated current over working current
    ferrite_bead_i = 1.25,
    /// Diodes, rated reverse voltage over working reverse voltage
    diode_vr = 1.25,
    /// MOSFETs, rated drain-source voltage over working voltage
    mosfet_vds = 1.25,
}

impl Derating {
    /// Where a factor comes from, for a finding's facts: the project file, or the default
    pub fn source(&self, name: &str) -> Provenance {
        match &self.file {
            Some(file) if self.set.contains(name) => Provenance::ProjectFile {
                file: file.clone(),
                key: format!("derating.{name}"),
            },
            _ => Provenance::Derived {
                rule: format!("default derating.{name}"),
            },
        }
    }

    /// Sets a factor from the project file; the error says what is wrong with the value
    pub(crate) fn set_checked(&mut self, name: &str, value: f64) -> Result<(), SetError> {
        if !(1.0..=FACTOR_MAX).contains(&value) {
            return Err(SetError::Range);
        }
        if self.put(name, value) {
            Ok(())
        } else {
            Err(SetError::Unknown)
        }
    }

    /// `other`'s set factors over these
    pub(crate) fn overlay(&mut self, other: &Derating) {
        for name in &other.set {
            let v = other.get(name).expect("a set key is a derating key");
            self.put(name, v);
        }
        if other.file.is_some() {
            self.file.clone_from(&other.file);
        }
    }
}

/// Why a derating factor was refused
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetError {
    /// Not a derating key
    Unknown,
    /// Below 1 or above [`FACTOR_MAX`]
    Range,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_table() {
        let d = Derating::default();
        for k in KEYS {
            assert_eq!(d.get(k.name), Some(k.default), "{}", k.name);
            assert!(!k.doc.is_empty(), "{} has a doc line", k.name);
        }
        assert_eq!(d.get("nope"), None);
    }

    #[test]
    fn a_set_factor_names_the_file() {
        let mut d = Derating {
            file: Some("b.berc.toml".into()),
            ..Derating::default()
        };
        d.set_checked("resistor_p", 3.0).unwrap();
        assert_eq!(d.resistor_p, 3.0);
        assert_eq!(
            d.source("resistor_p"),
            Provenance::ProjectFile {
                file: "b.berc.toml".into(),
                key: "derating.resistor_p".into()
            }
        );
        assert!(matches!(
            d.source("inductor_isat"),
            Provenance::Derived { .. }
        ));
        assert_eq!(d.set_checked("resistor_p", 0.5), Err(SetError::Range));
        assert_eq!(d.set_checked("resistr_p", 2.0), Err(SetError::Unknown));
    }
}
