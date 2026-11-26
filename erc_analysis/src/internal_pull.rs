//! Internal pulls of known parts: pins a datasheet gives a permanent pull-up or pull-down, so an open pin or a
//! missing external resistor is the part's intended circuit. A small table keyed by library symbol or value text
//! and pin name; extend it from the datasheet, with the table and revision named in `source`.

use erc_model::{Board, PinId};

/// Which way an internal pull holds its pin
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullDirection {
    Up,
    Down,
}

/// A permanent internal pull of one pin
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InternalPull {
    pub direction: PullDirection,
    pub kohm_min: f64,
    pub kohm_max: f64,
    /// The datasheet table the numbers come from
    pub source: &'static str,
}

/// How the pin name of a table row is matched, case-insensitively
enum PinMatch {
    Prefix(&'static str),
    Contains(&'static str),
}

struct Row {
    /// Prefixes of the library symbol name or the Value field
    parts: &'static [&'static str],
    pin: PinMatch,
    pull: InternalPull,
}

static TABLE: &[Row] = &[
    Row {
        parts: &["STM32G0B1", "STM32G0C1"],
        pin: PinMatch::Contains("NRST"),
        pull: InternalPull {
            direction: PullDirection::Up,
            kohm_min: 25.0,
            kohm_max: 55.0,
            source: "DS13560 rev 6 Table 60, RPU on NRST",
        },
    },
    Row {
        parts: &["USB2514"],
        pin: PinMatch::Prefix("PRTPWR"),
        pull: InternalPull {
            direction: PullDirection::Down,
            kohm_min: 0.0,
            kohm_max: f64::INFINITY,
            source: "USB2514B datasheet: PRTPWR/BC_EN has an internal pull-down",
        },
    },
];

/// The permanent internal pull of `pin` on a part in the table, if any
pub fn internal_pull(board: &Board, pin: PinId) -> Option<InternalPull> {
    let p = board.pin(pin);
    let part = board.part(p.part);
    let symbol = part.lib_id.rsplit(':').next().unwrap_or("");
    let value = part.field("Value").unwrap_or("");
    let name = p.name.to_ascii_uppercase();
    let starts = |text: &str, prefix: &str| {
        text.get(..prefix.len())
            .is_some_and(|t| t.eq_ignore_ascii_case(prefix))
    };
    TABLE
        .iter()
        .find(|r| {
            r.parts
                .iter()
                .any(|prefix| starts(symbol, prefix) || starts(value, prefix))
                && match r.pin {
                    PinMatch::Prefix(s) => name.starts_with(s),
                    PinMatch::Contains(s) => name.contains(s),
                }
        })
        .map(|r| r.pull)
}
