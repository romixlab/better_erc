//! The better_erc project file (CFG-2): `<name>.berc.toml` next to `<name>.kicad_pro`, else `berc.toml` in the
//! same directory, committed with the design. It states what the schematic cannot: net voltages, derating
//! factors, check settings, and the findings accepted for this project (waivers).
//!
//! [`ProjectFile::parse`] reads it into a typed [`ProjectFile`]: the [`Config`] a run uses and the
//! [`Waiver`]s applied after it. A wrong value is a [`ProjectFileError`] naming the file and the key; a key this
//! better_erc does not know is a warning, so a file written for a newer version still loads.
//!
//! ```toml
//! [nets."+3V3"]
//! voltage = 3.3
//! [nets.VBAT]
//! voltage = { min = 3.0, max = 4.2, nominal = 3.7 }
//!
//! [derating]
//! ceramic_capacitor_v = 2.0
//!
//! [checks]
//! disabled = ["NAME-5"]
//! severity = { "I2C-3" = "warning" }
//! [checks."I2C-9"]
//! min_pull_up = 2200
//!
//! [drawing.fields]
//! hide_defaults = "always"
//!
//! [parts]
//! lock = "parts.lock.toml"
//!
//! [[waiver]]
//! fingerprint = "I2C-1:net /SDA|part R5"
//! input_hash = "in1:0123456789abcdef"
//! who = "Jane Doe"
//! date = 2026-11-25
//! reason = "the pull-up sits on the mating board"
//! expires = 2027-06-01
//! ```

pub mod derating;
pub mod nets;
pub mod waiver;

use crate::catalogue::catalogue;
use crate::ctx::Config;
use crate::meta::Severity;
use derating::{Derating, SetError};
use erc_model::Time;
use nets::{NetFacts, NetVoltage, toml_key};
use std::collections::BTreeMap;
use std::fmt;
use toml::{Table, Value};
use waiver::Waiver;

/// The largest project file read, bytes
pub const FILE_SIZE_BYTES_MAX: u64 = 4 << 20;

/// The fallback name when `<name>.berc.toml` is not there
pub const FALLBACK_NAME: &str = "berc.toml";

/// A project file, read and checked
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProjectFile {
    /// The file name, relative to the project directory
    pub file: String,
    /// Nets, derating and check settings, for the run
    pub config: Config,
    pub waivers: Vec<Waiver>,
    /// The parts snapshot's settings (CFG-9)
    pub parts: PartsSettings,
    /// Keys this better_erc does not know, and other things read but not used
    pub warnings: Vec<String>,
}

/// `[parts]`: where the parts snapshot is
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartsSettings {
    /// The lock's path relative to the project directory; `None`: [`erc_parts::lock::FILE_NAME`]
    pub lock: Option<String>,
}

impl PartsSettings {
    /// The lock's path relative to the project directory
    pub fn lock_path(&self) -> &str {
        self.lock.as_deref().unwrap_or(erc_parts::lock::FILE_NAME)
    }
}

/// Why a project file cannot be used
#[derive(Debug)]
pub enum ProjectFileError {
    /// The file cannot be read
    Read { file: String, error: std::io::Error },
    /// Larger than [`FILE_SIZE_BYTES_MAX`]
    TooLarge { file: String, size_bytes: u64 },
    /// Not TOML
    Syntax {
        file: String,
        error: Box<toml::de::Error>,
    },
    /// A key with a wrong value, or a required key missing
    Key {
        file: String,
        key: String,
        message: String,
    },
}

impl fmt::Display for ProjectFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectFileError::Read { file, error } => write!(f, "{file}: {error}"),
            ProjectFileError::TooLarge { file, size_bytes } => write!(
                f,
                "{file}: {size_bytes} bytes, a project file is at most {FILE_SIZE_BYTES_MAX}"
            ),
            ProjectFileError::Syntax { file, error } => {
                write!(f, "{file}: not TOML: {}", error.to_string().trim_end())
            }
            ProjectFileError::Key { file, key, message } => write!(f, "{file}: {key}: {message}"),
        }
    }
}

impl std::error::Error for ProjectFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProjectFileError::Read { error, .. } => Some(error),
            ProjectFileError::Syntax { error, .. } => Some(error.as_ref()),
            _ => None,
        }
    }
}

