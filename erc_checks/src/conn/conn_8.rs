//! `CONN-8` MicroHDMI wired with the full size HDMI pinout

use super::connectors;
use crate::meta::*;
use crate::usb::connected;
use crate::{Ctx, Finding};
use erc_analysis::interface::is_high_speed_lane;
use erc_analysis::names::diff_half;
use erc_model::class::ConnectorKind;
use erc_model::{NetId, PartClass, PartId};

pub struct Conn8;

static META: CheckMeta = CheckMeta {
    id: "CONN-8",
    category: "CONN",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// A micro HDMI (type D) connector by its symbol, or by a footprint that says so (`HDMI_Micro`, `HDMI_D`, `Type_D`)
/// under a symbol of another HDMI type
fn type_d(cx: &Ctx, part: PartId) -> bool {
    let kind = match cx.classes().get(part) {
        PartClass::Connector { kind } => kind,
        _ => return false,
    };
    if kind == Some(ConnectorKind::MicroHdmi) {
        return true;
    }
    let fp = cx
        .view()
        .field(part, "Footprint")
        .unwrap_or("")
        .to_lowercase();
    fp.contains("hdmi")
        && (fp.contains("micro")
            || fp.contains("hdmi_d")
            || fp.contains("type_d")
            || fp.contains("typed"))
}

/// A TMDS line by its net name (`TMDS_D2+`, `HDMI_D2_P`, `D2+`)
fn tmds(name: &str) -> bool {
    let up = name.to_uppercase();
    is_high_speed_lane(name)
        || (diff_half(name).is_some()
            && (up.contains("TMDS")
                || up.contains("HDMI")
                || up.contains("D2")
                || up.contains("CK")))
}

fn hot_plug(name: &str) -> bool {
    let up = name.to_uppercase();
    up.contains("HPD") || up.contains("HOTPLUG") || up.contains("HOT_PLUG")
}

impl Check for Conn8 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A type D connector whose pin 1 carries a TMDS line or pin 19 hot-plug detect, or whose pin 18 is on a rail:
    /// the type A pinout (1 D2+, 18 +5 V, 19 HPD) where type D has 1 HPD, 18 SDA, 19 +5 V
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for part in connectors(cx) {
            if !type_d(cx, part) {
                continue;
            }
            let net = |number: &str| -> Option<NetId> {
                let pin = b
                    .part(part)
                    .pins
                    .iter()
                    .find(|p| b.pin(**p).number == number)?;
                connected(cx, *pin)
            };
            let mut what = Vec::new();
            let mut nets = Vec::new();
            if let Some(n) = net("1")
                && tmds(&b.net(n).name)
            {
                what.push(format!(
                    "pin 1 carries {} (a TMDS line; type D has HPD there)",
                    b.net(n).name
                ));
                nets.push(n);
            }
            if let Some(n) = net("18")
                && power.is_rail(n)
            {
                what.push(format!(
                    "pin 18 is on {} (+5 V of type A; type D has SDA there)",
                    b.net(n).name
                ));
                nets.push(n);
            }
            if let Some(n) = net("19")
                && hot_plug(&b.net(n).name)
            {
                what.push(format!(
                    "pin 19 carries {} (HPD of type A; type D has +5 V there)",
                    b.net(n).name
                ));
                nets.push(n);
            }
            if what.is_empty() {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .part(part)
                    .nets(nets)
                    .message(format!(
                        "micro HDMI connector {} wired with the type A pinout: {}: +5 V and DDC land on TMDS lanes",
                        b.part(part).designator,
                        what.join("; ")
                    ))
                    .fix("the type D pin table from the connector's datasheet (1 HPD, 2 utility, 3-14 TMDS, 15 CEC, 16 GND, 17 SCL, 18 SDA, 19 +5 V); a symbol per connector type")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Conn8));

#[cfg(test)]
mod tests {
    #[test]
    fn type_a_pinout() {
        let found = crate::testing::fingerprints(&super::Conn8, "conn-8_micro_hdmi_pinout");
        assert_eq!(
            found,
            [
                "CONN-8:net +5V|net /HPD1|net /TMDS1_D2+|part J1",
                "CONN-8:net +5V|net /HPD2|net /TMDS2_D2+|part J2",
            ],
            "{found:?}"
        );
    }
}
