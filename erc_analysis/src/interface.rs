//! Interface pins and nets by name: USB 2.0 data, MDIO, CAN bus lines, SD and eMMC card pins, DDR calibration and
//! power-up pins, MIPI D-PHY lanes and multi-gigabit lanes. What the USB, HS, MEM, ETH and SER checks look for
//! until pin roles come from part data (NET-7). A name is split into alternates at `/` (`PA12/USB_DP`) and words at
//! `_`, spaces and dots; `+` and `-` stay in a word (`D+`, `TX1-`).

use crate::names::{Half, base, diff_half};

fn alternates(name: &str) -> impl Iterator<Item = Vec<String>> + '_ {
    let plain = name.replace("~{", "").replace(['}', '#'], "");
    plain
        .split('/')
        .map(|a| {
            a.split(['_', ' ', '.', '(', ')', ','])
                .filter(|w| !w.is_empty())
                .map(str::to_uppercase)
                .collect()
        })
        .collect::<Vec<_>>()
        .into_iter()
}

/// The USB 2.0 data line a pin name states: `D+`, `DP`, `USBDP`, `USB_DP`, `UD+`, `OTG_FS_DP`, `DP1` (Type-C) are
/// D+; `D-`, `DM`, `USBDM`, `UD-`, `DN1` are D-. The first such word of a name decides (`USBDP_DN1`, a hub's
/// downstream D+, is D+).
pub fn usb_data(name: &str) -> Option<Half> {
    for words in alternates(name) {
        for w in &words {
            let w = w
                .strip_prefix("USB")
                .or_else(|| w.strip_prefix('U'))
                .unwrap_or(w);
            let stem = w.trim_end_matches(|c: char| c.is_ascii_digit());
            match stem {
                "D+" | "DP" => return Some(Half::Pos),
                "D-" | "DM" => return Some(Half::Neg),
                // `DN` alone is too common a word: only with a number (`DN1`)
                "DN" if stem.len() < w.len() => return Some(Half::Neg),
                _ => {}
            }
        }
    }
    None
}

/// Whether a pin name states MDIO, the management data line of an Ethernet PHY (`MDIO`, `ETH_MDIO`, `PA2/ETH_MDIO`)
pub fn is_mdio(name: &str) -> bool {
    alternates(name).any(|words| words.iter().any(|w| w == "MDIO"))
}

/// The CAN bus line a pin name states: `CANH`, `CAN_H`, `CANH1` are high, `CANL`, `CAN_L` low
pub fn can_line(name: &str) -> Option<Half> {
    for words in alternates(name) {
        for (i, w) in words.iter().enumerate() {
            let stem = w.trim_end_matches(|c: char| c.is_ascii_digit());
            let next = words.get(i + 1).map(String::as_str);
            match (stem, next) {
                ("CANH", _) | ("CAN", Some("H")) => return Some(Half::Pos),
                ("CANL", _) | ("CAN", Some("L")) => return Some(Half::Neg),
                _ => {}
            }
        }
    }
    None
}

/// What an SD card socket or eMMC pin is for
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SdPin {
    Cmd,
    /// A data line, `DAT0`..`DAT7`
    Dat(u8),
    Clk,
    /// A card-detect switch contact (`CD`, `DET`, `DET_A`, `CARD_DETECT`, `SW_CD`)
    CardDetect,
    /// A write-protect switch contact
    WriteProtect,
}

/// The SD or eMMC functions of a pin name, one per alternate (`DAT3/CD` is a data line and card detect over it):
/// `CMD`, `DAT0`..`DAT7` (`D0`..`D7`), `CLK`, `CD`, `DET`, `WP`. Read on card sockets and eMMC parts only, where
/// these words mean nothing else.
pub fn sd_pins(name: &str) -> Vec<SdPin> {
    let mut out = Vec::new();
    for words in alternates(name) {
        let found = words.iter().enumerate().find_map(|(i, w)| {
            let num = |p: &str| {
                w.strip_prefix(p)
                    .filter(|n| !n.is_empty())
                    .and_then(|n| n.parse::<u8>().ok())
                    .filter(|n| *n < 8)
            };
            match w.as_str() {
                "CMD" => Some(SdPin::Cmd),
                "CLK" | "SDCLK" | "CK" => Some(SdPin::Clk),
                "CD" | "DET" | "DETECT" | "NCD" => Some(SdPin::CardDetect),
                "CARD" if words.get(i + 1).is_some_and(|n| n.starts_with("DET")) => {
                    Some(SdPin::CardDetect)
                }
                "WP" | "NWP" => Some(SdPin::WriteProtect),
                "WRITE" if words.get(i + 1).is_some_and(|n| n.starts_with("PROT")) => {
                    Some(SdPin::WriteProtect)
                }
                _ => num("DAT").or_else(|| num("D")).map(SdPin::Dat),
            }
        });
        if let Some(f) = found
            && !out.contains(&f)
        {
            out.push(f);
        }
    }
    out
}

