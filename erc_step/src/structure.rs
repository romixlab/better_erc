//! Our own walk over the entities for what truck does not read: units, the assembly tree and colours.
//!
//! The product tree: a `PRODUCT_DEFINITION` has its shape through `PRODUCT_DEFINITION_SHAPE` and
//! `SHAPE_DEFINITION_REPRESENTATION`, and representations of the same shape are joined by plain
//! `SHAPE_REPRESENTATION_RELATIONSHIP`s. A `NEXT_ASSEMBLY_USAGE_OCCURRENCE` puts a child product into a parent; its
//! `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION` names the representation relationship with an
//! `ITEM_DEFINED_TRANSFORMATION` that moves the child. A `MAPPED_ITEM` places a whole representation inside
//! another one; a product whose shape is only placed that way is a child as well. The walk goes from the products
//! that are nobody's child down to every solid, multiplying the
//! transforms; a solid no product reaches is placed where its own coordinates put it.

use crate::entities::{Entities, entity, list, real, refs, string};
use crate::math::Transform;
use crate::{Colour, LengthUnit, Warning, WarningKind};
use ruststep::ast::EntityInstance;
use std::collections::{HashMap, HashSet};

/// The solid entities: their records and where their shells are.
const SOLIDS: [&str; 3] = [
    "MANIFOLD_SOLID_BREP",
    "BREP_WITH_VOIDS",
    "SHELL_BASED_SURFACE_MODEL",
];
/// Geometry the reader does not read, reported when a representation has it.
const UNSUPPORTED: [&str; 4] = [
    "FACETED_BREP",
    "TESSELLATED_SHELL",
    "TESSELLATED_SOLID",
    "TRIANGULATED_FACE",
];

const MAX_DEPTH: usize = 32;

/// One solid placed in the model frame.
#[derive(Clone, Debug)]
pub struct Placed {
    pub name: String,
    pub solid: u64,
    /// The solid's shells: `CLOSED_SHELL` or `OPEN_SHELL` entities, with `false` for an
    /// `ORIENTED_CLOSED_SHELL` or `ORIENTED_OPEN_SHELL` that turns its shell inside out.
    pub shells: Vec<(u64, bool)>,
    /// From the solid's own coordinates, scaled to mm, into the model frame in mm.
    pub transform: Transform,
    /// The solid's length unit in mm.
    pub unit_mm: f64,
    /// The angle unit of its context in radians (for cone angles).
    pub angle_rad: f64,
    /// The colour of a mapped item or product above the solid, used when the solid has none of its own.
    pub inherited_colour: Option<Colour>,
}

pub struct Structure {
    pub unit: LengthUnit,
    pub placed: Vec<Placed>,
    /// Colours by the entity they are on (solids, shells, faces, mapped items).
    pub colours: HashMap<u64, Colour>,
    /// The file's plane angle unit in radians (the first context's that is not radians).
    pub angle_rad: f64,
}

struct Walk<'e, 'a> {
    ents: &'e Entities<'a>,
    /// representation → (items, context)
    reps: HashMap<u64, (Vec<u64>, Option<u64>)>,
    /// product definition (or occurrence) → its PRODUCT_DEFINITION_SHAPEs
    shapes_of: HashMap<u64, Vec<u64>>,
    /// PRODUCT_DEFINITION_SHAPE → representations
    reps_of_shape: HashMap<u64, Vec<u64>>,
    /// plain SHAPE_REPRESENTATION_RELATIONSHIP links, both ways
    linked: HashMap<u64, Vec<u64>>,
    /// parent product definition → (occurrence, child product definition)
    children: HashMap<u64, Vec<(u64, u64)>>,
    /// occurrence → (rep_1, rep_2, transform item 1, transform item 2)
    occurrence_transform: HashMap<u64, (u64, u64, u64, u64)>,
    colours: HashMap<u64, Colour>,
    units: HashMap<u64, (Option<LengthUnit>, f64)>,
    placed: Vec<Placed>,
    reached: HashSet<u64>,
    warnings: Vec<Warning>,
}

