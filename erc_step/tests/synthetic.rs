//! STEP files written by the tests: a box and a cylinder through truck-stepio's writer, and hand-written files for
//! what truck does not read (units, assemblies, colours) and what breaks it (`$` names, a sphere smaller than the
//! tolerance).

use erc_step::{CurveKind, Model, Options, SurfaceKind, WarningKind, read_str};

fn read(text: &str) -> Model {
    read_str(text, &Options::default()).expect("reads")
}

fn assert_box(m: &Model, min: [f64; 3], max: [f64; 3]) {
    let b = m.bounds().expect("something meshed");
    for k in 0..3 {
        assert!(
            (b.min[k] - min[k]).abs() < 1e-6,
            "min {:?} != {min:?}",
            b.min
        );
        assert!(
            (b.max[k] - max[k]).abs() < 1e-6,
            "max {:?} != {max:?}",
            b.max
        );
    }
}

/// A STEP file built entity by entity.
struct Step {
    lines: Vec<String>,
}

#[derive(Clone, Copy)]
enum Unit {
    Mm,
    Metre,
    Inch,
}

impl Step {
    fn new() -> Self {
        Step { lines: Vec::new() }
    }

    fn add(&mut self, entity: impl Into<String>) -> u64 {
        self.lines.push(entity.into());
        self.lines.len() as u64
    }

