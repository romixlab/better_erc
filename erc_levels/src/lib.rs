//! Signal-level database: what each logic family, bus and differential standard defines for its levels.
//!
//! The data lives in `data/signal_levels.toml` (embedded, [`Database::builtin`]); this module is its typed
//! loader and its validator. Every family names the document it was read from (`docs`, a document in the
//! technical library with its title, revision, date and URL) and every level carries `verified`: `true` only
//! when the number was read from that document at the given location. See the file's header for the format.
//!
//! A limit is either a number in the level's unit or a linear expression of a supply (`"0.7*VDD"`,
//! `"VDD-0.4"`, `"VCC-1.025"`, `"VREF+0.1"`), evaluated with [`Family::env`] at a supply corner.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::OnceLock;

use exact_units::units::{Ampere, Farad, Kelvin, Ohm, Volt};
use exact_units::{Dim, Fixed, Point, origins, prefix};
use serde::Deserialize;

mod de;
mod expr;
mod validate;

pub use expr::{Expr, Limit};
pub use validate::Problem;

/// A voltage in volts.
pub type Volts = Fixed<Volt, prefix::One, f64>;
/// A current in amperes.
pub type Amperes = Fixed<Ampere, prefix::One, f64>;
/// A current in milliamperes, as test currents are given.
pub type MilliAmperes = Fixed<Ampere, prefix::Milli, f64>;
/// A resistance in ohms.
pub type Ohms = Fixed<Ohm, prefix::One, f64>;
/// A capacitance in farads.
pub type Farads = Fixed<Farad, prefix::One, f64>;
/// A temperature in degrees Celsius: a point on the Celsius scale, not an amount of kelvin.
pub type DegC = Point<origins::Celsius, Fixed<Kelvin, prefix::One, f64>>;

/// The supplies of a family at one corner, by name.
pub type Env = BTreeMap<String, Volts>;

/// The embedded database text.
pub const BUILTIN_TOML: &str = include_str!("../data/signal_levels.toml");

/// The format version this loader reads.
pub const SCHEMA: u32 = 1;

/// The whole database: documents and families.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Database {
    /// Format version, [`SCHEMA`].
    pub schema: u32,
    /// Source documents by id (`UM10204`, `TI-SCYA051`).
    #[serde(default)]
    pub docs: BTreeMap<String, Doc>,
    /// Families and standards by id (`i2c-fm`, `74lvc-3v3`, `lvds`).
    #[serde(default)]
    pub family: BTreeMap<String, Family>,
}

/// A source document.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Doc {
    /// Its title as printed.
    pub title: String,
    /// Who published it (NXP, TI, JEDEC, JEITA, USB-IF...).
    pub publisher: String,
    /// The document number (`UM10204`, `SCYA051B`), where it has one.
    pub number: Option<String>,
    /// The revision or version as printed (`7.0`, `B`).
    pub rev: Option<String>,
    /// Release date as printed, ISO where the day is known (`2021-10-01`, `2015-07`).
    pub date: Option<String>,
    /// Where it was fetched from.
    pub url: String,
    /// The file in the technical library, relative to `~/sync` (`design/Standards/UM10204.pdf`); none when the
    /// document could not be fetched (then nothing read from it is verified).
    pub library: Option<String>,
    /// What kind of document it is.
    pub kind: DocKind,
    /// For a substitute: the standard it stands in for (`JESD8-7A` for JEITA ED-5003A, `TIA-485-A` for a
    /// transceiver datasheet).
    pub substitute_for: Option<String>,
    /// Anything worth knowing about it (paywalled original, which part of it applies).
    pub note: Option<String>,
}

/// What kind of document a source is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocKind {
    /// The standard or specification itself.
    Standard,
    /// A harmonised twin of the standard by another body (JEITA for JEDEC).
    HarmonisedStandard,
    /// A vendor application note, handbook or design guide.
    Appnote,
    /// A part's datasheet.
    Datasheet,
}

