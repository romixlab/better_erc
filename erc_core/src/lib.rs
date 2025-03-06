pub mod i2c;
pub mod style;
pub(crate) mod util;

#[cfg(test)]
mod tests {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Suggestion,
}