impl Structure {
    pub fn read(ents: &Entities<'_>, warnings: &mut Vec<Warning>) -> Structure {
        let mut w = Walk {
            ents,
            reps: HashMap::new(),
            shapes_of: HashMap::new(),
            reps_of_shape: HashMap::new(),
            linked: HashMap::new(),
            children: HashMap::new(),
            occurrence_transform: HashMap::new(),
            colours: HashMap::new(),
            units: HashMap::new(),
            placed: Vec::new(),
            reached: HashSet::new(),
            warnings: Vec::new(),
        };
        w.index();
        let mut product_defs: Vec<u64> = ents
            .iter()
            .filter(|(_, e)| keyword_of(e).is_some_and(is_product_definition))
            .map(|(id, _)| id)
            .collect();
        product_defs.sort_unstable();
        let child_set: HashSet<u64> = w.children.values().flatten().map(|(_, c)| *c).collect();
        // A product whose shape another one places through MAPPED_ITEMs (an assembly written without
        // NEXT_ASSEMBLY_USAGE_OCCURRENCEs) is a child too: as a root its solids would be there twice
        let mapped: HashSet<u64> = ents
            .iter()
            .filter_map(|(id, _)| {
                ents.simple(id, "REPRESENTATION_MAP")?
                    .get(1)
                    .and_then(entity)
            })
            .collect();
        let is_mapped = |pd: u64| {
            let group = w.rep_group(pd);
            !group.is_empty() && group.iter().all(|r| mapped.contains(r))
        };
        let mut roots: Vec<u64> = product_defs
            .iter()
            .copied()
            .filter(|pd| !child_set.contains(pd) && !is_mapped(*pd))
            .collect();
        if roots.is_empty() {
            roots = product_defs
                .iter()
                .copied()
                .filter(|pd| !child_set.contains(pd))
                .collect();
        }
        let mut unit = None;
        for &root in &roots {
            let group = w.rep_group(root);
            if unit.is_none() {
                unit = group.iter().find_map(|r| w.rep_unit(*r).0);
            }
            w.visit_product(root, Transform::IDENTITY, None, 0);
        }
        w.place_unreached();
        if unit.is_none() {
            unit = w.placed.first().map(|p| LengthUnit {
                name: unit_name(p.unit_mm),
                mm: p.unit_mm,
            });
        }
        let mut reps: Vec<u64> = w.reps.keys().copied().collect();
        reps.sort_unstable();
        let angle_rad = reps
            .into_iter()
            .map(|r| w.rep_unit(r).1)
            .find(|a| *a != 1.0)
            .unwrap_or(1.0);
        warnings.append(&mut w.warnings);
        Structure {
            unit: unit.unwrap_or_default(),
            placed: w.placed,
            colours: w.colours,
            angle_rad,
        }
    }
}

fn keyword_of(e: &EntityInstance) -> Option<&str> {
    match e {
        EntityInstance::Simple { record, .. } => Some(record.name.as_str()),
        EntityInstance::Complex { .. } => None,
    }
}

fn is_product_definition(k: &str) -> bool {
    matches!(
        k,
        "PRODUCT_DEFINITION" | "PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS"
    )
}

fn unit_name(mm: f64) -> String {
    let named = [
        (1.0, "mm"),
        (1000.0, "m"),
        (10.0, "cm"),
        (25.4, "inch"),
        (0.001, "µm"),
        (304.8, "foot"),
    ];
    named
        .iter()
        .find(|(v, _)| (mm / v - 1.0).abs() < 1e-9)
        .map(|(_, n)| n.to_string())
        .unwrap_or_else(|| format!("{mm} mm"))
}

