//! Analyses over the better_erc board model ([`erc_model`]): what checks build on and the views of a design show.
//!
//! - [`class`]: part classes (resistor, MOSFET, connector) with the rule that decided each.
//! - [`value`]: a part's Value field read in its class's unit, per variant.
//! - [`graph`]: connectivity queries (parts on nets, nets of parts, chains of parts between two nets).
//! - [`i2c`]: the I2C bus recogniser: buses, pull-ups, ties, translators, segments.
//! - [`power`]: rails, grounds and switching nodes by name and pin type.
//!
//! Each analysis is computed once per board and run (the check engine keeps them in its context) and is plain
//! data, serialisable for the views and the MCP server.

pub mod class;
pub mod graph;
pub mod i2c;
pub mod power;
pub mod value;

pub use class::{PartClass, PartClasses};
pub use i2c::I2cBuses;
pub use power::{Power, PowerOptions};
