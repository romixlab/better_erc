//! Amplifier pins by name: the channels of op-amps and comparators (inputs `+`, `-`, `IN+`, `-INA`, `1IN-`, output
//! `OUT`, `OUTB`, KiCad's `~`), the sense inputs of current-sense amplifiers (`IN+`, `VIN-`, `SENSE+`, `CSN`), and
//! the names of analog inputs a divider may feed on purpose (`FB`, `ADC1`, `VREF`). What the ANA checks and IO-16
//! read until pin roles come from part data (NET-7).

use erc_model::{Board, EcadPinType, NetId, PartId, PinId};
use std::collections::BTreeMap;

/// What an amplifier pin is
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AmpPin {
    /// The non-inverting input
    InP,
    /// The inverting input
    InN,
    Out,
}

/// The role and channel (`A`, `1`, empty for one channel) of an op-amp or comparator pin, by name. `~` is KiCad's
/// name for an output, and only an output pin's: `etype` decides it.
pub fn amp_pin(name: &str, etype: Option<EcadPinType>) -> Option<(AmpPin, String)> {
    let n = normalise(name);
    if n == "~" || n.is_empty() {
        return matches!(
            etype,
            Some(EcadPinType::Output | EcadPinType::OpenCollector | EcadPinType::TriState)
        )
        .then(|| (AmpPin::Out, String::new()));
    }
    if matches!(etype, Some(EcadPinType::PowerIn | EcadPinType::PowerOut)) {
        return None;
    }
    split_channel(&n, |base| {
        Some(match base {
            "+" | "IN+" | "+IN" | "INP" | "VIN+" | "+VIN" | "NONINV" => AmpPin::InP,
            "-" | "IN-" | "-IN" | "INN" | "VIN-" | "-VIN" | "INV" => AmpPin::InN,
            "OUT" | "VOUT" | "OUTPUT" | "VO" => AmpPin::Out,
            _ => return None,
        })
    })
}

/// Whether a pin is a sense input of a current-sense amplifier, and which (`true` for the positive one), with
/// its channel
pub fn sense_pin(name: &str, etype: Option<EcadPinType>) -> Option<(bool, String)> {
    if matches!(etype, Some(EcadPinType::PowerIn | EcadPinType::PowerOut)) {
        return None;
    }
    split_channel(&normalise(name), |base| {
        Some(match base {
            "IN+" | "+IN" | "INP" | "VIN+" | "+VIN" | "SENSE+" | "SENSEP" | "SNS+" | "SNSP"
            | "ISENSE+" | "ISNS+" | "ISENP" | "CS+" | "CSP" | "RS+" => true,
            "IN-" | "-IN" | "INN" | "VIN-" | "-VIN" | "SENSE-" | "SENSEN" | "SNS-" | "SNSN"
            | "ISENSE-" | "ISNS-" | "ISENN" | "CS-" | "CSN" | "RS-" => false,
            _ => return None,
        })
    })
}

/// A pin name of an analog input or a setting pin that a divider feeds on purpose: feedback (`FB`, `VFB`, `ADJ`),
/// sense, reference, ADC inputs (`ADC1`, `AIN0`, `PA0/ADC1_IN0`), threshold and limit pins (`UVLO`, `ILIM`,
/// `RT`, `ISET`), and amplifier inputs
pub fn is_analog_input(name: &str) -> bool {
    if amp_pin(name, None).is_some_and(|(r, _)| r != AmpPin::Out) || sense_pin(name, None).is_some()
    {
        return true;
    }
    let plain = name.replace("~{", "").replace('}', "").to_uppercase();
    plain
        .split(['/', '_', '-', ' ', '(', ')', '.', ','])
        .filter(|w| !w.is_empty())
        .any(|w| {
            let base = w.trim_end_matches(|c: char| c.is_ascii_digit());
            matches!(
                base,
                "FB" | "VFB"
                    | "ADJ"
                    | "SENSE"
                    | "VSENSE"
                    | "ISENSE"
                    | "REF"
                    | "VREF"
                    | "REFIN"
                    | "UVLO"
                    | "OVP"
                    | "OV"
                    | "UV"
                    | "ILIM"
                    | "ISET"
                    | "IPROG"
                    | "PROG"
                    | "RT"
                    | "TRIP"
                    | "VSET"
                    | "ADC"
                    | "AIN"
                    | "AN"
                    | "ANA"
                    | "ANALOG"
                    | "VIN"
                    | "VINP"
                    | "VINN"
                    | "INP"
                    | "INN"
                    | "VCOMP"
                    | "COMP"
            ) && (base != "AN" || base.len() < w.len())
        })
}

