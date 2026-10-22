//! Drawing facts (Need::Drawing): what the schematic drawing says beyond connectivity, as queries the WIRE, NAME
//! and SHORT checks use: every label with its scope (where its name reaches), sheet pins matched with the
//! hierarchical labels of their child sheet, power symbols global and local, directive labels, no-connect flags
//! and the pins they sit on. A netlist import has none of this (its board has no labels); the engine reports the
//! need as missing there.

use erc_model::{Board, LabelId, LabelKind, NetId, PinId, Provenance, SheetId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Where a label's name reaches
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LabelScope {
    /// Its sheet instance only (local labels, KiCad 10 local power symbols)
    Sheet(SheetId),
    /// Every sheet (global labels, power symbols)
    Global,
    /// Its sheet, and the parent through the sheet pin of the same name (hierarchical labels)
    Hierarchical {
        sheet: SheetId,
        parent: Option<SheetId>,
    },
    /// A sheet pin: joins a net of the parent sheet with the child's hierarchical label of the same name
    SheetPin {
        parent: SheetId,
        child: Option<SheetId>,
    },
    /// Names no net: a directive label (net class flag) sits on a net it does not name
    Directive(SheetId),
}

/// A no-connect flag
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoConnectFlag {
    /// Index in [`Board::no_connects`]
    pub index: usize,
    pub sheet: SheetId,
    /// The pin it sits on; `None` for a flag on nothing (or on a wire without a pin)
    pub pin: Option<PinId>,
    /// The pin's net when the flag sits on a pin
    pub net: Option<NetId>,
}

/// The drawing facts of a board
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Drawing {
    /// The scope of every label, by [`LabelId`]
    pub scopes: Vec<LabelScope>,
    /// Net-naming labels (local, global, hierarchical, power) by text
    pub by_text: BTreeMap<String, Vec<LabelId>>,
    /// Sheet pins with the hierarchical labels of the same name in their child sheet
    pub sheet_pin_pairs: Vec<(LabelId, LabelId)>,
    /// Sheet pins whose child sheet has no hierarchical label of that name
    pub unmatched_sheet_pins: Vec<LabelId>,
    /// Hierarchical labels with no sheet pin of that name on their sheet's symbol in the parent (or in the root)
    pub unmatched_hierarchical: Vec<LabelId>,
    pub no_connects: Vec<NoConnectFlag>,
}

impl Drawing {
    /// Whether the board carries a drawing at all: labels, sheets with files (a netlist has neither)
    pub fn available(board: &Board) -> bool {
        board.source.tool == "kicad" && !board.sheets.is_empty()
    }

    pub fn of(board: &Board) -> Drawing {
        let mut d = Drawing::default();
        for (i, l) in board.labels.iter().enumerate() {
            let id = LabelId::from_index(i);
            let scope = match l.kind {
                LabelKind::Local | LabelKind::LocalPower => LabelScope::Sheet(l.sheet),
                LabelKind::Global | LabelKind::Power => LabelScope::Global,
                LabelKind::Hierarchical => LabelScope::Hierarchical {
                    sheet: l.sheet,
                    parent: board.sheet(l.sheet).parent,
                },
                LabelKind::SheetPin => LabelScope::SheetPin {
                    parent: l.sheet,
                    child: l.child,
                },
                LabelKind::Directive => LabelScope::Directive(l.sheet),
            };
            d.scopes.push(scope);
            if !matches!(l.kind, LabelKind::SheetPin | LabelKind::Directive) {
                d.by_text.entry(l.text.clone()).or_default().push(id);
            }
        }
        for (i, l) in board.labels.iter().enumerate() {
            let id = LabelId::from_index(i);
            match l.kind {
                LabelKind::SheetPin => {
                    let pair = l.child.and_then(|child| {
                        board.labels.iter().position(|h| {
                            h.kind == LabelKind::Hierarchical
                                && h.sheet == child
                                && h.text == l.text
                        })
                    });
                    match pair {
                        Some(h) => d.sheet_pin_pairs.push((id, LabelId::from_index(h))),
                        None => d.unmatched_sheet_pins.push(id),
                    }
                }
                LabelKind::Hierarchical => {
                    let matched = board.labels.iter().any(|p| {
                        p.kind == LabelKind::SheetPin
                            && p.child == Some(l.sheet)
                            && p.text == l.text
                    });
                    if !matched {
                        d.unmatched_hierarchical.push(id);
                    }
                }
                _ => {}
            }
        }
        for (i, n) in board.no_connects.iter().enumerate() {
            d.no_connects.push(NoConnectFlag {
                index: i,
                sheet: n.sheet,
                pin: n.pin,
                net: n.pin.and_then(|p| board.pin(p).net),
            });
        }
        d
    }

    pub fn scope(&self, label: LabelId) -> LabelScope {
        self.scopes[label.index()]
    }

    /// The net-naming labels with this text, everywhere
    pub fn labels_named<'a>(&'a self, text: &str) -> &'a [LabelId] {
        self.by_text.get(text).map_or(&[], Vec::as_slice)
    }

    /// The labels drawn on a sheet
    pub fn labels_on_sheet(board: &Board, sheet: SheetId) -> impl Iterator<Item = LabelId> + '_ {
        board
            .labels
            .iter()
            .enumerate()
            .filter(move |(_, l)| l.sheet == sheet)
            .map(|(i, _)| LabelId::from_index(i))
    }

    /// Labels attached to nothing (KiCad's `label_dangling`)
    pub fn dangling(board: &Board) -> impl Iterator<Item = LabelId> + '_ {
        board
            .labels
            .iter()
            .enumerate()
            .filter(|(_, l)| l.dangling)
            .map(|(i, _)| LabelId::from_index(i))
    }

    /// The labels of a kind
    pub fn of_kind(board: &Board, kind: LabelKind) -> impl Iterator<Item = LabelId> + '_ {
        board
            .labels
            .iter()
            .enumerate()
            .filter(move |(_, l)| l.kind == kind)
            .map(|(i, _)| LabelId::from_index(i))
    }

    /// Pins under a no-connect flag that are on a net with other component pins: the flag says "nothing here"
    /// and the wire says otherwise
    pub fn flagged_but_connected<'a>(
        &'a self,
        board: &'a Board,
    ) -> impl Iterator<Item = &'a NoConnectFlag> + 'a {
        self.no_connects.iter().filter(move |f| {
            f.net.is_some_and(|n| {
                board
                    .net(n)
                    .pins
                    .iter()
                    .filter(|p| board.is_component(board.pin(**p).part))
                    .count()
                    > 1
            })
        })
    }

    /// The sheet file a label is drawn in: the provenance of a finding on it
    pub fn source(board: &Board, label: LabelId) -> Provenance {
        Provenance::Schematic {
            file: board.sheet(board.label(label).sheet).file.clone(),
        }
    }
}
