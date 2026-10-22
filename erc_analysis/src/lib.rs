//! Analyses over the better_erc board model ([`erc_model`]): what checks build on and the views of a design show.
//!
//! - [`analog`]: op-amp and comparator channels, current-sense inputs and analog input names, by pin name.
//! - [`access`]: where a probe reaches a net (test points, connector pins, one series part away), 0 Ω groups.
//! - [`class`]: part classes (resistor, P-channel MOSFET, buck regulator) with the rule that decided each.
//! - [`value`]: each part's Value field read in its class's unit, as a variant mounts it, with tolerance,
//!   ratings and E-series.
//! - [`feed`]: power feeds: the source pins of each net (regulator and load-switch outputs, connectors,
//!   batteries) and the two-terminal parts that carry power between nets.
//! - [`graph`]: connectivity queries (parts on nets, nets of parts, chains of parts between two nets).
//! - [`i2c`]: the I2C bus recogniser: buses, pull-ups, ties, translators, segments.
//! - [`pin_name`]: the debug, bus and control functions a pin name states (`PA13/JTMS-SWDIO`, `USART1_TX`).
//! - [`pull`]: the fitted pull-up and pull-down resistors on a net.
//! - [`power`]: rails and grounds by name grammar, power symbols and pins, ground groups, switching nodes.
//! - [`supply`]: power-input pins on a net and the capacitors from it to a ground.
//! - [`package`]: a part's package from its footprint: case size, SMD or through-hole, board side.
//! - [`names`]: the name grammar beyond power: active-low markers, differential halves, numbered families, GPIO
//!   numbers and UART lines of pin names, names normalised for comparison.
//! - [`drawing`]: labels with their scopes, sheet pins, no-connect flags.
//!
//! Each analysis is computed once per board and run (the check engine keeps them in its context) and is plain
//! data, serialisable for the views and the MCP server.

pub mod access;
pub mod analog;
pub mod class;
pub mod drawing;
pub mod feed;
pub mod graph;
pub mod i2c;
pub mod names;
pub mod package;
pub mod pin_name;
pub mod power;
pub mod pull;
pub mod supply;
pub mod value;

pub use class::{ClassCoverage, ClassSource, PartClass, PartClasses};
pub use feed::Feeds;
pub use i2c::I2cBuses;
pub use power::{Ground, GroundGroup, GroundKind, Power, PowerOptions, Rail, RailKind};
pub use value::{PartValue, Values};
