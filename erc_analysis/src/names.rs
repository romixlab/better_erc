//! The name grammar of nets and pins beyond power (NAME, WIRE and SHORT checks): active-low markers and their
//! styles, the halves of differential pairs, numbered families (`IO0`..`IO7`), the GPIO number a pin name gives
//! (`PA5`, `GPIO17`, `P0.12`), UART TX and RX pins, and the normalised form under which two names mean one net.
//! Names are read without their sheet path (`/MCU/~{RESET}` is `~{RESET}`).

use serde::{Deserialize, Serialize};

mod signals;
pub use signals::{
    Interface, Polarity, Sense, Signal, interface_signals, mismatch, name_signals, polarity,
};

/// The last segment of a net name: `/MCU/SWDIO` is `SWDIO`
pub fn base(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

/// How a name marks an active-low signal
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LowStyle {
    /// KiCad overbar markup, `~{RESET}`
    Overbar,
    /// A leading tilde, `~RESET`
    Tilde,
    /// `RESET_N`
    SuffixN,
    /// `RESET_n`
    SuffixLowerN,
    /// `nRESET`, `NRST`
    PrefixN,
    /// `RESET#`
    Hash,
    /// `RESETn`
    TrailingN,
}

impl LowStyle {
    /// How the style looks, for messages
    pub fn example(self) -> &'static str {
        match self {
            LowStyle::Overbar => "~{X}",
            LowStyle::Tilde => "~X",
            LowStyle::SuffixN => "X_N",
            LowStyle::SuffixLowerN => "X_n",
            LowStyle::PrefixN => "nX",
            LowStyle::Hash => "X#",
            LowStyle::TrailingN => "Xn",
        }
    }
}

/// An active-low name: its marker style and the name without the marker
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveLow {
    pub style: LowStyle,
    pub base: String,
}

/// Function words that an upper-case `N` in front of makes active low (`NRST`, `NCS`); a bare upper-case `N` is
/// part of most words, so only these
const LOW_WORDS: &[&str] = &[
    "RST", "RESET", "CS", "SS", "OE", "WE", "RE", "IRQ", "INT", "EN", "SHDN", "PD", "WP", "HOLD",
    "BOOT", "FAULT", "ALERT", "PWRDN", "SLEEP", "STBY", "CE", "WAIT", "BUSY", "RDY", "READY",
];

/// Whether `t` is a word an `n` or `N` prefix makes active low: a known function word, possibly with a number
/// or a suffix after an underscore (`nCS0`, `NRST_MCU`)
fn low_word(t: &str) -> bool {
    let head = t.split('_').next().unwrap_or(t);
    let head = head.trim_end_matches(|c: char| c.is_ascii_digit());
    LOW_WORDS.contains(&head)
}

/// The active-low marker of a net or pin name, if it has one: `~{RESET}`, `~RESET`, `RESET_N`, `RESET_n`,
/// `nRESET`, `NRST`, `RESET#`, `RESETn`. `_N` is also the negative half of a differential pair: callers that know
/// the pair (its `_P` exists) leave those out.
pub fn active_low(name: &str) -> Option<ActiveLow> {
    let b = base(name).trim();
    if b.is_empty() {
        return None;
    }
    let low = |style, base: &str| {
        Some(ActiveLow {
            style,
            base: base.to_string(),
        })
    };
    if let Some(i) = b.find("~{") {
        let inner: String = b[..i]
            .chars()
            .chain(b[i + 2..].chars().filter(|c| *c != '}'))
            .collect();
        return (!inner.is_empty()).then_some(ActiveLow {
            style: LowStyle::Overbar,
            base: inner,
        });
    }
    if let Some(rest) = b.strip_prefix('~') {
        return if rest.is_empty() {
            None
        } else {
            low(LowStyle::Tilde, rest.trim_end_matches('~'))
        };
    }
    if let Some(rest) = b.strip_suffix('#')
        && !rest.is_empty()
    {
        return low(LowStyle::Hash, rest);
    }
    for (suffix, style) in [("_N", LowStyle::SuffixN), ("_n", LowStyle::SuffixLowerN)] {
        if let Some(rest) = b.strip_suffix(suffix)
            && !rest.is_empty()
        {
            return low(style, rest);
        }
    }
    // `nCS`, `nRESET`: a lower-case n before an upper-case word
    let mut chars = b.chars();
    if chars.next() == Some('n') && chars.next().is_some_and(|c| c.is_ascii_uppercase()) {
        return low(LowStyle::PrefixN, &b[1..]);
    }
    // `NRST`, `NCS`: an upper-case N only before a known function word
    if let Some(rest) = b.strip_prefix('N')
        && low_word(rest)
    {
        return low(LowStyle::PrefixN, rest);
    }
    // `RESETn`, `CSn`: a lower-case n after an upper-case word of two letters or more
    if let Some(rest) = b.strip_suffix('n')
        && rest.len() >= 2
        && rest.chars().any(|c| c.is_ascii_uppercase())
        && rest
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return low(LowStyle::TrailingN, rest);
    }
    None
}

