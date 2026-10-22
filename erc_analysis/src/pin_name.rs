//! What a pin's name says it is for: the debug, bus and control functions the DBG and TEST checks look for
//! (`SWDIO`, `PA13/JTMS-SWDIO`, `USART1_TX`, `~{RESET}`, `SPI2_MISO`, `nINT`, `PGOOD`). It reads the symbol's pin
//! name only, the name-grammar layer of pin roles (NET-7): a pin named by its port alone (`PA13`) says nothing
//! here until alternate functions are known (NET-12).

use std::fmt;

/// A function a pin name states
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PinFunction {
    Swdio,
    Swclk,
    /// Serial wire trace output
    Swo,
    Tck,
    Tms,
    Tdi,
    Tdo,
    Reset,
    UartTx,
    UartRx,
    UartRts,
    UartCts,
    SpiSck,
    SpiMosi,
    SpiMiso,
    SpiCs,
    I2cScl,
    I2cSda,
    /// An enable or shutdown input (`EN`, `ON`, `SHDN`)
    Enable,
    /// An interrupt or alert output (`INT`, `IRQ`, `ALERT`)
    Interrupt,
    /// A power-good output (`PG`, `PGOOD`, `PWRGD`)
    PowerGood,
    /// A fault flag output (`FAULT`, `FLT`)
    Fault,
}

impl PinFunction {
    pub fn name(self) -> &'static str {
        match self {
            PinFunction::Swdio => "SWDIO",
            PinFunction::Swclk => "SWCLK",
            PinFunction::Swo => "SWO",
            PinFunction::Tck => "TCK",
            PinFunction::Tms => "TMS",
            PinFunction::Tdi => "TDI",
            PinFunction::Tdo => "TDO",
            PinFunction::Reset => "reset",
            PinFunction::UartTx => "UART TX",
            PinFunction::UartRx => "UART RX",
            PinFunction::UartRts => "UART RTS",
            PinFunction::UartCts => "UART CTS",
            PinFunction::SpiSck => "SPI SCK",
            PinFunction::SpiMosi => "SPI MOSI",
            PinFunction::SpiMiso => "SPI MISO",
            PinFunction::SpiCs => "SPI CS",
            PinFunction::I2cScl => "I2C SCL",
            PinFunction::I2cSda => "I2C SDA",
            PinFunction::Enable => "enable",
            PinFunction::Interrupt => "interrupt",
            PinFunction::PowerGood => "power good",
            PinFunction::Fault => "fault",
        }
    }

    /// SWD and JTAG signals (reset not included)
    pub fn is_debug(self) -> bool {
        matches!(
            self,
            PinFunction::Swdio
                | PinFunction::Swclk
                | PinFunction::Swo
                | PinFunction::Tck
                | PinFunction::Tms
                | PinFunction::Tdi
                | PinFunction::Tdo
        )
    }

    pub fn is_uart(self) -> bool {
        matches!(
            self,
            PinFunction::UartTx | PinFunction::UartRx | PinFunction::UartRts | PinFunction::UartCts
        )
    }

    pub fn is_spi(self) -> bool {
        matches!(
            self,
            PinFunction::SpiSck | PinFunction::SpiMosi | PinFunction::SpiMiso | PinFunction::SpiCs
        )
    }

    /// An output that only pulls low on most parts (interrupt, power good, fault)
    pub fn is_flag(self) -> bool {
        matches!(
            self,
            PinFunction::Interrupt | PinFunction::PowerGood | PinFunction::Fault
        )
    }
}

impl fmt::Display for PinFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One function a pin name states, with the peripheral instance written before it (`USART1` in `USART1_TX`,
/// empty in `TXD`)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NamedFunction {
    pub function: PinFunction,
    pub instance: String,
}

/// The functions a pin name states. Alternates are separated by `/` (`PA13/SWDIO`), words by `_`, `-`, spaces
/// and brackets (`JTMS-SWDIO`, `USART1_TX`); an overbar (`~{RESET}`), a leading `n` or a trailing `N` or `#` mark
/// an active-low signal and are read past.
pub fn functions(name: &str) -> Vec<NamedFunction> {
    let mut out: Vec<NamedFunction> = Vec::new();
    let plain = name.replace("~{", "").replace(['}', '#'], "");
    for alternate in plain.split('/') {
        let words: Vec<&str> = alternate
            .split(['_', '-', ' ', '(', ')', '.', ','])
            .filter(|w| !w.is_empty())
            .collect();
        for (i, w) in words.iter().enumerate() {
            let Some(function) = word_function(w) else {
                continue;
            };
            let instance = words[..i].join("_").to_uppercase();
            let f = NamedFunction { function, instance };
            if !out.contains(&f) {
                out.push(f);
            }
        }
    }
    out
}

/// Whether a pin name states `function`
pub fn states(name: &str, function: PinFunction) -> bool {
    functions(name).iter().any(|f| f.function == function)
}

