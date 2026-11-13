//! A small STEP writer for tests and fixtures: axis-aligned boxes in mm, each a solid of its own, in one product.
//! Enough to stand in for a package model (a body and its leads) where a vendor file may not be committed.

use std::collections::HashMap;
use std::fmt::Write as _;

/// An axis-aligned box from `min` to `max`, mm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cuboid {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Cuboid {
    pub fn new(min: [f64; 3], max: [f64; 3]) -> Cuboid {
        Cuboid { min, max }
    }
}

/// A STEP file (AP214) of one product `name` holding each box as a solid, in mm.
pub fn cuboids(name: &str, boxes: &[Cuboid]) -> String {
    let mut s = Writer::default();
    let length = s.add("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))");
    let angle = s.add("(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.))");
    let solid = s.add("(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT())");
    let ctx = s.add(format!(
        "(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#{length},#{angle},#{solid}))\
         REPRESENTATION_CONTEXT('',''))"
    ));
    let solids: Vec<u64> = boxes
        .iter()
        .enumerate()
        .map(|(i, b)| s.cuboid(&format!("{name}_{}", i + 1), b))
        .collect();
    let ac = s.add("APPLICATION_CONTEXT('automotive design')");
    let pc = s.add(format!("PRODUCT_CONTEXT('',#{ac},'mechanical')"));
    let p = s.add(format!("PRODUCT('{name}','{name}','',(#{pc}))"));
    let pdf = s.add(format!("PRODUCT_DEFINITION_FORMATION('','',#{p})"));
    let pdc = s.add(format!(
        "PRODUCT_DEFINITION_CONTEXT('part definition',#{ac},'design')"
    ));
    let pd = s.add(format!("PRODUCT_DEFINITION('design','',#{pdf},#{pdc})"));
    let pds = s.add(format!("PRODUCT_DEFINITION_SHAPE('','',#{pd})"));
    let origin = s.axis2([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    let mut items = format!("#{origin}");
    for id in &solids {
        let _ = write!(items, ",#{id}");
    }
    let rep = s.add(format!(
        "ADVANCED_BREP_SHAPE_REPRESENTATION('',({items}),#{ctx})"
    ));
    s.add(format!("SHAPE_DEFINITION_REPRESENTATION(#{pds},#{rep})"));
    s.text(name)
}

#[derive(Default)]
struct Writer {
    lines: Vec<String>,
}

impl Writer {
    fn add(&mut self, entity: impl Into<String>) -> u64 {
        self.lines.push(entity.into());
        self.lines.len() as u64
    }

    fn text(&self, name: &str) -> String {
        let mut s = format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('erc_step cuboids'),'2;1');\n\
             FILE_NAME('{name}.step','2026-01-01T00:00:00',(''),(''),'','','');\n\
             FILE_SCHEMA(('AUTOMOTIVE_DESIGN {{ 1 0 10303 214 1 1 1 1 }}'));\nENDSEC;\nDATA;\n"
        );
        for (i, l) in self.lines.iter().enumerate() {
            let _ = writeln!(s, "#{}={l};", i + 1);
        }
        s.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        s
    }

    fn point(&mut self, p: [f64; 3]) -> u64 {
        self.add(format!(
            "CARTESIAN_POINT('',({:?},{:?},{:?}))",
            p[0], p[1], p[2]
        ))
    }

    fn direction(&mut self, d: [f64; 3]) -> u64 {
        self.add(format!("DIRECTION('',({:?},{:?},{:?}))", d[0], d[1], d[2]))
    }

    fn axis2(&mut self, origin: [f64; 3], z: [f64; 3], x: [f64; 3]) -> u64 {
        let (o, z, x) = (self.point(origin), self.direction(z), self.direction(x));
        self.add(format!("AXIS2_PLACEMENT_3D('',#{o},#{z},#{x})"))
    }

    /// A box B-rep; returns the solid.
    fn cuboid(&mut self, name: &str, b: &Cuboid) -> u64 {
        let (min, max) = (b.min, b.max);
        let corner = |i: usize| [0, 1, 2].map(|k| if i >> k & 1 == 1 { max[k] } else { min[k] });
        let vertex: Vec<u64> = (0..8)
            .map(|i| {
                let p = self.point(corner(i));
                self.add(format!("VERTEX_POINT('',#{p})"))
            })
            .collect();
        let mut edges = HashMap::new();
        for a in 0..8usize {
            for k in 0..3 {
                let b = a | 1 << k;
                if b == a {
                    continue;
                }
                let (pa, pb) = (corner(a), corner(b));
                let mut d = [0.0; 3];
                d[k] = 1.0;
                let (p, dir) = (self.point(pa), self.direction(d));
                let v = self.add(format!("VECTOR('',#{dir},{:?})", pb[k] - pa[k]));
                let line = self.add(format!("LINE('',#{p},#{v})"));
                let e = self.add(format!(
                    "EDGE_CURVE('',#{},#{},#{line},.T.)",
                    vertex[a], vertex[b]
                ));
                edges.insert((a, b), e);
            }
        }
        let mut faces = Vec::new();
        for k in 0..3 {
            for side in 0..2usize {
                // The corners of the face counter-clockwise seen from outside
                let (i, j) = if side == 1 {
                    ((k + 1) % 3, (k + 2) % 3)
                } else {
                    ((k + 2) % 3, (k + 1) % 3)
                };
                let at = |a: usize, b: usize| side << k | a << i | b << j;
                let ring = [at(0, 0), at(1, 0), at(1, 1), at(0, 1)];
                let mut list = Vec::new();
                for n in 0..4 {
                    let (a, b) = (ring[n], ring[(n + 1) % 4]);
                    let (e, f) = match edges.get(&(a, b)) {
                        Some(e) => (*e, "T"),
                        None => (edges[&(b, a)], "F"),
                    };
                    list.push(format!(
                        "#{}",
                        self.add(format!("ORIENTED_EDGE('',*,*,#{e},.{f}.)"))
                    ));
                }
                let lp = self.add(format!("EDGE_LOOP('',({}))", list.join(",")));
                let bound = self.add(format!("FACE_OUTER_BOUND('',#{lp},.T.)"));
                let mut normal = [0.0; 3];
                normal[k] = if side == 1 { 1.0 } else { -1.0 };
                let mut x = [0.0; 3];
                x[i] = 1.0;
                let ax = self.axis2(corner(ring[0]), normal, x);
                let plane = self.add(format!("PLANE('',#{ax})"));
                faces.push(format!(
                    "#{}",
                    self.add(format!("ADVANCED_FACE('',(#{bound}),#{plane},.T.)"))
                ));
            }
        }
        let shell = self.add(format!("CLOSED_SHELL('',({}))", faces.join(",")));
        self.add(format!("MANIFOLD_SOLID_BREP('{name}',#{shell})"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boxes_read_back() {
        let text = cuboids(
            "pkg",
            &[
                Cuboid::new([-1.0, -0.5, 0.1], [1.0, 0.5, 0.6]),
                Cuboid::new([-1.2, -0.2, 0.0], [-0.8, 0.2, 0.1]),
            ],
        );
        let m = crate::read_str(&text, &crate::Options::default()).unwrap();
        assert_eq!(m.parts.len(), 2);
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
        let b = m.bounds().unwrap();
        assert_eq!((b.min, b.max), ([-1.2, -0.5, 0.0], [1.0, 0.5, 0.6]));
    }
}
