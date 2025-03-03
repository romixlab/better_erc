use std::collections::HashMap;

#[derive(Debug)]
pub struct Netlist {
    pub parts: HashMap<String, Part>,
    pub nets: HashMap<String, Net>,
}

#[derive(Debug)]
pub struct Part {
    pub name: String,
    pub description: String,
    pub footprints: Vec<String>,
    pub fields: HashMap<String, String>,
    pub pins: HashMap<String, Pin>,
    pub banks: HashMap<String, Bank>,
}

#[derive(Debug)]
pub struct Net {
    pub nodes: Vec<Node>,
}

#[derive(Debug)]
pub struct Node {
    pub part_ref: String,
    pub part_pin: String,
    // pub pin_type: PinType,
}

#[derive(Debug)]
pub struct Pin {
    pub default_mode: PinMode,
    pub alternate_modes: HashMap<String, PinMode>,
    pub bank_name: Option<String>,
    // voltage thresholds vs bank voltage table
    // max sink/source current
    // max frequency
    // min/max voltage or from bank?
}

#[derive(Debug)]
pub struct PinMode {
    pub ty: PinType,
    pub pull_up: Option<Pull>,
    pub pull_down: Option<Pull>,
    pub io_standard: Option<IOStandard>, // pub quiescent current vs bank voltage table
}

#[derive(Debug)]
pub enum PinType {
    DigitalInput,
    DigitalOutput,
    DigitalIO,
    AnalogInput,
    AnalogOutput,
    AnalogIO,
    PowerIn,
    PowerOut,
    PowerIO,
    OpenCollector,
    OpenEmitter,
    /// High, Low or High-Z
    TriState,
    /// Physically left unconnected and can be used for routing of other signals for example
    Unconnected,
    /// Unknown
    Unspecified,
    /// Unpowered (resistor, capacitor, ...)
    Passive,
}

#[derive(Debug)]
pub enum IOStandard {
    LVTTL,
    LVCMOS33,
    LVCMOS18,
    LVCMOS15,
    LVCMOS12,
}

#[derive(Debug)]
pub enum Pull {
    Unknown,
    Resistor { resistance: f32 },
    Current { current: f32 },
}

#[derive(Debug)]
pub struct Bank {
    pub total_source_current: f32,
    pub total_sink_current: f32,
    // min max voltage
}