/// What a DRAM pin is for, among those the MEM checks look at
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DdrPin {
    /// The calibration pin, `ZQ`, `ZQ0`, `DDR_ZQ`
    Zq,
    /// The DRAM reset, `RESET`, `~{RESET}`, `RESET_N`, `DDR_RESETN`
    Reset,
    /// Clock enable, `CKE`, `CKE0`, `DDR_CKE`
    Cke,
}

/// The DDR function of a pin name
pub fn ddr_pin(name: &str) -> Option<DdrPin> {
    for words in alternates(name) {
        for w in &words {
            let stem = w.trim_end_matches(|c: char| c.is_ascii_digit());
            match stem {
                "ZQ" => return Some(DdrPin::Zq),
                "CKE" => return Some(DdrPin::Cke),
                "RESET" | "RESETN" | "NRESET" | "RST" | "RSTN" | "NRST" => {
                    return Some(DdrPin::Reset);
                }
                _ => {}
            }
        }
    }
    None
}

/// Whether a pin or net name is one line of a MIPI D-PHY lane: a word starting with `DSI`, `CSI` or `MIPI` and a
/// differential half (`DSI_D0_P`, `CSI0_CLK_N`, `MIPI_CSI_D1P`, `DSI_CKN`)
pub fn is_mipi_lane(name: &str) -> bool {
    let b = base(name);
    let mipi = alternates(b).any(|words| {
        words
            .iter()
            .any(|w| w.starts_with("DSI") || w.starts_with("CSI") || w.starts_with("MIPI"))
    });
    mipi && half(b).is_some()
}

/// Whether a pin or net name is one line of a multi-gigabit lane: USB 3 SuperSpeed (`SSTX+`, `USB3_RX_N`, a Type-C
/// `TX1+`), PCIe (`PCIE_TX0_P`, `PERP0`, `PETN1`), SATA, HDMI TMDS, DisplayPort main link, transceiver lanes of
/// FPGAs (`MGTTXP0`, `GTX`)
pub fn is_high_speed_lane(name: &str) -> bool {
    let b = base(name);
    let up = b.to_uppercase();
    let words: Vec<&str> = up
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let starts = |p: &[&str]| words.iter().any(|w| p.iter().any(|p| w.starts_with(p)));
    // PCIe pin names carry the half glued (`PERP0`, `PETN1`), as do FPGA transceivers (`MGTTXP0`)
    if starts(&[
        "PERP", "PERN", "PETP", "PETN", "MGTTX", "MGTRX", "GTXTX", "GTXRX", "GTPTX", "GTPRX",
        "GTHTX", "GTHRX",
    ]) {
        return true;
    }
    if half(b).is_none() {
        return false;
    }
    if starts(&["SSTX", "SSRX", "USB3", "PCIE", "SATA", "TMDS", "SGMII"]) {
        return true;
    }
    if words.contains(&"SS") && starts(&["TX", "RX"]) {
        return true;
    }
    // A Type-C SuperSpeed contact name: TX1+, RX2-
    let typec = |w: &&str| matches!(*w, "TX1" | "TX2" | "RX1" | "RX2");
    if words.len() == 1 && words.iter().any(typec) {
        return true;
    }
    // DisplayPort main link: `DP_ML0_P`, `ML_LANE0+`
    words.iter().any(|w| w.starts_with("ML"))
        && (words.contains(&"DP") || starts(&["LANE", "ML0", "ML1", "ML2", "ML3"]))
}