/// One channel of an op-amp or comparator: its pins by role (a pin the symbol does not have is `None`)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmpUnit {
    pub part: PartId,
    /// The symbol unit and the channel the names give (`(2, "")`, `(1, "B")`)
    pub unit: u32,
    pub channel: String,
    pub inp: Option<PinId>,
    pub inn: Option<PinId>,
    pub out: Option<PinId>,
}

impl AmpUnit {
    /// `U1`, `U1 unit 2`, `U1 channel B`
    pub fn label(&self, board: &Board) -> String {
        let d = &board.part(self.part).designator;
        let units = board
            .part(self.part)
            .pins
            .iter()
            .map(|p| board.pin(*p).unit)
            .filter(|u| *u > 0)
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        if !self.channel.is_empty() {
            format!("{d} channel {}", self.channel)
        } else if units > 1 {
            format!("{d} unit {}", self.unit)
        } else {
            d.clone()
        }
    }
}

/// The channels of an op-amp or comparator, by symbol unit and the channel its pin names give, each with at least
/// one input
pub fn amp_units(board: &Board, part: PartId) -> Vec<AmpUnit> {
    let mut units: BTreeMap<(u32, String), AmpUnit> = BTreeMap::new();
    for pin in &board.part(part).pins {
        let p = board.pin(*pin);
        let Some((role, channel)) = amp_pin(&p.name, p.etype.get().copied()) else {
            continue;
        };
        let u = units
            .entry((p.unit, channel.clone()))
            .or_insert_with(|| AmpUnit {
                part,
                unit: p.unit,
                channel,
                inp: None,
                inn: None,
                out: None,
            });
        let slot = match role {
            AmpPin::InP => &mut u.inp,
            AmpPin::InN => &mut u.inn,
            AmpPin::Out => &mut u.out,
        };
        slot.get_or_insert(*pin);
    }
    units
        .into_values()
        .filter(|u| u.inp.is_some() || u.inn.is_some())
        .collect()
}

/// The supply nets of an amplifier: the nets of its power-input pins named as a positive supply (`V+`, `VCC`,
/// `VDD`, `+VS`) and as a negative supply or ground (`V-`, `VEE`, `VSS`, `GND`, `-VS`)
pub fn amp_supplies(board: &Board, part: PartId) -> (Vec<NetId>, Vec<NetId>) {
    let (mut pos, mut neg) = (Vec::new(), Vec::new());
    for pin in &board.part(part).pins {
        let p = board.pin(*pin);
        if p.etype.get() != Some(&EcadPinType::PowerIn) {
            continue;
        }
        let Some(net) = p.net else { continue };
        let n = normalise(&p.name);
        if matches!(
            n.as_str(),
            "V+" | "+VS" | "VS+" | "VCC" | "VDD" | "+V" | "VCC+" | "AVDD"
        ) {
            pos.push(net);
        } else if matches!(
            n.as_str(),
            "V-" | "-VS" | "VS-" | "VEE" | "VSS" | "GND" | "-V" | "AGND" | "AVSS"
        ) {
            neg.push(net);
        }
    }
    pos.sort();
    pos.dedup();
    neg.sort();
    neg.dedup();
    (pos, neg)
}

/// Upper case, the overbar and spaces out, `−` as `-`
fn normalise(name: &str) -> String {
    name.replace("~{", "")
        .replace(['}', ' ', '_'], "")
        .replace('−', "-")
        .to_uppercase()
}