    fn text(&self) -> String {
        let mut s = String::from(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('erc_step test'),'2;1');\n\
             FILE_NAME('test.step','2026-11-13T00:00:00',(''),(''),'','','');\n\
             FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));\nENDSEC;\nDATA;\n",
        );
        for (i, l) in self.lines.iter().enumerate() {
            s.push_str(&format!("#{}={l};\n", i + 1));
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

    /// A geometric context with this length unit (angles in radians).
    fn context(&mut self, unit: Unit) -> u64 {
        let length = match unit {
            Unit::Mm => self.add("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))"),
            Unit::Metre => self.add("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.))"),
            Unit::Inch => {
                let mm = self.add("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))");
                let m = self.add(format!(
                    "LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(25.4),#{mm})"
                ));
                let e = self.add("DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.)");
                self.add(format!(
                    "(CONVERSION_BASED_UNIT('INCH',#{m})LENGTH_UNIT()NAMED_UNIT(#{e}))"
                ))
            }
        };
        let angle = self.add("(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.))");
        let solid = self.add("(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT())");
        self.add(format!(
            "(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#{length},#{angle},#{solid}))\
             REPRESENTATION_CONTEXT('',''))"
        ))
    }

    /// A box B-rep from `min` to `max`; returns the solid and its faces (-x, +x, -y, +y, -z, +z). `face_name`
    /// is written as each face's name (`''` or `$`).
    fn cuboid(&mut self, min: [f64; 3], max: [f64; 3], face_name: &str) -> (u64, Vec<u64>) {
        let corner = |i: usize| [0, 1, 2].map(|k| if i >> k & 1 == 1 { max[k] } else { min[k] });
        let mut vertex = Vec::new();
        for i in 0..8 {
            let p = self.point(corner(i));
            vertex.push(self.add(format!("VERTEX_POINT('',#{p})")));
        }
        let mut edges = std::collections::HashMap::new();
        for a in 0..8usize {
            for k in 0..3 {
                let b = a | 1 << k;
                if b == a {
                    continue;
                }
                let (pa, pb) = (corner(a), corner(b));
                let len = pb[k] - pa[k];
                let mut d = [0.0; 3];
                d[k] = 1.0;
                let (p, dir) = (self.point(pa), self.direction(d));
                let v = self.add(format!("VECTOR('',#{dir},{len:?})"));
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
                // corners of the face counter-clockwise seen from outside
                let (i, j) = if side == 1 {
                    ((k + 1) % 3, (k + 2) % 3)
                } else {
                    ((k + 2) % 3, (k + 1) % 3)
                };
                let at = |a: usize, b: usize| side << k | a << i | b << j;
                let ring = [at(0, 0), at(1, 0), at(1, 1), at(0, 1)];
                let mut oriented = Vec::new();
                for n in 0..4 {
                    let (a, b) = (ring[n], ring[(n + 1) % 4]);
                    let (e, forward) = match edges.get(&(a, b)) {
                        Some(e) => (*e, true),
                        None => (edges[&(b, a)], false),
                    };
                    let f = if forward { "T" } else { "F" };
                    oriented.push(self.add(format!("ORIENTED_EDGE('',*,*,#{e},.{f}.)")));
                }
                let list = oriented
                    .iter()
                    .map(|o| format!("#{o}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let lp = self.add(format!("EDGE_LOOP('',({list}))"));
                let bound = self.add(format!("FACE_OUTER_BOUND('',#{lp},.T.)"));
                let mut normal = [0.0; 3];
                normal[k] = if side == 1 { 1.0 } else { -1.0 };
                let mut x = [0.0; 3];
                x[i] = 1.0;
                let ax = self.axis2(corner(ring[0]), normal, x);
                let plane = self.add(format!("PLANE('',#{ax})"));
                faces.push(self.add(format!(
                    "ADVANCED_FACE({face_name},(#{bound}),#{plane},.T.)"
                )));
            }
        }
        let list = faces
            .iter()
            .map(|f| format!("#{f}"))
            .collect::<Vec<_>>()
            .join(",");
        let shell = self.add(format!("CLOSED_SHELL('',({list}))"));
        (
            self.add(format!("MANIFOLD_SOLID_BREP('box',#{shell})")),
            faces,
        )
    }

    /// A product with a representation holding `items`; returns (product definition, representation, its origin
    /// placement).
    fn product(&mut self, name: &str, items: &[u64], ctx: u64) -> (u64, u64, u64) {
        let ac = self.add("APPLICATION_CONTEXT('automotive design')");
        let pc = self.add(format!("PRODUCT_CONTEXT('',#{ac},'mechanical')"));
        let p = self.add(format!("PRODUCT('{name}','{name}','',(#{pc}))"));
        let pdf = self.add(format!("PRODUCT_DEFINITION_FORMATION('','',#{p})"));
        let pdc = self.add(format!(
            "PRODUCT_DEFINITION_CONTEXT('part definition',#{ac},'design')"
        ));
        let pd = self.add(format!("PRODUCT_DEFINITION('design','',#{pdf},#{pdc})"));
        let pds = self.add(format!("PRODUCT_DEFINITION_SHAPE('','',#{pd})"));
        let origin = self.axis2([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
        let mut list = vec![format!("#{origin}")];
        list.extend(items.iter().map(|i| format!("#{i}")));
        let kind = if items.is_empty() {
            "SHAPE_REPRESENTATION"
        } else {
            "ADVANCED_BREP_SHAPE_REPRESENTATION"
        };
        let rep = self.add(format!("{kind}('',({}),#{ctx})", list.join(",")));
        self.add(format!("SHAPE_DEFINITION_REPRESENTATION(#{pds},#{rep})"));
        (pd, rep, origin)
    }

    /// Puts `child` into `parent`, moved to the frame `target` of the parent.
    fn occurrence(&mut self, parent: (u64, u64, u64), child: (u64, u64, u64), target: u64) {
        let nauo = self.add(format!(
            "NEXT_ASSEMBLY_USAGE_OCCURRENCE('1','','',#{},#{},$)",
            parent.0, child.0
        ));
        let pds = self.add(format!("PRODUCT_DEFINITION_SHAPE('','',#{nauo})"));
        let idt = self.add(format!(
            "ITEM_DEFINED_TRANSFORMATION('','',#{},#{target})",
            child.2
        ));
        let rel = self.add(format!(
            "(REPRESENTATION_RELATIONSHIP('','',#{},#{})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#{idt})\
             SHAPE_REPRESENTATION_RELATIONSHIP())",
            child.1, parent.1
        ));
        self.add(format!(
            "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#{rel},#{pds})"
        ));
    }

    /// A STYLED_ITEM giving `item` this colour.
    fn colour(&mut self, item: u64, rgb: [f64; 3], overriding: Option<u64>) -> u64 {
        let c = self.add(format!(
            "COLOUR_RGB('',{:?},{:?},{:?})",
            rgb[0], rgb[1], rgb[2]
        ));
        let fasc = self.add(format!("FILL_AREA_STYLE_COLOUR('',#{c})"));
        let fas = self.add(format!("FILL_AREA_STYLE('',(#{fasc}))"));
        let ssfa = self.add(format!("SURFACE_STYLE_FILL_AREA(#{fas})"));
        let sss = self.add(format!("SURFACE_SIDE_STYLE('',(#{ssfa}))"));
        let ssu = self.add(format!("SURFACE_STYLE_USAGE(.BOTH.,#{sss})"));
        let psa = self.add(format!("PRESENTATION_STYLE_ASSIGNMENT((#{ssu}))"));
        match overriding {
            Some(o) => self.add(format!("OVER_RIDING_STYLED_ITEM('',(#{psa}),#{item},#{o})")),
            None => self.add(format!("STYLED_ITEM('',(#{psa}),#{item})")),
        }
    }
}

fn single_box(unit: Unit, min: [f64; 3], max: [f64; 3], face_name: &str) -> String {
    let mut s = Step::new();
    let ctx = s.context(unit);
    let (solid, _) = s.cuboid(min, max, face_name);
    s.product("part", &[solid], ctx);
    s.text()
}

#[test]
fn box_in_mm_with_its_faces_and_edges() {
    let m = read(&single_box(Unit::Mm, [0.0; 3], [2.0, 3.0, 4.0], "''"));
    assert!(m.warnings.is_empty(), "{:?}", m.warnings);
    assert_eq!(m.unit.name, "mm");
    assert_eq!(m.parts.len(), 1);
    let part = &m.parts[0];
    assert_eq!(part.name, "part");
    assert_eq!(part.faces.len(), 6);
    assert_eq!(part.edges.len(), 12);
    assert_box(&m, [0.0; 3], [2.0, 3.0, 4.0]);
    // every face a plane with its normal out of the box, every triangle wound to match
    for f in &part.faces {
        let SurfaceKind::Plane { normal, origin } = f.surface else {
            panic!("{:?}", f.surface)
        };
        let mesh = f.mesh.as_ref().unwrap();
        for t in &mesh.triangles {
            let p = t.map(|i| mesh.positions[i as usize]);
            let n = erc_step::math::cross(
                erc_step::math::sub(p[1], p[0]),
                erc_step::math::sub(p[2], p[0]),
            );
            assert!(
                erc_step::math::dot(n, normal) > 0.0,
                "triangle wound against the normal"
            );
        }
        let centre_side = erc_step::math::dot(erc_step::math::sub([1.0, 1.5, 2.0], origin), normal);
        assert!(centre_side < 0.0, "normal points into the box");
    }
    for e in &part.edges {
        assert!(matches!(e.curve, CurveKind::Line { .. }));
        assert_eq!(e.faces.len(), 2);
        let len = erc_step::measure::edge_length(e).unwrap().value;
        assert!([2.0, 3.0, 4.0].iter().any(|l| (l - len).abs() < 1e-9));
    }
    // the box's opposite faces measure its size
    let plane = |i: usize| part.faces[i].surface.plane().unwrap();
    let m = erc_step::measure::plane_to_plane(plane(4), plane(5), [1.0, 1.0, 1.0]).unwrap();
    assert!((m.value - 4.0).abs() < 1e-9);
}

#[test]
fn inch_and_metre_are_scaled_to_mm() {
    let m = read(&single_box(Unit::Inch, [0.0; 3], [1.0, 0.5, 0.1], "''"));
    assert_eq!(m.unit.name, "inch");
    assert_box(&m, [0.0; 3], [25.4, 12.7, 2.54]);
    // the tolerance is in the file's unit: a metre model meshes to the same triangles, no stray vertices
    let m = read(&single_box(
        Unit::Metre,
        [0.0; 3],
        [0.002, 0.003, 0.0005],
        "''",
    ));
    assert_eq!(m.unit.name, "m");
    assert_eq!(m.unit.mm, 1000.0);
    assert_box(&m, [0.0; 3], [2.0, 3.0, 0.5]);
    assert!(m.warnings.is_empty(), "{:?}", m.warnings);
}

#[test]
fn unnamed_faces_are_read() {
    // `ADVANCED_FACE($,...)`: truck drops every such face without the text fix
    let m = read(&single_box(Unit::Mm, [0.0; 3], [1.0; 3], "$"));
    assert!(m.warnings.is_empty(), "{:?}", m.warnings);
    assert_eq!(m.face_count(), 6);
    assert!(m.parts[0].faces.iter().all(|f| f.mesh.is_some()));
}

#[test]
fn assembly_moves_its_parts() {
    let mut s = Step::new();
    let ctx = s.context(Unit::Mm);
    let (solid, _) = s.cuboid([0.0; 3], [2.0, 1.0, 0.5], "''");
    let pin = s.product("pin", &[solid], ctx);
    let target1 = s.axis2([10.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    // turned 90° about z: x goes to y
    let target2 = s.axis2([0.0, 20.0, 1.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]);
    let root = s.product("assembly", &[], ctx);
    // the representation relationship lists the parent's placements as items too
    s.occurrence(root, pin, target1);
    s.occurrence(root, pin, target2);
    let m = read(&s.text());
    assert!(m.warnings.is_empty(), "{:?}", m.warnings);
    assert_eq!(m.parts.len(), 2, "the pin once per use");
    let boxes: Vec<_> = m
        .parts
        .iter()
        .map(|p| {
            Model {
                parts: vec![p.clone()],
                ..Default::default()
            }
            .bounds()
            .unwrap()
        })
        .collect();
    let near = |a: [f64; 3], b: [f64; 3]| (0..3).all(|k| (a[k] - b[k]).abs() < 1e-6);
    assert!(
        boxes
            .iter()
            .any(|b| near(b.min, [10.0, 0.0, 0.0]) && near(b.max, [12.0, 1.0, 0.5])),
        "{boxes:?}"
    );
    assert!(
        boxes
            .iter()
            .any(|b| near(b.min, [-1.0, 20.0, 1.0]) && near(b.max, [0.0, 22.0, 1.5])),
        "{boxes:?}"
    );
    assert!(m.parts.iter().all(|p| p.name == "pin"));
    // the kinds move with the part: the turned pin's +x face looks along +y
    let turned = m
        .parts
        .iter()
        .find(|p| p.transform.translation[1] > 10.0)
        .unwrap();
    let SurfaceKind::Plane { normal, .. } = turned.faces[1].surface else {
        panic!()
    };
    assert!(near(normal, [0.0, 1.0, 0.0]), "{normal:?}");
}

/// An assembly of MAPPED_ITEMs without occurrences (some exporters write them so): each child product's solid once,
/// where the assembly maps it, not a second time as a product of its own
#[test]
fn mapped_products_are_not_roots() {
    let mut s = Step::new();
    let ctx = s.context(Unit::Mm);
    let (solid, _) = s.cuboid([0.0; 3], [2.0, 1.0, 0.5], "''");
    let pin = s.product("pin", &[solid], ctx);
    let map = s.add(format!("REPRESENTATION_MAP(#{},#{})", pin.2, pin.1));
    let target = s.axis2([10.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    let item = s.add(format!("MAPPED_ITEM('',#{map},#{target})"));
    s.product("assembly", &[item], ctx);
    let m = read(&s.text());
    assert!(m.warnings.is_empty(), "{:?}", m.warnings);
    assert_eq!(
        m.parts.len(),
        1,
        "{:?}",
        m.parts.iter().map(|p| &p.name).collect::<Vec<_>>()
    );
    assert_box(&m, [10.0, 0.0, 0.0], [12.0, 1.0, 0.5]);
}

#[test]
fn colours_of_solids_and_faces() {
    let mut s = Step::new();
    let ctx = s.context(Unit::Mm);
    let (solid, faces) = s.cuboid([0.0; 3], [1.0; 3], "''");
    let styled = s.colour(solid, [0.2, 0.2, 0.2], None);
    s.colour(faces[5], [1.0, 0.8, 0.0], Some(styled));
    s.product("part", &[solid], ctx);
    let m = read(&s.text());
    let part = &m.parts[0];
    assert_eq!(part.colour, Some([0.2, 0.2, 0.2]));
    assert_eq!(part.faces[5].colour, Some([1.0, 0.8, 0.0]));
    assert!(
        part.faces[..5]
            .iter()
            .all(|f| f.colour == Some([0.2, 0.2, 0.2]))
    );
}

#[test]
fn sphere_smaller_than_the_tolerance() {
    // a hemisphere of radius 0.004 mm closed by a disc; truck asserts on a sphere under the tolerance (0.01 mm)
    let mut s = Step::new();
    let ctx = s.context(Unit::Mm);
    let r = 0.004;
    let p = s.point([r, 0.0, 0.0]);
    let v = s.add(format!("VERTEX_POINT('',#{p})"));
    let ax = s.axis2([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    let circle = s.add(format!("CIRCLE('',#{ax},{r:?})"));
    let e = s.add(format!("EDGE_CURVE('',#{v},#{v},#{circle},.T.)"));
    let up = s.add(format!("ORIENTED_EDGE('',*,*,#{e},.T.)"));
    let down = s.add(format!("ORIENTED_EDGE('',*,*,#{e},.F.)"));
    let l1 = s.add(format!("EDGE_LOOP('',(#{up}))"));
    let l2 = s.add(format!("EDGE_LOOP('',(#{down}))"));
    let b1 = s.add(format!("FACE_OUTER_BOUND('',#{l1},.T.)"));
    let b2 = s.add(format!("FACE_OUTER_BOUND('',#{l2},.T.)"));
    let sax = s.axis2([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    let sphere = s.add(format!("SPHERICAL_SURFACE('',#{sax},{r:?})"));
    let pax = s.axis2([0.0; 3], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]);
    let plane = s.add(format!("PLANE('',#{pax})"));
    let f1 = s.add(format!("ADVANCED_FACE('',(#{b1}),#{sphere},.T.)"));
    let f2 = s.add(format!("ADVANCED_FACE('',(#{b2}),#{plane},.T.)"));
    let shell = s.add(format!("CLOSED_SHELL('',(#{f1},#{f2}))"));
    let solid = s.add(format!("MANIFOLD_SOLID_BREP('dome',#{shell})"));
    s.product("dome", &[solid], ctx);
    let m = read(&s.text());
    let part = &m.parts[0];
    assert_eq!(part.faces.len(), 2);
    assert_eq!(
        part.faces[0].surface,
        SurfaceKind::Sphere {
            centre: [0.0; 3],
            radius: r,
            outward: true
        }
    );
    assert!(matches!(part.edges[0].curve, CurveKind::Circle { radius, .. } if radius == r));
    // no panic; a face that does not mesh is a warning, never a lost model
    for (f, w) in part.faces.iter().zip([0, 1]) {
        assert!(
            f.mesh.is_some()
                || m.warnings
                    .iter()
                    .any(|x| x.kind == WarningKind::FaceNotMeshed),
            "face {w} neither meshed nor warned"
        );
    }
    assert!(part.faces[0].mesh.is_some(), "{:?}", m.warnings);
}

mod written_by_truck {
    use super::*;
    use truck_modeling::*;

    fn step_of(solid: &Solid) -> String {
        let compressed = solid.compress();
        let design = truck_stepio::out::StepDesign::from_model(truck_stepio::out::StepModel::from(
            &compressed,
        ));
        truck_stepio::out::StepDisplay::new(
            truck_stepio::out::StepHeaderDescriptor::default(),
            design,
        )
        .to_string()
    }

    #[test]
    fn box_from_truck() {
        let v = builder::vertex(Point3::new(0.0, 0.0, 0.0));
        let e = builder::tsweep(&v, Vector3::unit_x() * 2.0);
        let f = builder::tsweep(&e, Vector3::unit_y() * 3.0);
        let solid = builder::tsweep(&f, Vector3::unit_z() * 4.0);
        let m = read(&step_of(&solid));
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
        assert_eq!(m.face_count(), 6);
        assert_box(&m, [0.0; 3], [2.0, 3.0, 4.0]);
    }

    #[test]
    fn cylinder_from_truck() {
        let v = builder::vertex(Point3::new(1.5, 0.0, 0.0));
        let circle = builder::rsweep(&v, Point3::origin(), Vector3::unit_z(), Rad(7.0), 4);
        let disc = builder::try_attach_plane(&[circle]).unwrap();
        let solid = builder::tsweep(&disc, Vector3::unit_z() * 2.0);
        let m = read(&step_of(&solid));
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
        // the sweep in 4 parts: 4 side faces and the 2 discs
        assert_eq!(m.face_count(), 6);
        let b = m.bounds().unwrap();
        // the polygon of a circle lies inside it, within the tolerance
        for k in 0..2 {
            assert!(b.max[k] <= 1.5 + 1e-9 && b.max[k] > 1.5 - 0.011, "{b:?}");
            assert!(b.min[k] >= -1.5 - 1e-9 && b.min[k] < -1.5 + 0.011, "{b:?}");
        }
        assert!((b.min[2] - 0.0).abs() < 1e-9 && (b.max[2] - 2.0).abs() < 1e-9);
    }
}

#[test]
fn budget_run_out_leaves_faces_unmeshed_with_warnings() {
    let opts = Options {
        budget: std::time::Duration::ZERO,
        ..Options::default()
    };
    let m = read_str(&single_box(Unit::Mm, [0.0; 3], [1.0; 3], "''"), &opts).unwrap();
    assert_eq!(m.face_count(), 6, "the faces and their kinds stay");
    assert!(
        m.parts[0]
            .faces
            .iter()
            .all(|f| f.mesh.is_none() && f.surface.plane().is_some())
    );
    let budget = m
        .warnings
        .iter()
        .filter(|w| w.kind == WarningKind::FaceNotMeshed && w.message.contains("budget"));
    assert_eq!(budget.count(), 6, "{:?}", m.warnings);
}

#[test]
fn not_a_step_file() {
    assert!(matches!(
        read_str("hello", &Options::default()),
        Err(erc_step::ReadError::Parse(_))
    ));
    let r = erc_step::read_bytes(
        b"ISO-10303-21;\nHEADER;\nENDSEC;\n".to_vec(),
        &Options::default(),
    );
    assert!(matches!(r, Err(erc_step::ReadError::Parse(_))), "{r:?}");
}