/// A family or standard in one supply variant.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Family {
    /// Display name (`I²C Fast-mode`, `74LVC at 3.3 V`).
    pub name: String,
    /// The family the variants belong to (`I2C`, `74LVC`), for grouping.
    pub group: String,
    /// What kind of interface it is.
    pub kind: FamilyKind,
    /// The standard's own name and number (`TIA/EIA-644-A`, `JESD8C.01`).
    pub standard: Option<String>,
    /// The default source of every value of this family.
    pub source: Source,
    /// Supplies and references, in the order expressions may use them: a supply may refer to earlier ones
    /// (`VREF = 0.5*VDDQ`). The first one is the family's main supply.
    #[serde(default)]
    pub supply: Vec<Supply>,
    /// Ambient temperature range the values hold over.
    #[serde(default, deserialize_with = "de::degc_range")]
    pub temp_c: Option<[DegC; 2]>,
    /// Termination the levels assume.
    pub termination: Option<Termination>,
    /// The levels.
    #[serde(default)]
    pub level: Vec<Level>,
    /// Other families this one relates to (the CMOS levels SPI runs at).
    #[serde(default)]
    pub see: Vec<String>,
    /// Free text: "no electrical standard of its own", which parts of the standard are left out.
    pub notes: Option<String>,
}

/// What kind of interface a family is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FamilyKind {
    /// Push-pull single-ended logic.
    SingleEnded,
    /// Open-drain or open-collector single-ended (I²C, 1-Wire).
    OpenDrain,
    /// A differential pair.
    Differential,
    /// A single-ended bus with levels of its own that are not plain logic (RS-232, LIN, CC).
    Bus,
    /// No electrical standard of its own (SPI, SWD): the `notes` say what levels it uses.
    NoLevels,
}

/// Where a value was read.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// Document id in [`Database::docs`].
    pub doc: String,
    /// Section, table or page in it (`Table 10, p. 43`).
    pub loc: Option<String>,
}

/// A supply or reference voltage of a family.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supply {
    /// Its name as expressions use it (`VDD`, `VCC`, `VDDQ`, `VREF`, `VTT`, `VEE`).
    pub name: String,
    pub min: Option<Limit>,
    pub typ: Option<Limit>,
    pub max: Option<Limit>,
    /// Read from the source document.
    pub verified: bool,
    /// Location in the family's document, when it differs from the family's.
    pub loc: Option<String>,
    /// Another document than the family's.
    pub doc: Option<String>,
    pub note: Option<String>,
}

/// The termination the levels are specified with.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Termination {
    /// How it is terminated (`100 ohm across the pair`, `50 ohm to VCC-2 V`, `45 ohm to ground each line`).
    pub kind: String,
    /// The resistance.
    #[serde(default, deserialize_with = "de::fixed")]
    pub r_ohm: Option<Ohms>,
    /// The voltage it terminates to, as a limit (`"VCC-2"`), where it ends at a rail.
    pub to: Option<Limit>,
    pub verified: bool,
    /// Another document than the family's.
    pub doc: Option<String>,
    pub loc: Option<String>,
    pub note: Option<String>,
}

/// One level of a family: a parameter with its limits and conditions.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Level {
    /// The parameter. Single-ended: `VIH`, `VIL`, `VT+`, `VT-`, `VHYS`, `VOH`, `VOL`, `IIH`, `IIL`, `II`,
    /// `IOZ`, `IOH`, `IOL`, `IOS`, `CI`. Differential: `VOD`, `dVOD`, `VOS` (output common mode), `dVOS`,
    /// `VTH` / `VTL` (differential input thresholds), `VID` (input differential range), `VIC` (input common
    /// mode range), `VCM`, `RIN`, `RTERM`. Others are allowed with a `desc`; the first letter gives the
    /// dimension (V, I, R, C).
    pub param: String,
    /// What it is, for a parameter outside the list above.
    pub desc: Option<String>,
    pub min: Option<Limit>,
    pub typ: Option<Limit>,
    pub max: Option<Limit>,
    pub unit: Unit,
    /// The state or mode it applies to (`dominant`, `recessive`, `high-speed`, `mark`).
    pub state: Option<String>,
    /// The supply this row is specified at (74HC at 2.0, 4.5 and 6.0 V).
    #[serde(default, deserialize_with = "de::fixed")]
    pub vsupply: Option<Volts>,
    /// Output current of the test, positive into the pin (JEDEC's sign: IOH is negative).
    #[serde(default, deserialize_with = "de::fixed")]
    pub iout_ma: Option<MilliAmperes>,
    /// Temperature range of this row, when it differs from the family's.
    #[serde(default, deserialize_with = "de::degc_range")]
    pub temp_c: Option<[DegC; 2]>,
    /// The one temperature this row is specified at (ECL tables with 0, 25 and 85 °C columns).
    #[serde(default, deserialize_with = "de::degc")]
    pub t_c: Option<DegC>,
    /// Other test conditions as printed (load, input voltage).
    pub cond: Option<String>,
    /// Read from the source document at the given location.
    pub verified: bool,
    /// Another document than the family's.
    pub doc: Option<String>,
    /// Location in the document, when it differs from the family's.
    pub loc: Option<String>,
    pub note: Option<String>,
}