impl Walk<'_, '_> {
    fn index(&mut self) {
        let ents = self.ents;
        let mut srr_plain = Vec::new();
        let mut styled = Vec::new();
        let mut overriding = Vec::new();
        for (id, e) in ents.iter() {
            match e {
                EntityInstance::Simple { record, .. } => {
                    let p = crate::entities::params(record);
                    let k = record.name.as_str();
                    match k {
                        "PRODUCT_DEFINITION_SHAPE" => {
                            if let Some(def) = p.get(2).and_then(entity) {
                                self.shapes_of.entry(def).or_default().push(id);
                            }
                        }
                        "SHAPE_DEFINITION_REPRESENTATION" => {
                            if let (Some(s), Some(r)) =
                                (p.first().and_then(entity), p.get(1).and_then(entity))
                            {
                                self.reps_of_shape.entry(s).or_default().push(r);
                            }
                        }
                        "SHAPE_REPRESENTATION_RELATIONSHIP" => {
                            if let (Some(a), Some(b)) =
                                (p.get(2).and_then(entity), p.get(3).and_then(entity))
                            {
                                srr_plain.push((a, b));
                            }
                        }
                        "NEXT_ASSEMBLY_USAGE_OCCURRENCE" => {
                            if let (Some(parent), Some(child)) =
                                (p.get(3).and_then(entity), p.get(4).and_then(entity))
                            {
                                self.children.entry(parent).or_default().push((id, child));
                            }
                        }
                        "STYLED_ITEM" => styled.push(id),
                        "OVER_RIDING_STYLED_ITEM" => overriding.push(id),
                        _ if k.ends_with("REPRESENTATION") => {
                            if let (Some(items), Some(ctx)) = (p.get(1).and_then(list), p.get(2)) {
                                self.reps.insert(
                                    id,
                                    (items.iter().filter_map(entity).collect(), entity(ctx)),
                                );
                            }
                        }
                        _ => {}
                    }
                }
                EntityInstance::Complex { subsuper, .. } => {
                    // a representation relationship with a transformation is complex; so are some representations
                    let has = |n: &str| subsuper.0.iter().any(|r| r.name == n);
                    if has("REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION") {
                        continue;
                    }
                    if let Some(rep) = subsuper
                        .0
                        .iter()
                        .find(|r| r.name.ends_with("REPRESENTATION"))
                    {
                        let p = crate::entities::params(rep);
                        if let (Some(items), Some(ctx)) = (p.get(1).and_then(list), p.get(2)) {
                            self.reps.insert(
                                id,
                                (items.iter().filter_map(entity).collect(), entity(ctx)),
                            );
                        }
                    }
                }
            }
        }
        for (a, b) in srr_plain {
            self.linked.entry(a).or_default().push(b);
            self.linked.entry(b).or_default().push(a);
        }
        // CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(relationship, PRODUCT_DEFINITION_SHAPE of the occurrence)
        for (_, e) in ents.iter() {
            let EntityInstance::Simple { record, .. } = e else {
                continue;
            };
            if record.name != "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION" {
                continue;
            }
            let p = crate::entities::params(record);
            let (Some(rel), Some(pds)) = (p.first().and_then(entity), p.get(1).and_then(entity))
            else {
                continue;
            };
            let Some(occurrence) = ents
                .simple(pds, "PRODUCT_DEFINITION_SHAPE")
                .and_then(|q| q.get(2))
                .and_then(entity)
            else {
                continue;
            };
            let Some(rr) = ents.record(rel, "REPRESENTATION_RELATIONSHIP") else {
                continue;
            };
            let Some(t) = ents.record(rel, "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION")
            else {
                continue;
            };
            let (Some(rep1), Some(rep2)) = (rr.get(2).and_then(entity), rr.get(3).and_then(entity))
            else {
                continue;
            };
            let Some(idt) = t.first().and_then(entity) else {
                continue;
            };
            match ents.simple(idt, "ITEM_DEFINED_TRANSFORMATION") {
                Some(q) => {
                    if let (Some(a1), Some(a2)) =
                        (q.get(2).and_then(entity), q.get(3).and_then(entity))
                    {
                        self.occurrence_transform
                            .insert(occurrence, (rep1, rep2, a1, a2));
                    }
                }
                None => self.warn(
                    Some(idt),
                    WarningKind::Placement,
                    "a transformation other than ITEM_DEFINED_TRANSFORMATION",
                ),
            }
        }
        // plain styles first, overriding ones after, so they win
        for id in styled.into_iter().chain(overriding) {
            let Some(p) = ents
                .simple(id, "STYLED_ITEM")
                .or_else(|| ents.simple(id, "OVER_RIDING_STYLED_ITEM"))
            else {
                continue;
            };
            let (Some(styles), Some(item)) = (p.get(1), p.get(2).and_then(entity)) else {
                continue;
            };
            if let Some(c) = self.style_colour(&refs(styles)) {
                self.colours.insert(item, c);
            }
        }
    }