/// The positive or negative half of a differential pair
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Half {
    Pos,
    Neg,
}

/// How a name marks the half of a differential pair
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PairStyle {
    /// `USB_P` / `USB_N`
    UnderscorePN,
    /// `TX0P` / `TX0N`: only where both halves exist ([`pair_halves`])
    GluedPN,
    /// `USB+` / `USB-`, `D+` / `D-`
    PlusMinus,
    /// `USB_DP` / `USB_DM`
    DpDm,
    /// `CAN_H` / `CAN_L`, `CANH` / `CANL`
    HighLow,
}

impl PairStyle {
    pub fn example(self) -> &'static str {
        match self {
            PairStyle::UnderscorePN => "X_P/X_N",
            PairStyle::GluedPN => "XP/XN",
            PairStyle::PlusMinus => "X+/X-",
            PairStyle::DpDm => "X_DP/X_DM",
            PairStyle::HighLow => "X_H/X_L",
        }
    }
}

/// A name read as the half of a differential pair
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DiffHalf {
    /// The name without the half marker (`USB` for `USB_DP`, `USB_D` for `USB_D+`)
    pub base: String,
    pub half: Half,
    pub style: PairStyle,
}

impl DiffHalf {
    /// The key two halves of one pair share whatever their style: the base without separators and a trailing `D`
    /// (`USB_D+` and `USB_DM` both give `USB`)
    pub fn key(&self) -> String {
        let k: String = self
            .base
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_uppercase();
        k.strip_suffix('D').map_or(k.clone(), str::to_string)
    }
}

/// The differential half a name is in a style that says so on its own: `X_P`/`X_N`, `X+`/`X-`, `X_DP`/`X_DM`
/// (`DP`/`DM` alone too), `X_H`/`X_L`, `CANH`/`CANL`. A glued `XP`/`XN` is a pair only when both exist
/// ([`pair_halves`]). `_N` is also an active-low marker: a lone `_N` is not a pair by itself.
pub fn diff_half(name: &str) -> Option<DiffHalf> {
    let b = base(name).trim();
    let up = b.to_uppercase();
    let half = |base: &str, half, style| {
        Some(DiffHalf {
            base: base.to_string(),
            half,
            style,
        })
    };
    if let Some(rest) = b.strip_suffix('+')
        && !rest.is_empty()
    {
        return half(rest, Half::Pos, PairStyle::PlusMinus);
    }
    if let Some(rest) = b.strip_suffix('-')
        && !rest.is_empty()
    {
        return half(rest, Half::Neg, PairStyle::PlusMinus);
    }
    for (suffix, h) in [("DP", Half::Pos), ("DM", Half::Neg)] {
        if let Some(rest) = up.strip_suffix(suffix)
            && (rest.is_empty() || rest.ends_with(['_', '.']))
        {
            return half(rest.trim_end_matches(['_', '.']), h, PairStyle::DpDm);
        }
    }
    for (suffix, h, style) in [
        ("_P", Half::Pos, PairStyle::UnderscorePN),
        ("_N", Half::Neg, PairStyle::UnderscorePN),
        ("_H", Half::Pos, PairStyle::HighLow),
        ("_L", Half::Neg, PairStyle::HighLow),
    ] {
        if let Some(rest) = up.strip_suffix(suffix)
            && !rest.is_empty()
        {
            return half(&b[..rest.len()], h, style);
        }
    }
    for (word, h) in [("CANH", Half::Pos), ("CANL", Half::Neg)] {
        if up.ends_with(word) {
            return half(&b[..b.len() - 1], h, PairStyle::HighLow);
        }
    }
    None
}