/// A name read as `base` with an optional channel: leading digits (`1IN-`), a letter A-D or digits after `IN` or
/// `OUT` (`INA+`, `OUT2`), or a trailing letter A-D or digits (`+INA`, `IN+1`)
fn split_channel<T>(n: &str, base: impl Fn(&str) -> Option<T>) -> Option<(T, String)> {
    if let Some(t) = base(n) {
        return Some((t, String::new()));
    }
    let is_channel = |s: &str| {
        !s.is_empty()
            && (s.chars().all(|c| c.is_ascii_digit())
                || (s.len() == 1 && matches!(s, "A" | "B" | "C" | "D")))
    };
    // Leading digits
    let digits = n.len() - n.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits > 0
        && let Some(t) = base(&n[digits..])
    {
        return Some((t, n[..digits].to_string()));
    }
    // Trailing letter or digits
    for cut in 1..n.len().min(3) {
        let (head, tail) = n.split_at(n.len() - cut);
        if is_channel(tail)
            && let Some(t) = base(head)
        {
            return Some((t, tail.to_string()));
        }
    }
    // After IN or OUT
    for key in ["IN", "OUT"] {
        if let Some(at) = n.find(key) {
            let after = at + key.len();
            for cut in 1..=2 {
                if after + cut > n.len() {
                    break;
                }
                let channel = &n[after..after + cut];
                if is_channel(channel) {
                    let rest = format!("{}{}", &n[..after], &n[after + cut..]);
                    if let Some(t) = base(&rest) {
                        return Some((t, channel.to_string()));
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::AmpPin::*;
    use super::*;

    type Row = (
        &'static str,
        Option<EcadPinType>,
        Option<(AmpPin, &'static str)>,
    );

    #[test]
    fn amp_pins() {
        let out = Some(EcadPinType::Output);
        let table: &[Row] = &[
            ("+", Some(EcadPinType::Input), Some((InP, ""))),
            ("-", Some(EcadPinType::Input), Some((InN, ""))),
            ("~", out, Some((Out, ""))),
            ("~", Some(EcadPinType::Passive), None),
            ("IN+", None, Some((InP, ""))),
            ("-INA", None, Some((InN, "A"))),
            ("+INB", None, Some((InP, "B"))),
            ("1IN-", None, Some((InN, "1"))),
            ("2OUT", None, Some((Out, "2"))),
            ("OUTA", None, Some((Out, "A"))),
            ("INA+", None, Some((InP, "A"))),
            ("IN1-", None, Some((InN, "1"))),
            ("OUT", None, Some((Out, ""))),
            ("V+", Some(EcadPinType::PowerIn), None),
            ("V-", Some(EcadPinType::PowerIn), None),
            ("VDD", None, None),
            ("PA0", None, None),
        ];
        for (name, etype, want) in table {
            let got = amp_pin(name, *etype);
            let want = want.map(|(r, c)| (r, c.to_string()));
            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn sense_pins() {
        assert_eq!(sense_pin("IN+", None), Some((true, String::new())));
        assert_eq!(sense_pin("VIN-", None), Some((false, String::new())));
        assert_eq!(sense_pin("CSN", None), Some((false, String::new())));
        assert_eq!(sense_pin("SENSE+", None), Some((true, String::new())));
        assert_eq!(sense_pin("OUT", None), None);
        assert_eq!(sense_pin("+", None), None);
    }

    #[test]
    fn analog_inputs() {
        for name in [
            "FB",
            "VFB",
            "PA0/ADC1_IN0",
            "AIN3",
            "AN1",
            "VREF",
            "~{UVLO}",
            "-",
            "IN+",
        ] {
            assert!(is_analog_input(name), "{name}");
        }
        for name in ["EN", "PA0", "BOOT0", "SDA", "RESET", "ANT"] {
            assert!(!is_analog_input(name), "{name}");
        }
    }
}
