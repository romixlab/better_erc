//! Connector pins by what they are for: shell, shield and mounting pins, the contacts of a Type-C receptacle or
//! plug by number (`A5` is CC1) or name (`CC1`, `VBUS`), and whether a part is a plug. What the USB and CONN checks
//! read off a connector symbol.

use erc_model::{Board, PartId, PinId, PinInst};

/// A shell, shield or mounting pin of a connector: named `SHIELD`, `SHELL`, `SH`, `MP`, `MH`, `MountPin`, `CASE`
/// (with a number or not, `SHIELD2`, `MP1`), or numbered like one (`S1`, `SH2`, `MP`) with no name of its own
pub fn is_shell_pin(pin: &PinInst) -> bool {
    let name = pin.name.to_uppercase().replace(['_', ' ', '-'], "");
    let stem = name.trim_end_matches(|c: char| c.is_ascii_digit());
    if matches!(
        stem,
        "SHIELD"
            | "SHELL"
            | "SH"
            | "MP"
            | "MH"
            | "MOUNTPIN"
            | "MOUNTINGPIN"
            | "MOUNT"
            | "MOUNTING"
            | "CASE"
            | "EH"
    ) {
        return true;
    }
    let unnamed = name.is_empty() || name == "~" || name == pin.number.to_uppercase();
    let number = pin.number.to_uppercase();
    let nstem = number.trim_end_matches(|c: char| c.is_ascii_digit());
    unnamed && (matches!(nstem, "SH" | "MP" | "MH") || (nstem == "S" && nstem.len() < number.len()))
}

/// Whether a connector part is a plug (a dongle's or captive cable's end), not a receptacle: its symbol name or
/// description says `plug` and not `receptacle`
pub fn is_plug(board: &Board, part: PartId) -> bool {
    let p = board.part(part);
    let symbol = p.symbol.map(|s| &board.symbols[s.index()]);
    let text = format!(
        "{} {} {}",
        p.lib_id,
        symbol.map_or("", |s| s.description.as_str()),
        symbol.map_or("", |s| s.keywords.as_str())
    )
    .to_lowercase();
    let words: Vec<&str> = text
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    words.contains(&"plug") && !words.contains(&"receptacle")
}

/// What a Type-C contact is for
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TypeCPin {
    Gnd,
    Vbus,
    /// `A5`: CC1 on a receptacle, the one CC contact of a plug
    Cc1,
    /// `B5`: CC2 on a receptacle, VCONN on a plug
    Cc2,
    /// USB 2.0 D+ of row A (`A6`) or B (`B6`)
    Dp {
        row_b: bool,
    },
    /// USB 2.0 D- of row A (`A7`) or B (`B7`)
    Dn {
        row_b: bool,
    },
    Sbu {
        row_b: bool,
    },
    /// A SuperSpeed contact (`A2`, `A3`, `A10`, `A11` and their B twins)
    SuperSpeed,
    Shell,
}

/// The Type-C role of a connector pin: by its contact number (`A1`..`B12`) where the symbol numbers them so, else by
/// its name (`GND`, `VBUS`, `CC1`, `CC2`, `CC`, `VCONN`, `DP1`, `DN2`, `SBU1`, `SHIELD`)
pub fn type_c_pin(pin: &PinInst) -> Option<TypeCPin> {
    use TypeCPin as T;
    if is_shell_pin(pin) {
        return Some(T::Shell);
    }
    let number = pin.number.to_uppercase();
    if let Some((row_b, contact)) = number
        .strip_prefix('A')
        .map(|c| (false, c))
        .or_else(|| number.strip_prefix('B').map(|c| (true, c)))
        && let Ok(contact) = contact.parse::<u32>()
    {
        return Some(match contact {
            1 | 12 => T::Gnd,
            4 | 9 => T::Vbus,
            5 if row_b => T::Cc2,
            5 => T::Cc1,
            6 => T::Dp { row_b },
            7 => T::Dn { row_b },
            8 => T::Sbu { row_b },
            2 | 3 | 10 | 11 => T::SuperSpeed,
            _ => return None,
        });
    }
    let name = pin.name.to_uppercase().replace(['_', ' '], "");
    Some(match name.as_str() {
        "GND" => T::Gnd,
        "VBUS" => T::Vbus,
        "CC1" | "CC" => T::Cc1,
        "CC2" | "VCONN" => T::Cc2,
        "DP1" | "D1+" => T::Dp { row_b: false },
        "DP2" | "D2+" => T::Dp { row_b: true },
        "DN1" | "D1-" => T::Dn { row_b: false },
        "DN2" | "D2-" => T::Dn { row_b: true },
        "SBU1" => T::Sbu { row_b: false },
        "SBU2" => T::Sbu { row_b: true },
        _ => return None,
    })
}

/// The pins of a Type-C connector with the role `role`
pub fn type_c_pins(board: &Board, part: PartId, role: TypeCPin) -> Vec<PinId> {
    board
        .part(part)
        .pins
        .iter()
        .copied()
        .filter(|p| type_c_pin(board.pin(*p)) == Some(role))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use erc_model::{Facts, PartId};

    fn pin(number: &str, name: &str) -> PinInst {
        PinInst {
            part: PartId::from_index(0),
            number: number.into(),
            name: name.into(),
            unit: 1,
            etype: Facts::default(),
            hidden: false,
            net: None,
            no_connect: false,
            alt: None,
        }
    }

    #[test]
    fn shell_pins() {
        for (number, name) in [
            ("S1", "SHIELD"),
            ("MP", "MountPin"),
            ("11", "SHELL"),
            ("13", "SH2"),
            ("S2", ""),
            ("MP1", "~"),
        ] {
            assert!(is_shell_pin(&pin(number, name)), "{number} {name}");
        }
        for (number, name) in [("1", "VBUS"), ("S1", "SDA"), ("S", "~"), ("2", "SHDN")] {
            assert!(!is_shell_pin(&pin(number, name)), "{number} {name}");
        }
    }

    #[test]
    fn type_c_contacts() {
        use TypeCPin as T;
        for (number, name, role) in [
            ("A1", "GND", T::Gnd),
            ("B9", "VBUS", T::Vbus),
            ("A5", "CC1", T::Cc1),
            ("B5", "VCONN", T::Cc2),
            ("A6", "D+", T::Dp { row_b: false }),
            ("B7", "D-", T::Dn { row_b: true }),
            ("B8", "SBU2", T::Sbu { row_b: true }),
            ("A2", "TX1+", T::SuperSpeed),
            ("S1", "SHIELD", T::Shell),
            ("3", "CC2", T::Cc2),
            ("7", "DN1", T::Dn { row_b: false }),
        ] {
            assert_eq!(
                type_c_pin(&pin(number, name)),
                Some(role),
                "{number} {name}"
            );
        }
        assert_eq!(type_c_pin(&pin("4", "D+")), None);
    }
}