/// The names a project file may have for a design whose main file is `main` (`board.kicad_pro`): its own
/// `board.berc.toml` first, then [`FALLBACK_NAME`]
pub fn candidate_names(main: &str) -> [String; 2] {
    let stem = main.rsplit_once('.').map_or(main, |(s, _)| s);
    [format!("{stem}.berc.toml"), FALLBACK_NAME.to_owned()]
}

/// The reader of one file: the file name for errors, the warnings so far
struct Reader {
    file: String,
    warnings: Vec<String>,
}

impl Reader {
    fn err(&self, key: &str, message: impl Into<String>) -> ProjectFileError {
        ProjectFileError::Key {
            file: self.file.clone(),
            key: key.to_owned(),
            message: message.into(),
        }
    }

    fn unknown(&mut self, key: &str) {
        self.warnings
            .push(format!("{}: unknown key `{key}`, ignored", self.file));
    }

    fn table<'v>(&self, key: &str, v: &'v Value) -> Result<&'v Table, ProjectFileError> {
        v.as_table()
            .ok_or_else(|| self.err(key, format!("a table, not {}", kind(v))))
    }

    fn number(&self, key: &str, v: &Value) -> Result<f64, ProjectFileError> {
        let n = match v {
            Value::Integer(i) => *i as f64,
            Value::Float(f) if f.is_finite() => *f,
            _ => return Err(self.err(key, format!("a number, not {}", kind(v)))),
        };
        Ok(n)
    }

    fn string<'v>(&self, key: &str, v: &'v Value) -> Result<&'v str, ProjectFileError> {
        v.as_str()
            .ok_or_else(|| self.err(key, format!("a string, not {}", kind(v))))
    }

    /// A date: a TOML date (`2026-11-25`) or a string of one
    fn date(&self, key: &str, v: &Value) -> Result<Time, ProjectFileError> {
        let bad = || self.err(key, format!("a date like 2026-11-25, not {}", show(v)));
        match v {
            Value::Datetime(d) if d.time.is_none() && d.offset.is_none() => {
                let d = d.date.ok_or_else(bad)?;
                Time::parse(&format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)).ok_or_else(bad)
            }
            Value::String(s) if s.len() == 10 => Time::parse(s).ok_or_else(bad),
            _ => Err(bad()),
        }
    }
}

/// What a value is, for an error
fn kind(v: &Value) -> &'static str {
    match v {
        Value::String(_) => "a string",
        Value::Integer(_) => "an integer",
        Value::Float(_) => "a float",
        Value::Boolean(_) => "a boolean",
        Value::Datetime(_) => "a date",
        Value::Array(_) => "an array",
        Value::Table(_) => "a table",
    }
}

/// A value as written, for an error
fn show(v: &Value) -> String {
    v.to_string()
}

impl ProjectFile {
    /// Reads a project file's text; `file` names it in errors and provenance
    pub fn parse(file: &str, text: &str) -> Result<ProjectFile, ProjectFileError> {
        let table: Table = text.parse().map_err(|e| ProjectFileError::Syntax {
            file: file.to_owned(),
            error: Box::new(e),
        })?;
        let mut r = Reader {
            file: file.to_owned(),
            warnings: Vec::new(),
        };
        let mut config = Config {
            file: Some(file.to_owned()),
            derating: Derating {
                file: Some(file.to_owned()),
                ..Derating::default()
            },
            ..Config::default()
        };
        let mut waivers = Vec::new();
        let mut parts = PartsSettings::default();
        for (key, v) in &table {
            match key.as_str() {
                "nets" => read_nets(&mut r, v, &mut config.nets)?,
                "derating" => read_derating(&mut r, v, &mut config.derating)?,
                "checks" => read_checks(&mut r, v, &mut config)?,
                "drawing" => read_settings(&mut r, "drawing", v, &mut config)?,
                "waiver" => waivers = read_waivers(&mut r, v)?,
                "parts" => parts = read_parts(&mut r, v)?,
                _ => r.unknown(key),
            }
        }
        Ok(ProjectFile {
            file: file.to_owned(),
            config,
            waivers,
            parts,
            warnings: r.warnings,
        })
    }
}