/// The unit of a level's numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Unit {
    V,
    #[serde(rename = "mV")]
    MilliV,
    /// Differential peak-to-peak volts (`VOD` of USB 3 and PCIe).
    #[serde(rename = "Vppd")]
    Vppd,
    #[serde(rename = "mVppd")]
    MilliVppd,
    A,
    #[serde(rename = "mA")]
    MilliA,
    #[serde(rename = "uA")]
    MicroA,
    #[serde(rename = "nA")]
    NanoA,
    #[serde(rename = "ohm")]
    Ohm,
    #[serde(rename = "kohm")]
    KiloOhm,
    #[serde(rename = "pF")]
    PicoF,
    #[serde(rename = "nF")]
    NanoF,
}

impl Unit {
    /// Its dimension.
    pub fn dim(self) -> Dim<'static> {
        use Unit::*;
        match self {
            V | MilliV | Vppd | MilliVppd => <Volt as exact_units::Unit>::DIM,
            A | MilliA | MicroA | NanoA => <Ampere as exact_units::Unit>::DIM,
            Ohm | KiloOhm => <exact_units::units::Ohm as exact_units::Unit>::DIM,
            PicoF | NanoF => <Farad as exact_units::Unit>::DIM,
        }
    }

    /// The factor to its coherent SI unit (V, A, Ω, F).
    pub fn factor(self) -> f64 {
        use Unit::*;
        match self {
            V | Vppd | A | Ohm => 1.0,
            MilliV | MilliVppd | MilliA => 1e-3,
            MicroA => 1e-6,
            NanoA | NanoF => 1e-9,
            KiloOhm => 1e3,
            PicoF => 1e-12,
        }
    }
}

/// The dimension a parameter name implies by its first letter (after a `d` for a change: `dVOD`), if any.
pub fn param_dim(param: &str) -> Option<Dim<'static>> {
    match param.trim_start_matches('d').chars().next()? {
        'V' => Some(<Volt as exact_units::Unit>::DIM),
        'I' => Some(<Ampere as exact_units::Unit>::DIM),
        'R' => Some(<exact_units::units::Ohm as exact_units::Unit>::DIM),
        'C' => Some(<Farad as exact_units::Unit>::DIM),
        _ => None,
    }
}

/// A limit with its unit, in the coherent SI unit of its dimension.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quantity {
    Voltage(Volts),
    Current(Amperes),
    Resistance(Ohms),
    Capacitance(Farads),
}

impl Quantity {
    /// The number in V, A, Ω or F.
    pub fn si(self) -> f64 {
        match self {
            Quantity::Voltage(v) => v.get(),
            Quantity::Current(v) => v.get(),
            Quantity::Resistance(v) => v.get(),
            Quantity::Capacitance(v) => v.get(),
        }
    }
}

/// Which supply corner to evaluate expressions at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    Min,
    Typ,
    Max,
}

/// An error loading the database.
#[derive(Debug)]
pub enum LoadError {
    /// Not valid TOML or not the format.
    Parse(toml::de::Error),
    /// Another format version.
    Schema(u32),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Parse(e) => write!(f, "signal-level database: {e}"),
            LoadError::Schema(v) => write!(
                f,
                "signal-level database: schema {v}, this loader reads {SCHEMA}"
            ),
        }
    }
}

impl std::error::Error for LoadError {}

impl Database {
    /// Parse a database from TOML text.
    pub fn parse(text: &str) -> Result<Database, LoadError> {
        let db: Database = toml::from_str(text).map_err(LoadError::Parse)?;
        if db.schema != SCHEMA {
            return Err(LoadError::Schema(db.schema));
        }
        Ok(db)
    }

    /// The embedded database, parsed once. Panics if the embedded file is broken (the tests keep it valid).
    pub fn builtin() -> &'static Database {
        static DB: OnceLock<Database> = OnceLock::new();
        DB.get_or_init(|| Database::parse(BUILTIN_TOML).expect("embedded signal_levels.toml"))
    }

    /// A family by id.
    pub fn get(&self, id: &str) -> Option<&Family> {
        self.family.get(id)
    }

    /// The families of a group (`74LVC`), in id order.
    pub fn group<'a>(&'a self, group: &'a str) -> impl Iterator<Item = (&'a str, &'a Family)> + 'a {
        self.family
            .iter()
            .filter(move |(_, f)| f.group == group)
            .map(|(id, f)| (id.as_str(), f))
    }

    /// Everything wrong with the data: unknown documents, missing sources, unordered limits, crossed
    /// thresholds, units that don't fit their parameter. Empty when the data is sound.
    pub fn validate(&self) -> Vec<Problem> {
        validate::validate(self)
    }
}

