//! Numbers in the TOML read into exact_units types.

use exact_units::{Fixed, Point, Storage};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer};

use crate::DegC;

/// A plain number as a `Fixed` in the field's unit and scale (`iout_ma = -4.0` is −4 mA).
pub(crate) fn fixed<'de, D: Deserializer<'de>, U: exact_units::Unit, P: exact_units::FixedScale>(
    d: D,
) -> Result<Option<Fixed<U, P, f64>>, D::Error>
where
    f64: Storage,
{
    Option::<f64>::deserialize(d)?
        .map(|v| Fixed::try_new(v).map_err(D::Error::custom))
        .transpose()
}

/// A temperature in °C.
pub(crate) fn degc<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DegC>, D::Error> {
    Option::<f64>::deserialize(d)?
        .map(|v| Point::try_from_raw(v).map_err(D::Error::custom))
        .transpose()
}

/// A temperature range `[lo, hi]` in °C.
pub(crate) fn degc_range<'de, D: Deserializer<'de>>(d: D) -> Result<Option<[DegC; 2]>, D::Error> {
    Option::<[f64; 2]>::deserialize(d)?
        .map(|[lo, hi]| {
            Ok([
                Point::try_from_raw(lo).map_err(D::Error::custom)?,
                Point::try_from_raw(hi).map_err(D::Error::custom)?,
            ])
        })
        .transpose()
}