fn read_nets(
    r: &mut Reader,
    v: &Value,
    nets: &mut BTreeMap<String, NetFacts>,
) -> Result<(), ProjectFileError> {
    for (name, v) in r.table("nets", v)? {
        let prefix = format!("nets.{}", toml_key(name));
        let mut facts = NetFacts::default();
        for (key, v) in r.table(&prefix, v)? {
            let path = format!("{prefix}.{key}");
            match key.as_str() {
                "voltage" => facts.voltage = Some(read_voltage(r, &path, v)?),
                _ => r.unknown(&path),
            }
        }
        nets.insert(name.to_owned(), facts);
    }
    Ok(())
}

/// `3.3`, or `{ min = 3.0, max = 3.6, nominal = 3.3 }` (nominal optional)
fn read_voltage(r: &mut Reader, path: &str, v: &Value) -> Result<NetVoltage, ProjectFileError> {
    let t = match v {
        Value::Table(t) => t,
        _ => {
            let n = r.number(path, v).map_err(|_| {
                r.err(
                    path,
                    format!(
                        "volts as a number, or {{ min, max, nominal }}, not {}",
                        kind(v)
                    ),
                )
            })?;
            return Ok(NetVoltage::fixed(n));
        }
    };
    let (mut min, mut max, mut nominal) = (None, None, None);
    for (key, v) in t {
        let p = format!("{path}.{key}");
        match key.as_str() {
            "min" => min = Some(r.number(&p, v)?),
            "max" => max = Some(r.number(&p, v)?),
            "nominal" => nominal = Some(r.number(&p, v)?),
            _ => r.unknown(&p),
        }
    }
    let (Some(min), Some(max)) = (min, max) else {
        return Err(r.err(path, "a range needs both min and max"));
    };
    if min > max {
        return Err(r.err(path, format!("min {min} V is above max {max} V")));
    }
    if let Some(n) = nominal
        && !(min..=max).contains(&n)
    {
        return Err(r.err(path, format!("nominal {n} V is outside {min}..{max} V")));
    }
    Ok(NetVoltage { nominal, min, max })
}

/// `[parts]`: `lock`, a relative path inside the project directory
fn read_parts(r: &mut Reader, v: &Value) -> Result<PartsSettings, ProjectFileError> {
    let mut parts = PartsSettings::default();
    for (key, v) in r.table("parts", v)? {
        let path = format!("parts.{key}");
        match key.as_str() {
            "lock" => {
                let p = r.string(&path, v)?;
                let inside = !p.is_empty()
                    && !std::path::Path::new(p).is_absolute()
                    && !p.split(['/', '\\']).any(|c| c == "..");
                if !inside {
                    return Err(r.err(
                        &path,
                        format!("a path inside the project directory, not {p:?}"),
                    ));
                }
                parts.lock = Some(p.to_owned());
            }
            _ => r.unknown(&path),
        }
    }
    Ok(parts)
}

fn read_derating(r: &mut Reader, v: &Value, d: &mut Derating) -> Result<(), ProjectFileError> {
    for (key, v) in r.table("derating", v)? {
        let path = format!("derating.{key}");
        let n = r.number(&path, v)?;
        match d.set_checked(key, n) {
            Ok(()) => {}
            Err(SetError::Unknown) => r.unknown(&path),
            Err(SetError::Range) => {
                return Err(r.err(
                    &path,
                    format!("a factor from 1 to {}, not {n}", derating::FACTOR_MAX),
                ));
            }
        }
    }
    Ok(())
}

/// Whether a catalogue check has this id
fn known_check(id: &str) -> bool {
    catalogue().entries.iter().any(|e| e.id == id)
}

fn read_checks(r: &mut Reader, v: &Value, config: &mut Config) -> Result<(), ProjectFileError> {
    for (key, v) in r.table("checks", v)? {
        let path = format!("checks.{key}");
        match key.as_str() {
            "disabled" => read_disabled(r, &path, v, config)?,
            "severity" => read_severity(r, &path, v, config)?,
            id => {
                for (name, v) in r.table(&path, v)? {
                    let p = format!("{path}.{name}");
                    let text = setting_text(r, &p, v)?;
                    set_param(r, &p, &format!("{id}.{name}"), &text, config)?;
                }
            }
        }
    }
    Ok(())
}

/// A warning for a check id the catalogue does not have
fn warn_unknown_check(r: &mut Reader, path: &str, id: &str) {
    if !known_check(id) {
        r.warnings.push(format!(
            "{}: {path}: no check {id} in the catalogue",
            r.file
        ));
    }
}

