//! `SHORT` - Shorts and net collisions

mod short_1;
mod short_11;
mod short_12;
mod short_2;
mod short_3;
mod short_4;
mod short_5;
mod short_6;
mod short_7;

use erc_model::{Board, EcadPinType, PartClass, PartId, PinId};

/// The pins of a part whose name is one of `names` (any case)
pub(crate) fn pins_named(b: &Board, part: PartId, names: &[&str]) -> Vec<PinId> {
    b.part(part)
        .pins
        .iter()
        .copied()
        .filter(|p| names.iter().any(|n| b.pin(*p).name.eq_ignore_ascii_case(n)))
        .collect()
}

/// A part with two terminals that only passes current between them (R, C, L, ferrite, diode, fuse)
pub(crate) fn two_terminal(c: PartClass) -> bool {
    matches!(
        c,
        PartClass::Resistor
            | PartClass::Capacitor { .. }
            | PartClass::Inductor
            | PartClass::FerriteBead
            | PartClass::Diode { .. }
            | PartClass::Fuse { .. }
    )
}

/// A part that joins its two nets at DC by design: a 0 Ω, jumper, net tie, a closed switch
pub(crate) fn is_link(c: PartClass, ohms: Option<f64>) -> bool {
    matches!(
        c,
        PartClass::Jumper | PartClass::NetTie | PartClass::Switch { .. }
    ) || (c == PartClass::Resistor && ohms == Some(0.0))
}

/// A pin that drives: output, tri-state, open collector or emitter, power output
pub(crate) fn drives(t: Option<&EcadPinType>) -> bool {
    matches!(
        t,
        Some(
            EcadPinType::Output
                | EcadPinType::TriState
                | EcadPinType::OpenCollector
                | EcadPinType::OpenEmitter
                | EcadPinType::PowerOut
        )
    )
}

/// A programmable part: its IOs are set as inputs or outputs by firmware or a bitstream
pub(crate) fn programmable(c: PartClass) -> bool {
    matches!(c, PartClass::Mcu | PartClass::Soc | PartClass::Fpga)
}