/// The glued half `XP`/`XN` of a name, read only when the other half exists
fn glued_half(name: &str) -> Option<DiffHalf> {
    let b = base(name).trim();
    let (rest, h) = if let Some(r) = b.strip_suffix('P') {
        (r, Half::Pos)
    } else {
        (b.strip_suffix('N')?, Half::Neg)
    };
    // `TX0P`, `CLKP`: at least two characters before, the last alphanumeric
    (rest.len() >= 2 && rest.ends_with(|c: char| c.is_ascii_alphanumeric())).then(|| DiffHalf {
        base: rest.to_string(),
        half: h,
        style: PairStyle::GluedPN,
    })
}

/// A complete differential pair among names: the indices of its positive and negative half
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    pub pos: usize,
    pub neg: usize,
    pub style: PairStyle,
    pub base: String,
}

/// The differential pairs among `names` (net names with their sheet paths: halves pair only on the same path and
/// base), in every style, glued `XP`/`XN` included; and the halves left without a partner (in the styles that say
/// so on their own, without `_N`, which is as likely an active-low marker)
pub fn pair_halves(names: &[&str]) -> (Vec<Pair>, Vec<(usize, DiffHalf)>) {
    let dir = |n: &str| n.rsplit_once('/').map_or("", |(d, _)| d).to_string();
    let read: Vec<Option<DiffHalf>> = names.iter().map(|n| diff_half(n)).collect();
    let mut pairs = Vec::new();
    let mut paired = vec![false; names.len()];
    for glued in [false, true] {
        for i in 0..names.len() {
            if paired[i] {
                continue;
            }
            let Some(h) = (if glued {
                glued_half(names[i])
            } else {
                read[i].clone()
            }) else {
                continue;
            };
            if h.half != Half::Pos {
                continue;
            }
            let same_dir = |j: usize| dir(names[j]) == dir(names[i]);
            let partner = (0..names.len()).find(|j| {
                !paired[*j]
                    && same_dir(*j)
                    && (if glued {
                        glued_half(names[*j])
                    } else {
                        read[*j].clone()
                    })
                    .is_some_and(|o| o.half == Half::Neg && o.style == h.style && o.base == h.base)
            });
            if let Some(j) = partner {
                paired[i] = true;
                paired[j] = true;
                pairs.push(Pair {
                    pos: i,
                    neg: j,
                    style: h.style,
                    base: h.base.clone(),
                });
            }
        }
    }
    let lone = read
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !paired[*i])
        .filter_map(|(i, h)| Some((i, h?)))
        .filter(|(_, h)| h.style != PairStyle::UnderscorePN || h.half == Half::Pos)
        .collect();
    (pairs, lone)
}

/// A name with an index: `IO4` is (`IO`, 4, ``), `CH0_ADC` (`CH`, 0, `_ADC`); the last run of digits is the index
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Numbered {
    pub prefix: String,
    pub index: u32,
    pub suffix: String,
}

impl Numbered {
    /// The family the name is in: its sheet path, prefix and suffix (`/MCU/IO#`)
    pub fn family(&self, name: &str) -> String {
        let dir = name.rsplit_once('/').map_or("", |(d, _)| d);
        format!("{dir}/{}#{}", self.prefix, self.suffix)
    }
}