/// `checks.disabled = ["NAME-5"]`
fn read_disabled(
    r: &mut Reader,
    path: &str,
    v: &Value,
    config: &mut Config,
) -> Result<(), ProjectFileError> {
    let Value::Array(ids) = v else {
        return Err(r.err(path, format!("an array of check ids, not {}", kind(v))));
    };
    for id in ids {
        let id = r.string(path, id)?;
        warn_unknown_check(r, path, id);
        config.disabled.insert(id.to_owned());
    }
    Ok(())
}

/// `checks.severity = { "I2C-3" = "warning" }`
fn read_severity(
    r: &mut Reader,
    path: &str,
    v: &Value,
    config: &mut Config,
) -> Result<(), ProjectFileError> {
    for (id, v) in r.table(path, v)? {
        let p = format!("{path}.{}", toml_key(id));
        let word = r.string(&p, v)?;
        let sev = Severity::from_name(word).ok_or_else(|| {
            r.err(
                &p,
                format!("error, warning, suggestion or info, not {word:?}"),
            )
        })?;
        warn_unknown_check(r, &p, id);
        config.severity.insert(id.to_owned(), sev);
    }
    Ok(())
}

/// A setting's value as [`Config::set_text`] takes it: a number or a word
fn setting_text(r: &Reader, path: &str, v: &Value) -> Result<String, ProjectFileError> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Integer(_) | Value::Float(_) => Ok(r.number(path, v)?.to_string()),
        _ => Err(r.err(path, format!("a number or a word, not {}", kind(v)))),
    }
}

/// A shared setting table (`[drawing]`), flattened to its dotted keys (`drawing.fields.hide_defaults`)
fn read_settings(
    r: &mut Reader,
    path: &str,
    v: &Value,
    config: &mut Config,
) -> Result<(), ProjectFileError> {
    if let Value::Table(t) = v {
        for (key, v) in t {
            read_settings(r, &format!("{path}.{key}"), v, config)?;
        }
        return Ok(());
    }
    let text = setting_text(r, path, v)?;
    set_param(r, path, path, &text, config)
}

/// Sets a check setting by its [`Config`] key; one no check declares is a warning
fn set_param(
    r: &mut Reader,
    path: &str,
    key: &str,
    text: &str,
    config: &mut Config,
) -> Result<(), ProjectFileError> {
    if crate::ctx::declared(key).is_err() {
        r.unknown(path);
        return Ok(());
    }
    config.set_text(key, text).map_err(|e| r.err(path, e))
}

fn read_waivers(r: &mut Reader, v: &Value) -> Result<Vec<Waiver>, ProjectFileError> {
    let Value::Array(entries) = v else {
        return Err(r.err("waiver", "[[waiver]] entries, not a single value"));
    };
    entries
        .iter()
        .enumerate()
        .map(|(i, e)| read_waiver(r, &format!("waiver[{i}]"), e))
        .collect()
}

/// One `[[waiver]]` entry
fn read_waiver(r: &mut Reader, path: &str, e: &Value) -> Result<Waiver, ProjectFileError> {
    let mut strings: BTreeMap<&str, String> = BTreeMap::new();
    let (mut date, mut expires) = (None, None);
    for (key, v) in r.table(path, e)? {
        let p = format!("{path}.{key}");
        match key.as_str() {
            k @ ("fingerprint" | "board" | "input_hash" | "who" | "reason") => {
                strings.insert(k, r.string(&p, v)?.to_owned());
            }
            "date" => date = Some(r.date(&p, v)?),
            "expires" => expires = Some(r.date(&p, v)?),
            _ => r.unknown(&p),
        }
    }
    let need = |name: &str| {
        r.err(
            &format!("{path}.{name}"),
            "missing: a waiver needs fingerprint, who, date and reason",
        )
    };
    let mut take = |name: &str| strings.remove(name).filter(|s| !s.trim().is_empty());
    Ok(Waiver {
        fingerprint: take("fingerprint").ok_or_else(|| need("fingerprint"))?,
        board: take("board"),
        input_hash: take("input_hash"),
        who: take("who").ok_or_else(|| need("who"))?,
        date: date.ok_or_else(|| need("date"))?,
        reason: take("reason").ok_or_else(|| need("reason"))?,
        expires,
    })
}

#[cfg(feature = "import")]
mod fs;
#[cfg(feature = "import")]
pub use fs::{append_waiver, find, load};

#[cfg(test)]
mod tests;
