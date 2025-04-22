mod config;
mod diagnostics;
pub mod general;
pub mod i2c;
pub mod pcba;
pub mod power;
pub mod style;
pub(crate) mod util;

pub use pcba::Pcba;

#[cfg(test)]
mod tests {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    SevereWarning,
    Warning,
    Info,
    Suggestion,
}