    fn warn(&mut self, entity: Option<u64>, kind: WarningKind, message: impl Into<String>) {
        self.warnings.push(Warning {
            entity,
            kind,
            message: message.into(),
        });
    }

    /// The colour of a style: a surface colour first (`SURFACE_STYLE_USAGE` ... `COLOUR_RGB`), else any colour
    /// (a curve style's).
    fn style_colour(&self, styles: &[u64]) -> Option<Colour> {
        let mut fallback = None;
        let mut stack: Vec<(u64, bool, usize)> = styles.iter().map(|s| (*s, false, 0)).collect();
        let mut seen = HashSet::new();
        while let Some((id, surface, depth)) = stack.pop() {
            if depth > 10 || !seen.insert(id) {
                continue;
            }
            if let Some(c) = self.colour(id) {
                if surface {
                    return Some(c);
                }
                fallback.get_or_insert(c);
                continue;
            }
            let Some(k) = self.ents.keyword(id) else {
                continue;
            };
            let surface = surface || k.starts_with("SURFACE_STYLE") || k == "SURFACE_SIDE_STYLE";
            if let Some(EntityInstance::Simple { record, .. }) = self.ents.get(id) {
                for p in crate::entities::params(record) {
                    let mut push = |r: u64| stack.push((r, surface, depth + 1));
                    match p {
                        ruststep::ast::Parameter::List(l) => {
                            l.iter().filter_map(entity).for_each(&mut push)
                        }
                        other => entity(other).into_iter().for_each(&mut push),
                    }
                }
            }
        }
        fallback
    }

    fn colour(&self, id: u64) -> Option<Colour> {
        if let Some(p) = self.ents.simple(id, "COLOUR_RGB") {
            let c = |i: usize| p.get(i).and_then(real).map(|v| v.clamp(0.0, 1.0) as f32);
            return Some([c(1)?, c(2)?, c(3)?]);
        }
        let name = self
            .ents
            .simple(id, "DRAUGHTING_PRE_DEFINED_COLOUR")
            .and_then(|p| p.first())
            .and_then(string)?;
        Some(match name {
            "red" => [1.0, 0.0, 0.0],
            "green" => [0.0, 1.0, 0.0],
            "blue" => [0.0, 0.0, 1.0],
            "yellow" => [1.0, 1.0, 0.0],
            "magenta" => [1.0, 0.0, 1.0],
            "cyan" => [0.0, 1.0, 1.0],
            "black" => [0.0, 0.0, 0.0],
            "white" => [1.0, 1.0, 1.0],
            _ => return None,
        })
    }

