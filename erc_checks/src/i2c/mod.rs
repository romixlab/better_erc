//! `I2C` - I2C, on the buses of [`erc_analysis::i2c`]

mod i2c_1;
mod i2c_10;
mod i2c_2;
mod i2c_3;
mod i2c_7;
mod i2c_9;

use crate::Ctx;
use erc_analysis::i2c::{BusIdx, I2cBus};
use erc_analysis::value::{PartValue, format_si, value_as};
use erc_model::{PartId, Unit};
use std::collections::BTreeSet;

/// A resistor's value in the variant checked
fn ohms(cx: &Ctx, part: PartId) -> PartValue {
    value_as(cx.board(), part, cx.variant, Unit::Ohm)
}

/// `4.7 kΩ`, or the text as written when it does not read as a resistance
fn shown(v: &PartValue) -> String {
    v.value
        .map_or_else(|| format!("`{}`", v.text), |r| format_si(r, Unit::Ohm))
}

/// `/I2C1_3V3 and /I2C_U7`: the bus names of a segment
fn names(cx: &Ctx, segment: &BTreeSet<BusIdx>) -> String {
    let i2c = cx.i2c();
    let names: Vec<&str> = segment
        .iter()
        .map(|i| i2c.buses[*i].name.as_str())
        .collect();
    names.join(" and ")
}

/// The line nets of a bus
fn lines(bus: &I2cBus) -> [erc_model::NetId; 2] {
    [bus.scl, bus.sda]
}
