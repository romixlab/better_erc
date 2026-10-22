//! What a name says about its signal beyond the marker: the polarity with the sense of its function word (LOGIC-8)
//! and the interface signals it carries (MUX-1).
//!
//! - [`polarity`]: active low or high (the marker of [`active_low`]) and the function word with its [`Sense`]:
//!   asserting `EN` or `PG` lets something run, asserting `SHDN`, `RESET` or `FAULT` stops it, `INT` is an event.
//! - [`interface_signals`]: the I2C, SPI, UART, USB and CAN signals of a pin over every name it has (`PA9/USART1_TX`,
//!   its alternates), [`name_signals`] of one name (`I2C1_SCL`).

use super::{Uart, active_low, base, uart_pin, words};
use serde::{Deserialize, Serialize};

/// What asserting a function does
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sense {
    /// Lets something run or says it is good: `EN`, `CS`, `PG`, `READY`
    Go,
    /// Stops, resets or reports a fault: `SHDN`, `RESET`, `PD`, `FAULT`
    Stop,
    /// An event with no run or stop meaning: `INT`, `IRQ`, `ALERT`
    Event,
}

/// What a name says about its signal's polarity
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Polarity {
    pub active_low: bool,
    /// The function word that gives the name its meaning (`EN` in `VDD_EN`)
    pub word: Option<&'static str>,
    pub sense: Option<Sense>,
}

/// Function words with their sense
const WORDS: &[(&str, Sense)] = &[
    ("EN", Sense::Go),
    ("ENA", Sense::Go),
    ("ENABLE", Sense::Go),
    ("CE", Sense::Go),
    ("OE", Sense::Go),
    ("WE", Sense::Go),
    ("CS", Sense::Go),
    ("SS", Sense::Go),
    ("ON", Sense::Go),
    ("RUN", Sense::Go),
    ("WAKE", Sense::Go),
    ("PG", Sense::Go),
    ("PGOOD", Sense::Go),
    ("PWRGD", Sense::Go),
    ("PWROK", Sense::Go),
    ("POK", Sense::Go),
    ("READY", Sense::Go),
    ("RDY", Sense::Go),
    ("DONE", Sense::Go),
    ("VALID", Sense::Go),
    ("LOCK", Sense::Go),
    ("SHDN", Sense::Stop),
    ("SHUTDOWN", Sense::Stop),
    ("PD", Sense::Stop),
    ("PWRDN", Sense::Stop),
    ("STBY", Sense::Stop),
    ("STANDBY", Sense::Stop),
    ("SLEEP", Sense::Stop),
    ("DIS", Sense::Stop),
    ("DISABLE", Sense::Stop),
    ("OFF", Sense::Stop),
    ("RESET", Sense::Stop),
    ("RST", Sense::Stop),
    ("MR", Sense::Stop),
    ("CLR", Sense::Stop),
    ("HOLD", Sense::Stop),
    ("WP", Sense::Stop),
    ("FAULT", Sense::Stop),
    ("FLT", Sense::Stop),
    ("ERR", Sense::Stop),
    ("ERROR", Sense::Stop),
    ("INT", Sense::Event),
    ("IRQ", Sense::Event),
    ("ALERT", Sense::Event),
];

fn sense_word(w: &str) -> Option<(&'static str, Sense)> {
    let w = w.trim_end_matches(|c: char| c.is_ascii_digit());
    WORDS.iter().find(|(x, _)| *x == w).copied()
}

/// The polarity a pin or net name states: `None` when it names no function word and carries no active-low marker
/// (`PA3`, `OUT`, `Net-(R1-Pad2)`). A pin name with alternate functions (`PA4/SPI1_NSS`) states none, nor does a
/// lone `_N` without a function word (the negative half of a pair as likely). Net names are read without their
/// sheet path: pass a pin name as it is, a net name through [`base`].
pub fn polarity(name: &str) -> Option<Polarity> {
    let name = name.trim();
    if name.is_empty() || name[1..].contains('/') {
        return None;
    }
    let low = active_low(name);
    let rest = low.as_ref().map_or(name, |a| a.base.as_str());
    let found = words(rest).iter().find_map(|w| sense_word(w));
    match (&low, found) {
        (None, None) => None,
        (Some(a), None) if matches!(a.style, super::LowStyle::SuffixN) => None,
        _ => Some(Polarity {
            active_low: low.is_some(),
            word: found.map(|(w, _)| w),
            sense: found.map(|(_, s)| s),
        }),
    }
}