/// The index of a numbered name; a prefix of letters is needed (`IO4`, `LED_1`), a bare number is none
pub fn numbered(name: &str) -> Option<Numbered> {
    let b = base(name);
    let end = b.rfind(|c: char| c.is_ascii_digit())? + 1;
    let start = b[..end]
        .rfind(|c: char| !c.is_ascii_digit())
        .map_or(0, |i| i + 1);
    let prefix = &b[..start];
    if !prefix.chars().any(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    Some(Numbered {
        prefix: prefix.to_string(),
        index: b[start..end].parse().ok()?,
        suffix: b[end..].to_string(),
    })
}

/// A GPIO a pin name gives: the port (`A` for `PA5`, `0` for `P0.12`, empty for `GPIO17`) and the number
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Gpio {
    pub port: String,
    pub number: u32,
}

/// The GPIO of a pin name, from its first `/`-separated function that reads as one: `GPIO17`, `GP3`, `IO17`,
/// `PA5`, `P0.12`, `P1_3`, `PIO0_7` (`PA5/SPI1_SCK` is `PA5`)
pub fn gpio(pin_name: &str) -> Option<Gpio> {
    pin_name.split('/').find_map(|f| {
        let f = f.trim().to_uppercase();
        let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
        for prefix in ["GPIO", "GP", "IO"] {
            if let Some(n) = f.strip_prefix(prefix)
                && digits(n)
            {
                return Some(Gpio {
                    port: String::new(),
                    number: n.parse().ok()?,
                });
            }
        }
        let rest = f.strip_prefix("PIO").or_else(|| f.strip_prefix('P'))?;
        // `PA5`, `PB12`: a port letter
        let mut chars = rest.chars();
        let first = chars.next()?;
        if first.is_ascii_alphabetic() && digits(chars.as_str()) {
            return Some(Gpio {
                port: first.to_string(),
                number: chars.as_str().parse().ok()?,
            });
        }
        // `P0.12`, `P1_3`, `PIO0_7`
        let (port, n) = rest.split_once(['.', '_'])?;
        (digits(port) && digits(n)).then(|| Gpio {
            port: port.to_string(),
            number: n.parse().unwrap_or(0),
        })
    })
}

/// A UART line a pin name gives
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Uart {
    Tx,
    Rx,
}

/// Interfaces whose `TX`/`RX` are not a UART's
const NOT_UART: &[&str] = &[
    "CAN", "FDCAN", "RMII", "RGMII", "MII", "ETH", "SPI", "I2S", "SAI", "USB", "LIN", "SPDIF",
];

/// The UART line a word gives: `TX`, `TXD`, `TX1`, `U0TXD`, `UART1TX`
fn uart_word(t: &str) -> Option<Uart> {
    let t = t.trim_matches(|c: char| c.is_ascii_digit());
    let t = ["USART", "UART", "LPUART", "U"]
        .iter()
        .find_map(|p| {
            let r = t.strip_prefix(p)?;
            let r = r.trim_start_matches(|c: char| c.is_ascii_digit());
            (r != t).then_some(r)
        })
        .unwrap_or(t);
    match t.trim_end_matches(|c: char| c.is_ascii_digit()) {
        "TX" | "TXD" => Some(Uart::Tx),
        "RX" | "RXD" => Some(Uart::Rx),
        _ => None,
    }
}

/// The words of a name split at `_`, `-`, `.`, spaces and `/`, upper case
fn words(name: &str) -> Vec<String> {
    name.split(['_', '-', '.', ' ', '/'])
        .filter(|w| !w.is_empty())
        .map(str::to_uppercase)
        .collect()
}

/// The UART line a pin name gives, from any of its `/`-separated functions (`PA9/USART1_TX`, `TXD`, `U0TXD`,
/// `RXD0`); a CAN, Ethernet, SPI or USB `TX` is no UART line
pub fn uart_pin(pin_name: &str) -> Option<Uart> {
    pin_name.split('/').find_map(|f| {
        let w = words(f);
        if w.iter().any(|w| NOT_UART.iter().any(|n| w.starts_with(n))) {
            return None;
        }
        w.iter().find_map(|w| uart_word(w))
    })
}

/// The UART lines a net name mentions, in order (`MCU_TX_GPS_RX` is `[Tx, Rx]`)
pub fn uart_words(net_name: &str) -> Vec<Uart> {
    let w = words(base(net_name));
    if w.iter().any(|w| NOT_UART.iter().any(|n| w.starts_with(n))) {
        return Vec::new();
    }
    w.iter().filter_map(|w| uart_word(w)).collect()
}

/// A name normalised for "the same net meant twice" (WIRE-2): upper case, without separators (`_`, `.`, a `-`
/// between two characters, spaces), a leading `+` dropped, and every active-low marker written as one leading `!`
/// (`RESET_N`, `RESET_n`, `~{RESET}` and `RESET#` all give `!RESET`). A sign that makes a negative rail (`-5V`)
/// and the halves of pairs (`D+`, `D-`) stay.
pub fn normalised(name: &str) -> String {
    let b = base(name).trim();
    let (low, body) = match active_low(b) {
        Some(a) => (true, a.base),
        None => (false, b.to_string()),
    };
    let body = body.strip_prefix('+').unwrap_or(&body);
    let chars: Vec<char> = body.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        let inner = i > 0 && i + 1 < chars.len();
        match c {
            '_' | '.' | ' ' => {}
            '-' if inner => {}
            _ => out.extend(c.to_uppercase()),
        }
    }
    if low { format!("!{out}") } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(n: &str) -> Option<LowStyle> {
        active_low(n).map(|a| a.style)
    }

    #[test]
    fn active_low_markers() {
        assert_eq!(style("/MCU/~{RESET}"), Some(LowStyle::Overbar));
        assert_eq!(active_low("~{RESET}").unwrap().base, "RESET");
        assert_eq!(active_low("EXP.~{OE}").unwrap().base, "EXP.OE");
        assert_eq!(style("~HUB_RESET"), Some(LowStyle::Tilde));
        assert_eq!(style("SPI_CS_N"), Some(LowStyle::SuffixN));
        assert_eq!(style("RESET_n"), Some(LowStyle::SuffixLowerN));
        assert_eq!(style("nPWR_FAULT"), Some(LowStyle::PrefixN));
        assert_eq!(style("NRST"), Some(LowStyle::PrefixN));
        assert_eq!(style("OE#"), Some(LowStyle::Hash));
        assert_eq!(style("RESETn"), Some(LowStyle::TrailingN));
        assert_eq!(style("CSn"), Some(LowStyle::TrailingN));
        assert_eq!(style("NET"), None);
        assert_eq!(style("RESET"), None);
        assert_eq!(style("EN"), None);
        assert_eq!(style("Pin"), None);
        assert_eq!(style("~"), None);
        assert_eq!(style("NC"), None);
    }

    #[test]
    fn differential_halves() {
        let h = |n: &str| diff_half(n).map(|h| (h.base, h.half, h.style));
        assert_eq!(
            h("USB_DP"),
            Some(("USB".into(), Half::Pos, PairStyle::DpDm))
        );
        assert_eq!(h("DM"), Some(("".into(), Half::Neg, PairStyle::DpDm)));
        assert_eq!(
            h("/USB/USB_D+"),
            Some(("USB_D".into(), Half::Pos, PairStyle::PlusMinus))
        );
        assert_eq!(
            h("ETH_TX_N"),
            Some(("ETH_TX".into(), Half::Neg, PairStyle::UnderscorePN))
        );
        assert_eq!(
            h("CANH"),
            Some(("CAN".into(), Half::Pos, PairStyle::HighLow))
        );
        assert_eq!(
            h("CAN_L"),
            Some(("CAN".into(), Half::Neg, PairStyle::HighLow))
        );
        assert_eq!(h("SWDIO"), None);
        assert_eq!(diff_half("USB_DP").unwrap().key(), "USB");
        assert_eq!(diff_half("USB_D-").unwrap().key(), "USB");
    }

    #[test]
    fn pairs_among_names() {
        let names = [
            "/A/USB_DP",
            "/A/USB_DM",
            "LVDS_CLKP",
            "LVDS_CLKN",
            "RESET_N",
            "/B/ETH_P",
            "/B/ETH_N",
            "D+",
            "/C/D-",
        ];
        let (pairs, lone) = pair_halves(&names);
        let got: Vec<(&str, &str, PairStyle)> = pairs
            .iter()
            .map(|p| (names[p.pos], names[p.neg], p.style))
            .collect();
        assert_eq!(
            got,
            [
                ("/A/USB_DP", "/A/USB_DM", PairStyle::DpDm),
                ("/B/ETH_P", "/B/ETH_N", PairStyle::UnderscorePN),
                ("LVDS_CLKP", "LVDS_CLKN", PairStyle::GluedPN),
            ]
        );
        let lone: Vec<&str> = lone.iter().map(|(i, _)| names[*i]).collect();
        assert_eq!(
            lone,
            ["D+", "/C/D-"],
            "different sheets; RESET_N is no half"
        );
    }

    #[test]
    fn numbered_names() {
        let n = numbered("/MCU/IO4").unwrap();
        assert_eq!(
            (n.prefix.as_str(), n.index, n.suffix.as_str()),
            ("IO", 4, "")
        );
        assert_eq!(n.family("/MCU/IO4"), "/MCU/IO#");
        let n = numbered("CH10_ADC").unwrap();
        assert_eq!(
            (n.prefix.as_str(), n.index, n.suffix.as_str()),
            ("CH", 10, "_ADC")
        );
        assert_eq!(numbered("12"), None);
        assert_eq!(numbered("RESET"), None);
    }

    #[test]
    fn gpio_numbers() {
        let g = |n: &str| gpio(n).map(|g| (g.port, g.number));
        assert_eq!(g("GPIO17"), Some(("".into(), 17)));
        assert_eq!(g("IO5"), Some(("".into(), 5)));
        assert_eq!(g("PA5/SPI1_SCK"), Some(("A".into(), 5)));
        assert_eq!(g("PB12"), Some(("B".into(), 12)));
        assert_eq!(g("P0.12"), Some(("0".into(), 12)));
        assert_eq!(g("PIO1_3"), Some(("1".into(), 3)));
        assert_eq!(g("VDD"), None);
        assert_eq!(g("PG"), None);
        assert_eq!(g("PWR"), None);
    }

    #[test]
    fn uart_names() {
        assert_eq!(uart_pin("PA9/USART1_TX"), Some(Uart::Tx));
        assert_eq!(uart_pin("U0TXD"), Some(Uart::Tx));
        assert_eq!(uart_pin("RXD0"), Some(Uart::Rx));
        assert_eq!(uart_pin("RX"), Some(Uart::Rx));
        assert_eq!(uart_pin("PB8/FDCAN1_RX"), None);
        assert_eq!(uart_pin("RMII_TXD0"), None);
        assert_eq!(uart_pin("TXS"), None);
        assert_eq!(uart_words("/MCU/MCU_TX_GPS_RX"), [Uart::Tx, Uart::Rx]);
        assert_eq!(uart_words("UART1_TX"), [Uart::Tx]);
        assert_eq!(uart_words("CAN_TX"), []);
    }

    #[test]
    fn normalised_names() {
        assert_eq!(normalised("RESET_N"), normalised("RESET_n"));
        assert_eq!(normalised("RESET_N"), normalised("~{RESET}"));
        assert_eq!(normalised("RESET#"), "!RESET");
        assert_ne!(normalised("RESET"), normalised("RESET_N"));
        assert_eq!(normalised("+3V3"), normalised("3V3"));
        assert_ne!(normalised("+3V3"), normalised("3V3A"));
        assert_ne!(normalised("-5V"), normalised("5V"));
        assert_eq!(normalised("SPI.MOSI"), normalised("/X/SPI_MOSI"));
        assert_ne!(normalised("D+"), normalised("D-"));
        assert_ne!(normalised("USB_D_P"), normalised("USB_D_N"));
    }
}
