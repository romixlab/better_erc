use crate::prelude::*;
use crate::tabs::{TabUi, severity_color};
use better_erc::analysis::i2c::Node;
use better_erc::model::{Board, PartId};

#[derive(Default, Serialize, Deserialize)]
pub struct I2C {
    #[serde(skip)]
    transient: Option<Transient>,
}

struct Transient {}

impl TabUi for I2C {
    fn init(&mut self, _cx: &Context) {
        self.transient = Some(Transient {});
    }

    fn ui(&mut self, ui: &mut Ui, cx: &mut Context, _id: Id) {
        let Some(_t) = &mut self.transient else {
            return;
        };
        let s = cx.blocking_read();
        ScrollArea::vertical().show(ui, |ui| {
            for d in &s.designs {
                for f in d
                    .run
                    .report
                    .findings
                    .iter()
                    .filter(|f| f.check.starts_with("I2C-"))
                {
                    ui.colored_label(
                        severity_color(f.severity),
                        format!("{} {}: {}", f.check, f.board, f.message),
                    );
                }
            }
            for (board, a) in s.boards() {
                ui.monospace(buses_text(board, &a.i2c));
            }
        });
    }
}

/// The buses as text, with designators and net names: the debug dump of the recogniser's result
fn buses_text(b: &Board, buses: &better_erc::analysis::I2cBuses) -> String {
    let part = |p: PartId| b.part(p).designator.as_str();
    let bus = |i: usize| buses.buses[i].name.as_str();
    let mut out = format!("{}:\n", b.name);
    for x in &buses.buses {
        out += &format!(
            "  {}: SCL {}, SDA {}\n",
            x.name,
            b.net(x.scl).name,
            b.net(x.sda).name
        );
        if let Some(pu) = &x.pull_up {
            out += &format!(
                "    pull-up {} {} to {}\n",
                part(pu.scl),
                part(pu.sda),
                b.net(pu.rail).name
            );
        }
        for r in &x.redundant_pull_ups {
            out += &format!("    redundant pull-up {}\n", part(*r));
        }
        for n in &x.nodes {
            out += &match *n {
                Node::Device(p) => format!("    device {}\n", part(p)),
                Node::Connector(p) => format!("    connector {}\n", part(p)),
                Node::TestPoint(p) => format!("    test point {}\n", part(p)),
                Node::Translator {
                    part: p,
                    other_side,
                } => {
                    format!("    translator {} to {}\n", part(p), bus(other_side))
                }
                Node::DiscreteTranslator {
                    scl_fet,
                    sda_fet,
                    other_side,
                    ..
                } => format!(
                    "    discrete translator {} {} to {}\n",
                    part(scl_fet),
                    part(sda_fet),
                    bus(other_side)
                ),
                Node::Tie {
                    scl_tie,
                    sda_tie,
                    other_side,
                } => format!(
                    "    tie {} {} to {}\n",
                    part(scl_tie),
                    part(sda_tie),
                    bus(other_side)
                ),
                Node::Unknown(p) => format!("    unknown {}\n", part(p)),
            };
        }
    }
    for seg in &buses.direct_segments {
        out += &format!("  direct segment: {:?}\n", buses.segment_names(seg));
    }
    for seg in &buses.same_bus_segments {
        out += &format!("  same bus: {:?}\n", buses.segment_names(seg));
    }
    for n in &buses.notes {
        out += &format!("  note: {n}\n");
    }
    out
}
