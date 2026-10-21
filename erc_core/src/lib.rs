//! Analysis and checks on a [`Netlist`](ecad_file_format::netlist::Netlist): [`PcbAssembly::new`] derives the
//! power structure and the I2C buses and runs the checks. Check IDs (`I2C-1`, `BOM-9`) are those of the check
//! catalogue, `docs/checks.md`; thresholds live in `config.rs`.

mod config;
mod diagnostics;
pub mod general;
pub mod i2c;
pub mod pcba;
pub mod power;
pub mod style;
pub(crate) mod util;

pub use pcba::PcbAssembly;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    SevereWarning,
    Warning,
    Info,
    Suggestion,
}