/// Whether a signal named `from` driving an input named `to` inverts the meaning: active-low into active-high or
/// the reverse, for words of the same sense (`PGOOD` into `~EN`, `RESET` into `~RST`, `INT_N` into `IRQ`); for words
/// of opposite sense the markers must differ (`PG` into `~SHDN` shuts B down while A is not good, as meant). A word
/// of no known sense compares by marker alone.
pub fn mismatch(from: Polarity, to: Polarity) -> bool {
    let opposite = matches!(
        (from.sense, to.sense),
        (Some(Sense::Go), Some(Sense::Stop)) | (Some(Sense::Stop), Some(Sense::Go))
    );
    (from.active_low != to.active_low) != opposite
}

/// An interface a signal belongs to
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Interface {
    I2c,
    Spi,
    Uart,
    Usb,
    Can,
}

/// One interface signal
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Signal {
    Scl,
    Sda,
    Sck,
    /// Controller out (`MOSI`, `COPI`, `SIMO`)
    Mosi,
    /// Controller in (`MISO`, `CIPO`, `SOMI`)
    Miso,
    /// SPI data named from the part's own side (`SDI`, `SDO`, `DIN`, `DOUT`): either direction on a bus net
    SpiData,
    Cs,
    Tx,
    Rx,
    UsbDp,
    UsbDm,
    CanH,
    CanL,
}

impl Signal {
    pub fn interface(self) -> Interface {
        match self {
            Signal::Scl | Signal::Sda => Interface::I2c,
            Signal::Sck | Signal::Mosi | Signal::Miso | Signal::SpiData | Signal::Cs => {
                Interface::Spi
            }
            Signal::Tx | Signal::Rx => Interface::Uart,
            Signal::UsbDp | Signal::UsbDm => Interface::Usb,
            Signal::CanH | Signal::CanL => Interface::Can,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Signal::Scl => "SCL",
            Signal::Sda => "SDA",
            Signal::Sck => "SCK",
            Signal::Mosi => "MOSI",
            Signal::Miso => "MISO",
            Signal::SpiData => "SPI data (SDI/SDO)",
            Signal::Cs => "CS",
            Signal::Tx => "TX",
            Signal::Rx => "RX",
            Signal::UsbDp => "D+",
            Signal::UsbDm => "D-",
            Signal::CanH => "CANH",
            Signal::CanL => "CANL",
        }
    }

    /// Whether a net carrying `self` may land on a pin named `pin`: the same signal, or SPI data named from the
    /// part's side on a MOSI or MISO net
    pub fn fits(self, pin: Signal) -> bool {
        self == pin
            || matches!(
                (self, pin),
                (
                    Signal::Mosi | Signal::Miso | Signal::SpiData,
                    Signal::SpiData
                ) | (Signal::SpiData, Signal::Mosi | Signal::Miso)
            )
    }
}

fn signal_word(w: &str) -> Option<Signal> {
    Some(match w.trim_end_matches(|c: char| c.is_ascii_digit()) {
        "SCL" => Signal::Scl,
        "SDA" => Signal::Sda,
        "SCK" | "SCLK" => Signal::Sck,
        "MOSI" | "COPI" | "SIMO" => Signal::Mosi,
        "MISO" | "CIPO" | "SOMI" => Signal::Miso,
        "SDI" | "SDO" | "DIN" | "DOUT" => Signal::SpiData,
        "CS" | "NCS" | "CSN" | "SS" | "NSS" => Signal::Cs,
        // `UD+`/`UD-`: the CH340's USB pins
        "D+" | "UD+" => Signal::UsbDp,
        "D-" | "UD-" => Signal::UsbDm,
        "CANH" => Signal::CanH,
        "CANL" => Signal::CanL,
        _ => return None,
    })
}

/// The interface signals one name carries (`I2C1_SCL`: SCL; `SPI_MOSI`: MOSI; `USB_DP`: D+; `CAN_H`: CANH; UART
/// lines by [`uart_pin`]). `DP`/`DM`/`DN` and `D_P`/`D_N` count as USB data only in a name with a `USB` word (`DP`
/// alone is DisplayPort). A net name is read through [`base`].
pub fn name_signals(name: &str) -> Vec<Signal> {
    let cleaned = base(name).replace(['~', '{', '}'], "");
    // `words` splits at `-`: keep `D+`/`D-` whole first
    let w: Vec<String> = cleaned
        .split(['_', '.', ' '])
        .filter(|x| !x.is_empty())
        .flat_map(|x| {
            if x.ends_with(['+', '-']) && x.len() <= 3 {
                vec![x.to_uppercase()]
            } else {
                words(x)
            }
        })
        .collect();
    // `OTG_FS_DP`: an STM32's OTG peripheral is USB too
    let usb = w.iter().any(|x| {
        matches!(
            x.trim_end_matches(|c: char| c.is_ascii_digit()),
            "USB" | "OTG"
        )
    });
    let mut out = Vec::new();
    let mut push = |s: Signal| {
        if !out.contains(&s) {
            out.push(s);
        }
    };
    for (i, x) in w.iter().enumerate() {
        let next = w.get(i + 1).map(String::as_str);
        let sig = signal_word(x).or(match x.as_str() {
            "DP" | "USBDP" if usb || x == "USBDP" => Some(Signal::UsbDp),
            "DM" | "DN" | "USBDM" | "USBDN" if usb || x.starts_with("USB") => Some(Signal::UsbDm),
            "D" if usb => match next {
                Some("P" | "+") => Some(Signal::UsbDp),
                Some("N" | "M" | "-") => Some(Signal::UsbDm),
                _ => None,
            },
            "CAN" => match next {
                Some("H") => Some(Signal::CanH),
                Some("L") => Some(Signal::CanL),
                _ => None,
            },
            _ => None,
        });
        if let Some(s) = sig {
            push(s);
        }
    }
    match uart_pin(&cleaned) {
        Some(Uart::Tx) => push(Signal::Tx),
        Some(Uart::Rx) => push(Signal::Rx),
        None => {}
    }
    out
}