    /// The representations of a product definition: those of its shapes and those linked to them.
    fn rep_group(&self, pd: u64) -> Vec<u64> {
        let mut out = Vec::new();
        let mut stack: Vec<u64> = self
            .shapes_of
            .get(&pd)
            .into_iter()
            .flatten()
            .flat_map(|s| self.reps_of_shape.get(s).into_iter().flatten())
            .copied()
            .collect();
        let mut seen = HashSet::new();
        while let Some(r) = stack.pop() {
            if !seen.insert(r) {
                continue;
            }
            out.push(r);
            stack.extend(self.linked.get(&r).into_iter().flatten().copied());
        }
        out.sort_unstable();
        out
    }

    /// A representation's length unit (`None` when its context names none) and angle unit in radians.
    fn rep_unit(&mut self, rep: u64) -> (Option<LengthUnit>, f64) {
        let ctx = self.reps.get(&rep).and_then(|(_, c)| *c);
        let Some(ctx) = ctx else { return (None, 1.0) };
        if let Some(u) = self.units.get(&ctx) {
            return u.clone();
        }
        let mut length = None;
        let mut angle = 1.0;
        if let Some(units) = self
            .ents
            .record(ctx, "GLOBAL_UNIT_ASSIGNED_CONTEXT")
            .and_then(|p| p.first())
        {
            for u in refs(units) {
                if self.ents.is(u, "LENGTH_UNIT") {
                    match self.unit_size(u, "LENGTH", 0) {
                        Some(mm) => {
                            length = Some(LengthUnit {
                                name: unit_name(mm),
                                mm,
                            })
                        }
                        None => self.warn(
                            Some(u),
                            WarningKind::Unit,
                            "length unit not understood, mm assumed",
                        ),
                    }
                } else if self.ents.is(u, "PLANE_ANGLE_UNIT") {
                    angle = self.unit_size(u, "PLANE_ANGLE", 0).unwrap_or(1.0);
                }
            }
        }
        self.units.insert(ctx, (length.clone(), angle));
        (length, angle)
    }

    /// A unit's size: in mm for a length unit, in radians for an angle unit.
    fn unit_size(&self, u: u64, quantity: &str, depth: usize) -> Option<f64> {
        if depth > 4 {
            return None;
        }
        if let Some(si) = self.ents.record(u, "SI_UNIT") {
            let prefix = match si.first().and_then(crate::entities::enumeration) {
                None => 1.0,
                Some(p) => si_prefix(p)?,
            };
            return match si.get(1).and_then(crate::entities::enumeration)? {
                "METRE" => Some(prefix * 1000.0),
                "RADIAN" => Some(prefix),
                _ => None,
            };
        }
        let conv = self.ents.record(u, "CONVERSION_BASED_UNIT")?;
        let measure = conv.get(1).and_then(entity)?;
        let p = self
            .ents
            .simple(measure, &format!("{quantity}_MEASURE_WITH_UNIT"))?;
        let value = p.first().and_then(real)?;
        let base = p.get(1).and_then(entity)?;
        Some(value * self.unit_size(base, quantity, depth + 1)?)
    }

    fn visit_product(&mut self, pd: u64, t: Transform, colour: Option<Colour>, depth: usize) {
        if depth > MAX_DEPTH {
            self.warn(
                Some(pd),
                WarningKind::Placement,
                "assembly deeper than 32 levels, rest left out",
            );
            return;
        }
        let name = self.product_name(pd);
        let group = self.rep_group(pd);
        for &rep in &group {
            self.visit_rep(rep, t, &name, colour, depth);
        }
        let children = self.children.get(&pd).cloned().unwrap_or_default();
        for (occurrence, child) in children {
            let tn = match self.occurrence_transform.get(&occurrence).copied() {
                Some((rep1, rep2, a1, a2)) => {
                    let child_group = self.rep_group(child);
                    // rep_1 is the child's by the standard, but some writers swap them
                    let (child_ax, parent_ax, child_rep, parent_rep) =
                        if child_group.contains(&rep1) || !child_group.contains(&rep2) {
                            (a1, a2, rep1, rep2)
                        } else {
                            (a2, a1, rep2, rep1)
                        };
                    let cu = self.rep_unit(child_rep).0.map_or(1.0, |u| u.mm);
                    let pu = self.rep_unit(parent_rep).0.map_or(1.0, |u| u.mm);
                    match (self.ents.axis2(child_ax), self.ents.axis2(parent_ax)) {
                        (Some(c), Some(p)) => p
                            .scaled_translation(pu)
                            .compose(&c.scaled_translation(cu).inverse()),
                        _ => {
                            self.warn(
                                Some(occurrence),
                                WarningKind::Placement,
                                "transformation axes not read",
                            );
                            Transform::IDENTITY
                        }
                    }
                }
                None => {
                    self.warn(
                        Some(occurrence),
                        WarningKind::Placement,
                        "assembly occurrence without a transformation",
                    );
                    Transform::IDENTITY
                }
            };
            self.visit_product(child, t.compose(&tn), colour, depth + 1);
        }
    }

