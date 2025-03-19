use ecad_file_format::passive_value::Ohm;
use std::ops::RangeInclusive;

/// For digital circuits, resistors lower than this value will be considered a 'tie'.
/// For power circuits, resistors lower than this value will be considered as current sense shunts.
/// Except when resistance is 0 Ohm, in which case it's a tie.
pub const MAX_TIE_RESISTANCE: Ohm = Ohm(100.0);

/// No warnings will be issued if I2C pull-ups are withing this range.
pub const I2C_ACCEPTABLE_PULL_UP_RANGE: RangeInclusive<Ohm> = Ohm(2200.0)..=Ohm(10_000.0);
