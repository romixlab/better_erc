//! `[nets."NAME"]`: what the project states about a net, ranked above what the schematic implies. The power
//! analysis reads it through [`ProjectNets`], an [`erc_analysis::power::NetSource`].

use crate::ctx::Config;
use erc_analysis::power::{
    DeclaredRole, GroundKind, NameRole, NetDecl, NetSource, RailKind, read_name,
};
use erc_model::{Board, Fact, NetId, Provenance};
use serde::{Deserialize, Serialize};

/// The facts of one net
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NetFacts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voltage: Option<NetVoltage>,
}

/// A net's voltage: one value, or a range with an optional nominal value, in volts
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetVoltage {
    /// The value to compute with: the one value, or the stated nominal of a range
    pub nominal: Option<f64>,
    pub min: f64,
    pub max: f64,
}

impl NetVoltage {
    /// One value
    pub fn fixed(v: f64) -> NetVoltage {
        NetVoltage {
            nominal: Some(v),
            min: v,
            max: v,
        }
    }

    /// A range rather than one value
    pub fn is_range(&self) -> bool {
        self.min != self.max
    }

    /// The largest magnitude the net reaches, for a rating
    pub fn peak(&self) -> f64 {
        self.min.abs().max(self.max.abs())
    }
}

/// The project's nets as a source for the power analysis: a net with a voltage is a rail (of the kind its name
/// says, a supply otherwise), or a ground when the voltage is 0 V and the name reads as one
pub struct ProjectNets<'c> {
    pub config: &'c Config,
}

impl NetSource for ProjectNets<'_> {
    fn net(&self, board: &Board, net: NetId) -> Option<NetDecl> {
        let name = &board.net(net).name;
        let v = self.config.nets.get(name)?.voltage?;
        let file = self.config.file.clone().unwrap_or_default();
        let source = Provenance::ProjectFile {
            file,
            key: format!("nets.{}.voltage", toml_key(name)),
        };
        let read = read_name(name, false);
        let role = match read.role {
            NameRole::Ground(kind) if v.peak() == 0.0 => DeclaredRole::Ground(kind),
            NameRole::Rail(kind) => DeclaredRole::Rail(kind),
            _ if v.peak() == 0.0 => DeclaredRole::Ground(GroundKind::Signal),
            _ => DeclaredRole::Rail(RailKind::Supply),
        };
        let ground = matches!(role, DeclaredRole::Ground(_));
        Some(NetDecl {
            role: Fact::new(role, source.clone()),
            voltage: if ground {
                None
            } else {
                v.nominal.map(|n| Fact::new(n, source.clone()))
            },
            range: (!ground && v.is_range()).then(|| Fact::new((v.min, v.max), source)),
        })
    }
}

/// A net name as a TOML key: bare when it can be, quoted otherwise (`"+3V3"`)
pub fn toml_key(name: &str) -> String {
    let bare = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if bare {
        name.to_owned()
    } else {
        toml::Value::String(name.to_owned()).to_string()
    }
}