fn word_function(word: &str) -> Option<PinFunction> {
    let w = word.to_uppercase();
    if let Some(f) = exact(&w) {
        return Some(f);
    }
    // Active low: nRST, RSTN, RESET_N (the `N` alone is a word of its own and reads as nothing)
    let low = w
        .strip_prefix('N')
        .or_else(|| w.strip_suffix('N'))
        .and_then(exact)
        .filter(|f| {
            matches!(
                f,
                PinFunction::Reset
                    | PinFunction::SpiCs
                    | PinFunction::Interrupt
                    | PinFunction::Fault
                    | PinFunction::Enable
                    | PinFunction::UartRts
                    | PinFunction::UartCts
            )
        });
    if low.is_some() {
        return low;
    }
    // Numbered: EN2, INT1, CS0
    let base = w.trim_end_matches(|c: char| c.is_ascii_digit());
    if base.len() < w.len() && !base.is_empty() {
        return exact(base).filter(|f| {
            matches!(
                f,
                PinFunction::Enable | PinFunction::Interrupt | PinFunction::SpiCs
            )
        });
    }
    None
}

fn exact(w: &str) -> Option<PinFunction> {
    Some(match w {
        "SWDIO" | "SWDAT" => PinFunction::Swdio,
        "SWCLK" | "SWCK" => PinFunction::Swclk,
        "SWO" | "TRACESWO" | "SWV" => PinFunction::Swo,
        "TCK" | "JTCK" => PinFunction::Tck,
        "TMS" | "JTMS" => PinFunction::Tms,
        "TDI" | "JTDI" => PinFunction::Tdi,
        "TDO" | "JTDO" => PinFunction::Tdo,
        "RST" | "RESET" | "NRST" | "NRESET" | "XRES" | "MCLR" => PinFunction::Reset,
        "TX" | "TXD" | "UTX" => PinFunction::UartTx,
        "RX" | "RXD" | "URX" => PinFunction::UartRx,
        "RTS" => PinFunction::UartRts,
        "CTS" => PinFunction::UartCts,
        "SCK" | "SCLK" => PinFunction::SpiSck,
        "MOSI" | "COPI" | "SIMO" => PinFunction::SpiMosi,
        "MISO" | "CIPO" | "SOMI" => PinFunction::SpiMiso,
        "CS" | "SS" | "NSS" | "NCS" | "CSN" => PinFunction::SpiCs,
        "SCL" => PinFunction::I2cScl,
        "SDA" => PinFunction::I2cSda,
        "EN" | "ENABLE" | "ON" | "SHDN" | "SHUTDOWN" => PinFunction::Enable,
        "INT" | "IRQ" | "ALERT" => PinFunction::Interrupt,
        "PG" | "PGOOD" | "PWRGD" | "POK" => PinFunction::PowerGood,
        "FAULT" | "FLT" => PinFunction::Fault,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::PinFunction::*;
    use super::*;

    fn fns(name: &str) -> Vec<PinFunction> {
        functions(name).into_iter().map(|f| f.function).collect()
    }

    #[test]
    fn names() {
        let table: &[(&str, &[PinFunction])] = &[
            ("SWDIO", &[Swdio]),
            ("PA13/JTMS-SWDIO", &[Tms, Swdio]),
            ("PA14/JTCK-SWCLK", &[Tck, Swclk]),
            ("PB3/JTDO/TRACESWO", &[Tdo, Swo]),
            ("~{RESET}", &[Reset]),
            ("NRST", &[Reset]),
            ("nRST", &[Reset]),
            ("RESET_N", &[Reset]),
            ("RST#", &[Reset]),
            ("USART1_TX", &[UartTx]),
            ("RXD", &[UartRx]),
            ("~{RTS}", &[UartRts]),
            ("SPI2_MISO", &[SpiMiso]),
            ("SCLK", &[SpiSck]),
            ("~{CS}", &[SpiCs]),
            ("CS2", &[SpiCs]),
            ("I2C1_SCL", &[I2cScl]),
            ("EN", &[Enable]),
            ("EN2", &[Enable]),
            ("~{SHDN}", &[Enable]),
            ("nINT", &[Interrupt]),
            ("INT1", &[Interrupt]),
            ("PGOOD", &[PowerGood]),
            ("~{FAULT}", &[Fault]),
            ("PA13", &[]),
            ("VDD", &[]),
            ("GND", &[]),
            ("TXEN", &[]),
            ("ENABLE_LDO", &[Enable]),
            ("N", &[]),
            ("~", &[]),
        ];
        for (name, want) in table {
            assert_eq!(fns(name), *want, "{name}");
        }
    }

    #[test]
    fn instances() {
        let f = functions("PA9/USART1_TX");
        assert_eq!(f[0].instance, "USART1");
        assert_eq!(functions("TXD")[0].instance, "");
    }
}