/// The interface signals of a pin over all its names: the name split at `/` (`PB6/I2C1_SCL/USART1_TX`), the
/// alternate functions of its symbol and the one chosen
pub fn interface_signals<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<Signal> {
    let mut out = Vec::new();
    for n in names {
        for f in n.split('/') {
            for s in name_signals(f) {
                if !out.contains(&s) {
                    out.push(s);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn low(name: &str) -> Option<bool> {
        polarity(name).map(|p| p.active_low)
    }

    #[test]
    fn polarities() {
        for n in [
            "~{RESET}",
            "~RESET",
            "nCS",
            "NRST",
            "OE#",
            "INT_N",
            "RESETn",
            "~{CS}",
            "PWR_~{EN}",
            "~{Q}",
        ] {
            assert_eq!(low(n), Some(true), "{n}");
        }
        for n in [
            "EN", "PGOOD", "VDD_EN", "PG_3V3", "RESET", "IRQ", "SHDN", "CS1",
        ] {
            assert_eq!(low(n), Some(false), "{n}");
        }
        for n in [
            "PA3",
            "OUT",
            "A0",
            "PA4/SPI1_NSS",
            "Net-(R1-Pad2)",
            "VDD",
            "LVDS_N",
        ] {
            assert_eq!(polarity(n), None, "{n}");
        }
        assert_eq!(polarity("VDD_EN").unwrap().word, Some("EN"));
        assert_eq!(polarity("~{SHDN}").unwrap().sense, Some(Sense::Stop));
    }

    #[test]
    fn mismatches() {
        let p = |n: &str| polarity(n).unwrap();
        assert!(mismatch(p("PGOOD"), p("~{EN}")));
        assert!(!mismatch(p("PG"), p("~{SHDN}")));
        assert!(mismatch(p("RESET"), p("~{RST}")));
        assert!(mismatch(p("INT_N"), p("IRQ")));
        assert!(!mismatch(p("~{INT}"), p("~{IRQ}")));
        assert!(!mismatch(p("~{FAULT}"), p("EN")));
        assert!(!mismatch(p("~{RESET}"), p("NRST")));
    }

    #[test]
    fn signals() {
        assert_eq!(name_signals("/MCU/I2C1_SCL"), [Signal::Scl]);
        assert_eq!(name_signals("SCL_3V3"), [Signal::Scl]);
        assert_eq!(name_signals("UART2_TXD"), [Signal::Tx]);
        assert_eq!(name_signals("USB_DP"), [Signal::UsbDp]);
        assert_eq!(name_signals("USB_D-"), [Signal::UsbDm]);
        assert_eq!(name_signals("USB1_D_N"), [Signal::UsbDm]);
        assert_eq!(name_signals("D+"), [Signal::UsbDp]);
        assert_eq!(name_signals("DP_AUX"), []);
        assert_eq!(name_signals("OTG_FS_DP"), [Signal::UsbDp]);
        assert_eq!(name_signals("UD-"), [Signal::UsbDm]);
        assert_eq!(name_signals("CAN_H"), [Signal::CanH]);
        assert_eq!(name_signals("~{CS}"), [Signal::Cs]);
        assert_eq!(name_signals("SPI_MOSI"), [Signal::Mosi]);
        assert_eq!(name_signals("CAN_TX"), []);
        assert_eq!(
            interface_signals(["PB6/I2C1_SCL/USART1_TX"]),
            [Signal::Scl, Signal::Tx]
        );
        assert!(Signal::Mosi.fits(Signal::SpiData));
        assert!(!Signal::Mosi.fits(Signal::Miso));
    }
}
