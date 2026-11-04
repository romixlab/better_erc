//! Numbers in the TOML read into exact_units types.

use exact_units::{Fixed, Point, Storage};
use serde::{Deserialize, Deserializer};

use crate::DegC;

/// A plain number as a `Fixed` in the field's unit and scale (`iout_ma = -4.0` is −4 mA).
pub(crate) fn fixed<'de, D: Deserializer<'de>, U, P>(
    d: D,
) -> Result<Option<Fixed<U, P, f64>>, D::Error>
where
    f64: Storage,
{
    Ok(Option::<f64>::deserialize(d)?.map(Fixed::new))
}

/// A temperature in °C.
pub(crate) fn degc<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DegC>, D::Error> {
    Ok(Option::<f64>::deserialize(d)?.map(Point::from_raw))
}

/// A temperature range `[lo, hi]` in °C.
pub(crate) fn degc_range<'de, D: Deserializer<'de>>(d: D) -> Result<Option<[DegC; 2]>, D::Error> {
    Ok(Option::<[f64; 2]>::deserialize(d)?.map(|r| r.map(Point::from_raw)))
}