    fn product_name(&self, pd: u64) -> String {
        let ents = self.ents;
        let formation = ents
            .simple(pd, "PRODUCT_DEFINITION")
            .or_else(|| ents.simple(pd, "PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS"))
            .and_then(|p| p.get(2))
            .and_then(entity);
        let product = formation.and_then(|f| {
            ents.simple(f, "PRODUCT_DEFINITION_FORMATION")
                .or_else(|| ents.simple(f, "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE"))
                .and_then(|p| p.get(2))
                .and_then(entity)
        });
        let p = product.and_then(|p| ents.simple(p, "PRODUCT"));
        let name = p
            .and_then(|p| p.get(1))
            .and_then(string)
            .filter(|s| !s.is_empty());
        let id = p.and_then(|p| p.first()).and_then(string);
        name.or(id).unwrap_or_default().to_string()
    }

    fn visit_rep(
        &mut self,
        rep: u64,
        t: Transform,
        name: &str,
        colour: Option<Colour>,
        depth: usize,
    ) {
        let Some((items, _)) = self.reps.get(&rep).cloned() else {
            return;
        };
        let (unit, angle) = self.rep_unit(rep);
        let unit_mm = unit.map_or(1.0, |u| u.mm);
        for item in items {
            self.visit_item(item, t, unit_mm, angle, name, colour, depth);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn visit_item(
        &mut self,
        item: u64,
        t: Transform,
        unit_mm: f64,
        angle_rad: f64,
        name: &str,
        colour: Option<Colour>,
        depth: usize,
    ) {
        let ents = self.ents;
        if let Some(shells) = solid_shells(ents, item) {
            self.reached.insert(item);
            let own = ents.name(item).filter(|s| !s.is_empty());
            self.placed.push(Placed {
                name: if name.is_empty() {
                    own.unwrap_or_default().to_string()
                } else {
                    name.to_string()
                },
                solid: item,
                shells,
                transform: t,
                unit_mm,
                angle_rad,
                inherited_colour: colour,
            });
            return;
        }
        if let Some(p) = ents.simple(item, "MAPPED_ITEM") {
            if depth > MAX_DEPTH {
                return;
            }
            let colour = self.colours.get(&item).copied().or(colour);
            let (Some(map), Some(target)) = (p.get(1).and_then(entity), p.get(2).and_then(entity))
            else {
                return;
            };
            let Some(m) = ents.simple(map, "REPRESENTATION_MAP") else {
                return;
            };
            let (Some(origin), Some(mapped)) =
                (m.first().and_then(entity), m.get(1).and_then(entity))
            else {
                return;
            };
            let mapped_unit = self.rep_unit(mapped).0.map_or(1.0, |u| u.mm);
            let tm = match (ents.axis2(origin), ents.axis2(target)) {
                (Some(o), Some(g)) => g
                    .scaled_translation(unit_mm)
                    .compose(&o.scaled_translation(mapped_unit).inverse()),
                _ => {
                    self.warn(
                        Some(item),
                        WarningKind::Placement,
                        "mapped item placement not read (not an AXIS2_PLACEMENT_3D)",
                    );
                    Transform::IDENTITY
                }
            };
            self.visit_rep(mapped, t.compose(&tm), name, colour, depth + 1);
            return;
        }
        if let Some(k) = ents.keyword(item)
            && UNSUPPORTED.contains(&k)
        {
            self.warn(
                Some(item),
                WarningKind::Unsupported,
                format!("{k} is not read"),
            );
        }
    }

    /// Solids no product reaches, placed by their own coordinates in the unit of a representation that holds them.
    fn place_unreached(&mut self) {
        let ents = self.ents;
        let mut holder: HashMap<u64, u64> = HashMap::new();
        for (rep, (items, _)) in &self.reps {
            for i in items {
                holder.entry(*i).or_insert(*rep);
            }
        }
        let mut solids: Vec<u64> = ents
            .iter()
            .filter(|(_, e)| keyword_of(e).is_some_and(|k| SOLIDS.contains(&k)))
            .map(|(id, _)| id)
            .filter(|id| !self.reached.contains(id))
            .collect();
        solids.sort_unstable();
        let had_tree = !self.children.is_empty();
        for s in solids {
            let (unit, angle) = holder
                .get(&s)
                .map(|r| self.rep_unit(*r))
                .unwrap_or((None, 1.0));
            if had_tree {
                self.warn(
                    Some(s),
                    WarningKind::Placement,
                    "solid not reached by the assembly tree, placed as it is",
                );
            }
            self.visit_item(
                s,
                Transform::IDENTITY,
                unit.map_or(1.0, |u| u.mm),
                angle,
                "",
                None,
                0,
            );
        }
    }
}

fn si_prefix(p: &str) -> Option<f64> {
    Some(match p {
        "EXA" => 1e18,
        "PETA" => 1e15,
        "TERA" => 1e12,
        "GIGA" => 1e9,
        "MEGA" => 1e6,
        "KILO" => 1e3,
        "HECTO" => 1e2,
        "DECA" => 1e1,
        "DECI" => 1e-1,
        "CENTI" => 1e-2,
        "MILLI" => 1e-3,
        "MICRO" => 1e-6,
        "NANO" => 1e-9,
        "PICO" => 1e-12,
        _ => return None,
    })
}

/// The shells of a solid entity, or `None` when `item` is not a solid.
fn solid_shells(ents: &Entities<'_>, item: u64) -> Option<Vec<(u64, bool)>> {
    let shell = |id: u64| -> (u64, bool) {
        match ents
            .simple(id, "ORIENTED_CLOSED_SHELL")
            .or_else(|| ents.simple(id, "ORIENTED_OPEN_SHELL"))
        {
            // ORIENTED_CLOSED_SHELL(name, *, shell, orientation)
            Some(p) => (
                p.get(2).and_then(entity).unwrap_or(id),
                p.get(3).and_then(crate::entities::logical).unwrap_or(true),
            ),
            None => (id, true),
        }
    };
    if let Some(p) = ents.simple(item, "MANIFOLD_SOLID_BREP") {
        return Some(p.get(1).and_then(entity).map(shell).into_iter().collect());
    }
    if let Some(p) = ents.simple(item, "BREP_WITH_VOIDS") {
        let mut v: Vec<_> = p.get(1).and_then(entity).map(shell).into_iter().collect();
        v.extend(
            p.get(2)
                .map(refs)
                .unwrap_or_default()
                .into_iter()
                .map(shell),
        );
        return Some(v);
    }
    if let Some(p) = ents.simple(item, "SHELL_BASED_SURFACE_MODEL") {
        return Some(
            p.get(1)
                .map(refs)
                .unwrap_or_default()
                .into_iter()
                .map(shell)
                .collect(),
        );
    }
    None
}