impl Family {
    /// The supply values at a corner, by name, each evaluated with the ones before it. A supply without a
    /// value at that corner falls back to typ, then to the middle of min and max.
    pub fn env(&self, corner: Corner) -> Result<Env, String> {
        let mut env = Env::new();
        for s in &self.supply {
            let v = s.at(corner, &env)?;
            env.insert(s.name.clone(), v);
        }
        Ok(env)
    }

    /// The levels of one parameter.
    pub fn levels<'a>(&'a self, param: &'a str) -> impl Iterator<Item = &'a Level> + 'a {
        self.level.iter().filter(move |l| l.param == param)
    }
}

impl Supply {
    /// Its value at a corner.
    pub fn at(&self, corner: Corner, env: &Env) -> Result<Volts, String> {
        Volts::try_new(self.at_v(corner, env)?).map_err(|e| e.to_string())
    }

    fn at_v(&self, corner: Corner, env: &Env) -> Result<f64, String> {
        let pick = match corner {
            Corner::Min => self.min.as_ref(),
            Corner::Typ => self.typ.as_ref(),
            Corner::Max => self.max.as_ref(),
        };
        if let Some(l) = pick {
            return l.eval(env);
        }
        if let Some(t) = &self.typ {
            return t.eval(env);
        }
        match (&self.min, &self.max) {
            (Some(a), Some(b)) => Ok((a.eval(env)? + b.eval(env)?) / 2.0),
            (Some(a), None) | (None, Some(a)) => a.eval(env),
            (None, None) => Err(format!("supply {} has no value", self.name)),
        }
    }
}

impl Level {
    /// Its limits at a supply corner, (min, typ, max), each with its unit.
    pub fn limits(&self, env: &Env) -> Result<[Option<Quantity>; 3], String> {
        let si = self.si(env)?;
        let dim = self.unit.dim();
        let q = |v: Option<f64>| -> Result<Option<Quantity>, String> {
            v.map(|v| {
                Ok(if dim == <Volt as exact_units::Unit>::DIM {
                    Quantity::Voltage(Volts::try_new(v).map_err(|e| e.to_string())?)
                } else if dim == <Ampere as exact_units::Unit>::DIM {
                    Quantity::Current(Amperes::try_new(v).map_err(|e| e.to_string())?)
                } else if dim == <Farad as exact_units::Unit>::DIM {
                    Quantity::Capacitance(Farads::try_new(v).map_err(|e| e.to_string())?)
                } else {
                    Quantity::Resistance(Ohms::try_new(v).map_err(|e| e.to_string())?)
                })
            })
            .transpose()
        };
        let [min, typ, max] = si;
        Ok([q(min)?, q(typ)?, q(max)?])
    }

    /// Its limits as voltages, (min, typ, max); `None` for a level that is not a voltage.
    pub fn volts(&self, env: &Env) -> Result<Option<[Option<Volts>; 3]>, String> {
        if self.unit.dim() != <Volt as exact_units::Unit>::DIM {
            return Ok(None);
        }
        let [min, typ, max] = self.si(env)?;
        let v = |x: Option<f64>| x.map(Volts::try_new).transpose().map_err(|e| e.to_string());
        Ok(Some([v(min)?, v(typ)?, v(max)?]))
    }

    /// Its limits as currents, (min, typ, max); `None` for a level that is not a current.
    pub fn amperes(&self, env: &Env) -> Result<Option<[Option<Amperes>; 3]>, String> {
        if self.unit.dim() != <Ampere as exact_units::Unit>::DIM {
            return Ok(None);
        }
        let [min, typ, max] = self.si(env)?;
        let a = |x: Option<f64>| {
            x.map(Amperes::try_new)
                .transpose()
                .map_err(|e| e.to_string())
        };
        Ok(Some([a(min)?, a(typ)?, a(max)?]))
    }

    /// The limits as plain numbers in V, A, Ω or F.
    pub(crate) fn si(&self, env: &Env) -> Result<[Option<f64>; 3], String> {
        let f = self.unit.factor();
        let one = |l: &Option<Limit>| -> Result<Option<f64>, String> {
            l.as_ref()
                .map(|l| l.eval(env).map(|v| if l.is_expr() { v } else { v * f }))
                .transpose()
        };
        Ok([one(&self.min)?, one(&self.typ)?, one(&self.max)?])
    }
}
