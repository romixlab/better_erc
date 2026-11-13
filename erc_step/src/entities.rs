//! The data section as an index by entity id, with accessors for the parameters our own walk reads.

use crate::math::{Transform, Vec3, cross, normalize, perpendicular, scale, sub};
use ruststep::ast::{EntityInstance, Name, Parameter, Record};
use std::collections::HashMap;

/// Every entity of the data section by its id (`#12`).
pub struct Entities<'a> {
    map: HashMap<u64, &'a EntityInstance>,
}

impl<'a> Entities<'a> {
    pub fn new(instances: &'a [EntityInstance]) -> Self {
        let map = instances
            .iter()
            .map(|e| match e {
                EntityInstance::Simple { id, .. } | EntityInstance::Complex { id, .. } => (*id, e),
            })
            .collect();
        Entities { map }
    }

    pub fn iter(&self) -> impl Iterator<Item = (u64, &'a EntityInstance)> + '_ {
        self.map.iter().map(|(id, e)| (*id, *e))
    }

    pub fn get(&self, id: u64) -> Option<&'a EntityInstance> {
        self.map.get(&id).copied()
    }

    /// The first parameter of a simple entity as a string: its name in most entities.
    pub fn name(&self, id: u64) -> Option<&'a str> {
        match self.map.get(&id)? {
            EntityInstance::Simple { record, .. } => params(record).first().and_then(string),
            EntityInstance::Complex { .. } => None,
        }
    }

    /// The keyword of a simple entity (`ADVANCED_FACE`); `None` for a complex one or an unknown id.
    pub fn keyword(&self, id: u64) -> Option<&'a str> {
        match self.map.get(&id)? {
            EntityInstance::Simple { record, .. } => Some(record.name.as_str()),
            EntityInstance::Complex { .. } => None,
        }
    }

    /// The parameters of a simple entity with this keyword.
    pub fn simple(&self, id: u64, keyword: &str) -> Option<&'a [Parameter]> {
        match self.map.get(&id)? {
            EntityInstance::Simple { record, .. } if record.name == keyword => Some(params(record)),
            _ => None,
        }
    }

    /// The parameters of the record with this keyword, in a simple or a complex entity.
    pub fn record(&self, id: u64, keyword: &str) -> Option<&'a [Parameter]> {
        match self.map.get(&id)? {
            EntityInstance::Simple { record, .. } if record.name == keyword => Some(params(record)),
            EntityInstance::Complex { subsuper, .. } => {
                subsuper.0.iter().find(|r| r.name == keyword).map(params)
            }
            _ => None,
        }
    }

    /// Whether the entity is, or in a complex entity contains, a record with this keyword.
    pub fn is(&self, id: u64, keyword: &str) -> bool {
        self.record(id, keyword).is_some()
    }

    /// A `CARTESIAN_POINT`'s coordinates, in the file's unit (a 2D point gets z = 0).
    pub fn point(&self, id: u64) -> Option<Vec3> {
        let p = self.simple(id, "CARTESIAN_POINT")?;
        let c = list(p.get(1)?)?;
        Some([
            real(c.first()?)?,
            real(c.get(1)?)?,
            c.get(2).and_then(real).unwrap_or(0.0),
        ])
    }

    /// A `DIRECTION`, normalized.
    pub fn direction(&self, id: u64) -> Option<Vec3> {
        let p = self.simple(id, "DIRECTION")?;
        let c = list(p.get(1)?)?;
        normalize([
            real(c.first()?)?,
            real(c.get(1)?)?,
            c.get(2).and_then(real).unwrap_or(0.0),
        ])
    }

    /// An `AXIS2_PLACEMENT_3D` as the frame it places, its origin in the file's unit. A missing axis is z, a
    /// missing reference direction x (or any perpendicular when z is near x), made orthogonal to the axis.
    pub fn axis2(&self, id: u64) -> Option<Transform> {
        let p = self.simple(id, "AXIS2_PLACEMENT_3D")?;
        let origin = self.point(entity(p.get(1)?)?)?;
        let z = p
            .get(2)
            .and_then(entity)
            .and_then(|d| self.direction(d))
            .unwrap_or([0.0, 0.0, 1.0]);
        let x_hint = p
            .get(3)
            .and_then(entity)
            .and_then(|d| self.direction(d))
            .unwrap_or([1.0, 0.0, 0.0]);
        let x = normalize(sub(x_hint, scale(z, crate::math::dot(x_hint, z))))
            .unwrap_or_else(|| perpendicular(z));
        let y = cross(z, x);
        Some(Transform::from_frame(origin, x, y, z))
    }
}

/// A record's parameters (`Record::parameter` is a list for every record in a data section).
pub fn params(record: &Record) -> &[Parameter] {
    match &record.parameter {
        Parameter::List(l) => l,
        other => std::slice::from_ref(other),
    }
}

/// `#12` as 12.
pub fn entity(p: &Parameter) -> Option<u64> {
    match p {
        Parameter::Ref(Name::Entity(id)) => Some(*id),
        Parameter::Typed { parameter, .. } => entity(parameter),
        _ => None,
    }
}

/// A real or an integer, also wrapped in a type (`LENGTH_MEASURE(25.4)`).
pub fn real(p: &Parameter) -> Option<f64> {
    match p {
        Parameter::Real(x) => Some(*x),
        Parameter::Integer(i) => Some(*i as f64),
        Parameter::Typed { parameter, .. } => real(parameter),
        _ => None,
    }
}

pub fn string(p: &Parameter) -> Option<&str> {
    match p {
        Parameter::String(s) => Some(s),
        Parameter::Typed { parameter, .. } => string(parameter),
        _ => None,
    }
}

pub fn list(p: &Parameter) -> Option<&[Parameter]> {
    match p {
        Parameter::List(l) => Some(l),
        _ => None,
    }
}

/// An enumeration value without its dots (`.MILLI.` as `MILLI`).
pub fn enumeration(p: &Parameter) -> Option<&str> {
    match p {
        Parameter::Enumeration(e) => Some(e),
        _ => None,
    }
}

/// `.T.` and `.F.`.
pub fn logical(p: &Parameter) -> Option<bool> {
    match enumeration(p)? {
        "T" => Some(true),
        "F" => Some(false),
        _ => None,
    }
}

/// The entity references in a list parameter.
pub fn refs(p: &Parameter) -> Vec<u64> {
    list(p)
        .map(|l| l.iter().filter_map(entity).collect())
        .unwrap_or_default()
}
