//! `PIN` - power inputs and outputs of the board: connector and battery pins on rails, told apart by whether a
//! regulator feeds the rail ([`erc_analysis::feed`])

mod pin_3;
mod pin_4;
mod pin_6;

use crate::Ctx;
use crate::rail::ground_named;
use erc_analysis::feed::{LinkKind, SourceKind, power_in_on};
use erc_model::class::ConnectorKind;
use erc_model::{NetId, PartClass, PartId, PinId};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) use crate::text::{designators, pin_list};

/// A connector's or battery's pins on one rail
#[derive(Clone, Debug)]
pub(crate) struct Port {
    pub part: PartId,
    pub net: NetId,
    pub pins: Vec<PinId>,
}

/// Which way power goes through a port
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Way {
    /// Power comes in: no regulator, load switch or power output feeds the rail
    In,
    /// The board hands a rail out: a regulator feeds it, or it is a USB, barrel or battery input passed on to a
    /// header
    Out,
}

/// Whether a connector kind is a typical power input (a USB receptacle, a barrel jack)
fn input_kind(class: PartClass) -> bool {
    matches!(
        class,
        PartClass::Battery
            | PartClass::Connector {
                kind: Some(
                    ConnectorKind::UsbA
                        | ConnectorKind::UsbB
                        | ConnectorKind::UsbMicro
                        | ConnectorKind::UsbC
                        | ConnectorKind::Barrel
                )
            }
    )
}

/// Every link but a resistor carries power out to a port or in from it (resistors only at 0 Ω: a pull-up from a
/// rail to a connector pin is no power path)
pub(crate) fn power_path(l: &erc_analysis::feed::Link) -> bool {
    l.kind != LinkKind::Resistor || l.ohms == Some(0.0)
}

/// The fitted connector and battery pins on rails, by part and rail, with which way power goes
pub(crate) fn ports(cx: &Ctx) -> Vec<(Port, Way)> {
    let feeds = cx.feeds();
    let power = cx.power();
    let regulated = feeds.fed_by(
        &[
            SourceKind::Regulator,
            SourceKind::LoadSwitch,
            SourceKind::PowerOut,
        ],
        &power_path,
    );
    let mut by: BTreeMap<(PartId, NetId), Vec<PinId>> = BTreeMap::new();
    for (net, sources) in &feeds.sources {
        if !power.is_rail(*net) {
            continue;
        }
        for s in sources {
            if matches!(s.kind, SourceKind::Connector | SourceKind::Battery) {
                by.entry((s.part, *net)).or_default().push(s.pin);
            }
        }
    }
    let inputs: Vec<(PartId, NetId)> = by
        .keys()
        .filter(|(p, n)| !regulated.contains_key(n) && input_kind(cx.classes().get(*p)))
        .copied()
        .collect();
    by.into_iter()
        .map(|((part, net), pins)| {
            let passed_on = !input_kind(cx.classes().get(part))
                && inputs
                    .iter()
                    .any(|(q, n)| *q != part && feeds.reach([*n], &power_path).contains_key(&net));
            let way = if regulated.contains_key(&net) || passed_on {
                Way::Out
            } else {
                Way::In
            };
            (Port { part, net, pins }, way)
        })
        .collect()
}

/// The power-input pins of fitted parts other than connectors on `nets` (ground-named ones left out)
pub(crate) fn loads_on(cx: &Ctx, nets: impl IntoIterator<Item = NetId>) -> Vec<PinId> {
    let b = cx.board();
    let mut out: Vec<PinId> = Vec::new();
    let mut seen = BTreeSet::new();
    for n in nets {
        if !seen.insert(n) {
            continue;
        }
        out.extend(
            power_in_on(cx.view(), n).into_iter().filter(|p| {
                !ground_named(cx, *p) && !cx.classes().get(b.pin(*p).part).is_connector()
            }),
        );
    }
    out
}
