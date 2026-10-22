//! Analyses over the better_erc board model ([`erc_model`]): what checks build on and the views of a design show.
//!
//! - [`class`]: part classes (resistor, P-channel MOSFET, buck regulator) with the rule that decided each.
//! - [`value`]: each part's Value field read in its class's unit, as a variant mounts it, with tolerance,
//!   ratings and E-series.
//! - [`graph`]: connectivity queries (parts on nets, nets of parts, chains of parts between two nets).
//! - [`i2c`]: the I2C bus recogniser: buses, pull-ups, ties, translators, segments.
//! - [`power`]: rails and grounds by name grammar, power symbols and pins, ground groups, switching nodes.
//!
//! Each analysis is computed once per board and run (the check engine keeps them in its context) and is plain
//! data, serialisable for the views and the MCP server.

pub mod class;
pub mod drawing;
pub mod graph;
pub mod i2c;
pub mod power;
pub mod value;

pub use class::{ClassCoverage, ClassSource, PartClass, PartClasses};
pub use i2c::I2cBuses;
pub use power::{Ground, GroundGroup, GroundKind, Power, PowerOptions, Rail, RailKind};
pub use value::{PartValue, Values};
