use crate::i2c::I2cDiagnostic;
use crate::style::StyleDiagnostic;

#[derive(Default, Debug)]
pub struct Diagnostics {
    pub i2c: Vec<I2cDiagnostic>,
    pub style: Vec<StyleDiagnostic>,
}