/// The differential half of a name, glued `XP`/`XN` included where the name says what it is (`DSI_D0P`)
fn half(name: &str) -> Option<Half> {
    if let Some(h) = diff_half(name) {
        return Some(h.half);
    }
    let up = name.to_uppercase();
    let (rest, h) = up
        .strip_suffix('P')
        .map(|r| (r, Half::Pos))
        .or_else(|| up.strip_suffix('N').map(|r| (r, Half::Neg)))?;
    // Glued only after a lane number or a clock or direction word: `D0P`, `CKN`, `CLKP`, `TXN`
    let glued = rest.ends_with(|c: char| c.is_ascii_digit())
        || ["CK", "CLK", "TX", "RX"].iter().any(|w| rest.ends_with(w));
    (rest.len() >= 2 && glued).then_some(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_data_lines() {
        for n in [
            "D+",
            "DP",
            "USBDP",
            "USB_DP",
            "UD+",
            "PA12/USB_DP",
            "OTG_FS_DP",
            "DP1",
            "USBDP_DN1",
            "USB_D+",
        ] {
            assert_eq!(usb_data(n), Some(Half::Pos), "{n}");
        }
        for n in ["D-", "DM", "USBDM", "UD-", "PA11/USB_DM", "DN2", "USBDM_UP"] {
            assert_eq!(usb_data(n), Some(Half::Neg), "{n}");
        }
        for n in ["DN", "PA12", "DATA", "SDA", "DPLL"] {
            assert_eq!(usb_data(n), None, "{n}");
        }
    }

    #[test]
    fn mdio_and_can() {
        assert!(is_mdio("MDIO") && is_mdio("PA2/ETH_MDIO") && !is_mdio("MDC"));
        assert_eq!(can_line("CANH"), Some(Half::Pos));
        assert_eq!(can_line("CAN_L"), Some(Half::Neg));
        assert_eq!(can_line("CAN1_RX"), None);
    }

    #[test]
    fn sd_card_pins() {
        assert_eq!(sd_pins("DAT3/CD"), [SdPin::Dat(3), SdPin::CardDetect]);
        assert_eq!(sd_pins("CMD"), [SdPin::Cmd]);
        assert_eq!(sd_pins("SDMMC1_D0"), [SdPin::Dat(0)]);
        assert_eq!(sd_pins("DET_A"), [SdPin::CardDetect]);
        assert_eq!(sd_pins("CARD_DETECT"), [SdPin::CardDetect]);
        assert_eq!(sd_pins("WP"), [SdPin::WriteProtect]);
        assert_eq!(sd_pins("CLK"), [SdPin::Clk]);
        assert!(sd_pins("VDD").is_empty() && sd_pins("D9").is_empty());
    }

    #[test]
    fn ddr_pins() {
        assert_eq!(ddr_pin("ZQ"), Some(DdrPin::Zq));
        assert_eq!(ddr_pin("DDR_ZQ0"), Some(DdrPin::Zq));
        assert_eq!(ddr_pin("~{RESET}"), Some(DdrPin::Reset));
        assert_eq!(ddr_pin("DDR_RESETN"), Some(DdrPin::Reset));
        assert_eq!(ddr_pin("CKE1"), Some(DdrPin::Cke));
        assert_eq!(ddr_pin("CK"), None);
    }

    #[test]
    fn lanes() {
        for n in ["DSI_D0_P", "CSI0_CLK_N", "/CAM/MIPI_CSI_D1P", "DSI_CKN"] {
            assert!(is_mipi_lane(n), "{n}");
        }
        assert!(!is_mipi_lane("CSI_RESET") && !is_mipi_lane("DSI_TE"));
        for n in [
            "SSTX+",
            "USB3_RX_N",
            "TX1+",
            "PCIE_TX0_P",
            "PERP0",
            "PETN1",
            "SATA_A_P",
            "TMDS_D2+",
            "DP_ML0_P",
            "MGTTXP0",
        ] {
            assert!(is_high_speed_lane(n), "{n}");
        }
        for n in ["USB_DP", "D+", "UART_TX", "TX1", "CAN_H", "ETH_TX_P"] {
            assert!(!is_high_speed_lane(n), "{n}");
        }
    }
}
