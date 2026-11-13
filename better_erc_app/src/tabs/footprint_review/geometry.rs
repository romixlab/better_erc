//! A datasheet page's drawing as the ruler sees it: the straight lines the PDF draws (each one's centre line and
//! width) and its words with their boxes, read from the PDF's own paths and glyphs ([`super::pdf`]), in page points,
//! y down. A ruler end snaps to them: to a line's end or the crossing of two lines, else onto the nearest line; and
//! the dimension's number printed between the ruler's ends is found among the words.

/// A straight piece of a line the page draws: its centre line and its width, points
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seg {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub width: f32,
}

/// A word on the page: its text and the box its glyphs' ink fills
#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub text: String,
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl Word {
    pub fn center(&self) -> [f32; 2] {
        [
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
        ]
    }
}

/// A page's lines and words, and the pieces it draws ([`super::prims`])
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageGeometry {
    pub segs: Vec<Seg>,
    pub words: Vec<Word>,
    pub prims: Vec<super::prims::Prim>,
}

/// What a ruler end snapped to
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapKind {
    /// A line's end
    End,
    /// Two lines crossing
    Crossing,
    /// Onto a line (on a level ruler: the line across it for one coordinate, the line along it for the other)
    Line,
    /// A footprint pad's centre (measuring the footprint)
    PadCentre,
    /// The middle of a rectangular pad's side: where its centre cross meets its edge
    PadSide,
    /// A corner of a 3D model view's outline or the end of one of its edges (VIEW-24)
    ModelCorner,
}

impl SnapKind {
    /// What a ruler end snapped to, in words
    pub fn name(self) -> &'static str {
        match self {
            SnapKind::End => "line end",
            SnapKind::Crossing => "crossing",
            SnapKind::Line => "line",
            SnapKind::PadCentre => "pad centre",
            SnapKind::PadSide => "pad side",
            SnapKind::ModelCorner => "model corner",
        }
    }
}

/// Where a ruler end snapped to, and the lines that made it (indices into [`PageGeometry::segs`]), to show them
#[derive(Clone, Debug, PartialEq)]
pub struct Snap {
    pub at: [f32; 2],
    pub kind: SnapKind,
    pub segs: Vec<usize>,
}

/// The number a dimension prints, found between the ruler's ends
#[derive(Clone, Debug, PartialEq)]
pub struct DimNumber {
    /// As printed, without what goes around it: `2.00`
    pub text: String,
    pub value: f64,
    /// The word's box
    pub min: [f32; 2],
    pub max: [f32; 2],
}

fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn cross(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    let d = sub(a, b);
    d[0].hypot(d[1])
}

impl Seg {
    /// The point of the centre line nearest `p`
    pub fn nearest(&self, p: [f32; 2]) -> [f32; 2] {
        let d = sub(self.b, self.a);
        let l2 = dot(d, d);
        if l2 <= f32::EPSILON {
            return self.a;
        }
        let t = (dot(sub(p, self.a), d) / l2).clamp(0.0, 1.0);
        [self.a[0] + d[0] * t, self.a[1] + d[1] * t]
    }

    /// How far `p` is from the line's edge (0 on it)
    pub fn distance(&self, p: [f32; 2]) -> f32 {
        (dist(self.nearest(p), p) - self.width / 2.0).max(0.0)
    }

    /// More vertical than horizontal
    fn steep(&self) -> bool {
        (self.b[1] - self.a[1]).abs() > (self.b[0] - self.a[0]).abs()
    }

    /// The centre line's X at height `y` (a steep line), kept within the line's ends
    fn x_at(&self, y: f32) -> f32 {
        let dy = self.b[1] - self.a[1];
        let t = ((y - self.a[1]) / dy).clamp(0.0, 1.0);
        self.a[0] + (self.b[0] - self.a[0]) * t
    }

    /// The centre line's Y at `x` (a shallow line), kept within the line's ends
    fn y_at(&self, x: f32) -> f32 {
        let dx = self.b[0] - self.a[0];
        if dx.abs() <= f32::EPSILON {
            return self.a[1];
        }
        let t = ((x - self.a[0]) / dx).clamp(0.0, 1.0);
        self.a[1] + (self.b[1] - self.a[1]) * t
    }

    /// Where the centre lines of `self` and `o` cross, when they do (not parallel, within both)
    fn crossing(&self, o: &Seg) -> Option<[f32; 2]> {
        let (r, s) = (sub(self.b, self.a), sub(o.b, o.a));
        let den = cross(r, s);
        if den.abs() <= 1e-6 * (dot(r, r) * dot(s, s)).sqrt().max(1e-12) {
            return None;
        }
        let q = sub(o.a, self.a);
        let (t, u) = (cross(q, s) / den, cross(q, r) / den);
        let slack = 1e-4;
        ((-slack..=1.0 + slack).contains(&t) && (-slack..=1.0 + slack).contains(&u))
            .then(|| [self.a[0] + r[0] * t, self.a[1] + r[1] * t])
    }
}

impl PageGeometry {
    /// Where a ruler end at `p` snaps within `radius` (points): the nearest line end or crossing of two lines;
    /// else, on a level ruler, each coordinate on its own: X from the nearest steep line (on a horizontal ruler the
    /// dimension's extension line), Y from the nearest shallow one; else onto
    /// the nearest line. None: no line that near.
    pub fn snap(&self, p: [f32; 2], radius: f32, level: bool) -> Option<Snap> {
        if radius <= 0.0 {
            return None;
        }
        let near: Vec<usize> = (0..self.segs.len())
            .filter(|&i| self.segs[i].distance(p) <= radius)
            .collect();
        if near.is_empty() {
            return None;
        }
        // Points first: line ends, then crossings of two near lines, the nearest one
        let mut best: Option<(f32, Snap)> = None;
        let mut offer = |at: [f32; 2], kind: SnapKind, segs: Vec<usize>| {
            let d = dist(at, p);
            if d <= radius && best.as_ref().is_none_or(|(bd, _)| d < *bd) {
                best = Some((d, Snap { at, kind, segs }));
            }
        };
        for &i in &near {
            let s = &self.segs[i];
            for e in [s.a, s.b] {
                offer(e, SnapKind::End, vec![i]);
            }
        }
        for (n, &i) in near.iter().enumerate() {
            for &j in &near[n + 1..] {
                if let Some(at) = self.segs[i].crossing(&self.segs[j]) {
                    // A crossing at a line's own end is that end (a corner of a polyline)
                    let end = |k: usize| {
                        let s = &self.segs[k];
                        dist(at, s.a) < 1e-3 || dist(at, s.b) < 1e-3
                    };
                    if !(end(i) && end(j)) {
                        offer(at, SnapKind::Crossing, vec![i, j]);
                    }
                }
            }
        }
        if let Some((_, s)) = best {
            return Some(s);
        }
        let nearest = |steep: Option<bool>| {
            near.iter()
                .copied()
                .filter(|&i| steep.is_none_or(|st| self.segs[i].steep() == st))
                .min_by(|&i, &j| {
                    self.segs[i]
                        .distance(p)
                        .total_cmp(&self.segs[j].distance(p))
                })
        };
        match level {
            // X from the nearest steep line, Y from the nearest shallow one: on a horizontal ruler the line across
            // it (an extension line) sets the length, on a vertical one likewise
            true => {
                let xs = nearest(Some(true));
                let ys = nearest(Some(false));
                let at = [
                    xs.map_or(p[0], |i| self.segs[i].x_at(p[1])),
                    ys.map_or(p[1], |i| self.segs[i].y_at(p[0])),
                ];
                Some(Snap {
                    at,
                    kind: SnapKind::Line,
                    segs: xs.into_iter().chain(ys).collect(),
                })
            }
            false => {
                let i = nearest(None)?;
                Some(Snap {
                    at: self.segs[i].nearest(p),
                    kind: SnapKind::Line,
                    segs: vec![i],
                })
            }
        }
    }

    /// The dimension's number for a ruler from `a` to `b`: of the words that read as a length and whose middle
    /// lies between the ends (across the ruler, within its length or 30 points of it), the one nearest the
    /// ruler's middle, the distance across counting fully and along it half
    pub fn dimension_number(&self, a: [f32; 2], b: [f32; 2]) -> Option<DimNumber> {
        let d = sub(b, a);
        let len = d[0].hypot(d[1]);
        if len <= f32::EPSILON {
            return None;
        }
        let u = [d[0] / len, d[1] / len];
        let reach = len.max(30.0);
        self.words
            .iter()
            .filter_map(|w| {
                let (text, value) = length_number(&w.text)?;
                let c = sub(w.center(), a);
                let along = dot(c, u);
                let across = cross(u, c).abs();
                if !(0.0..=len).contains(&along) || across > reach {
                    return None;
                }
                let score = across + (along - len / 2.0).abs() / 2.0;
                Some((
                    score,
                    DimNumber {
                        text,
                        value,
                        min: w.min,
                        max: w.max,
                    },
                ))
            })
            .min_by(|x, y| x.0.total_cmp(&y.0))
            .map(|(_, n)| n)
    }
}

/// A word read as a length: a positive number (a comma for the decimal point too), with a diameter or radius sign,
/// brackets or a tolerance around it, not a count (`4X`), a name (`SOT23`), a document number (`4223102/D`) or a ratio; the number as printed and its value
pub fn length_number(word: &str) -> Option<(String, f64)> {
    let w = word.trim();
    let w = w.trim_start_matches(['(', '[', 'Ø', 'ø', '⌀', '∅', 'R', '±']);
    let end = w
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || *c == '.' || *c == ','))
        .map_or(w.len(), |(i, _)| i);
    let (num, rest) = w.split_at(end);
    let num = num.trim_end_matches(['.', ',']);
    if !num.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    // A letter straight after is a count (4X), a name (SOT23) or a unit other than mm
    let next = rest.chars().next();
    if next.is_some_and(|c| c.is_alphabetic()) && !rest.to_ascii_lowercase().starts_with("mm") {
        return None;
    }
    // A document number or date (4223102/D, 03/2022) or a ratio (20:1)
    if next.is_some_and(|c| matches!(c, '/' | ':')) {
        return None;
    }
    let value: f64 = num.replace(',', ".").parse().ok()?;
    (value > 0.0).then(|| (num.to_string(), value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(a: [f32; 2], b: [f32; 2]) -> Seg {
        Seg { a, b, width: 0.5 }
    }

    /// A horizontal dimension 100 points long: its line at y 200 with a gap for the text, the extension lines at
    /// x 100 and 200 from the part above down past the dimension line, the number between them
    fn dimension() -> PageGeometry {
        PageGeometry {
            segs: vec![
                seg([100.0, 200.0], [140.0, 200.0]),
                seg([160.0, 200.0], [200.0, 200.0]),
                seg([100.0, 150.0], [100.0, 205.0]),
                seg([200.0, 150.0], [200.0, 205.0]),
            ],
            words: vec![
                Word {
                    text: "2.00".into(),
                    min: [142.0, 195.0],
                    max: [158.0, 203.0],
                },
                Word {
                    text: "4X".into(),
                    min: [120.0, 210.0],
                    max: [130.0, 218.0],
                },
                Word {
                    text: "0.50".into(),
                    min: [300.0, 195.0],
                    max: [316.0, 203.0],
                },
            ],
            prims: Vec::new(),
        }
    }

    #[test]
    fn snaps_to_ends_and_crossings_first() {
        let g = dimension();
        // Near the corner where the dimension line meets the extension line
        let s = g.snap([102.0, 198.0], 5.0, false).unwrap();
        assert_eq!(s.at, [100.0, 200.0]);
        // A line's end
        let s = g.snap([201.0, 204.0], 3.0, false).unwrap();
        assert_eq!((s.at, s.kind), ([200.0, 205.0], SnapKind::End));
        // A crossing in a line's middle
        let g2 = PageGeometry {
            segs: vec![
                seg([0.0, 50.0], [100.0, 50.0]),
                seg([50.0, 0.0], [50.0, 100.0]),
            ],
            words: Vec::new(),
            prims: Vec::new(),
        };
        let s = g2.snap([52.0, 48.0], 5.0, false).unwrap();
        assert_eq!((s.at, s.kind), ([50.0, 50.0], SnapKind::Crossing));
        assert_eq!(s.segs, [0, 1]);
    }

    #[test]
    fn snaps_onto_the_nearest_line() {
        let g = dimension();
        let s = g.snap([120.0, 202.0], 5.0, false).unwrap();
        assert_eq!((s.at, s.kind), ([120.0, 200.0], SnapKind::Line));
        assert!(g.snap([120.0, 210.0], 5.0, false).is_none(), "too far");
        assert!(
            g.snap([120.0, 202.0], 0.0, false).is_none(),
            "radius 0: off"
        );
    }

    /// A line's width counts: the pointer on the edge of a thick line is on it
    #[test]
    fn a_thick_line_is_near_from_its_edge() {
        let g = PageGeometry {
            segs: vec![Seg {
                a: [0.0, 100.0],
                b: [200.0, 100.0],
                width: 6.0,
            }],
            words: Vec::new(),
            prims: Vec::new(),
        };
        let s = g.snap([50.0, 107.0], 5.0, false).unwrap();
        assert_eq!(s.at, [50.0, 100.0], "the centre line");
    }

    /// On a horizontal ruler an end takes the X of the extension line across it, even beside the dimension line,
    /// and the Y of the line along it
    #[test]
    fn a_level_ruler_snaps_each_coordinate() {
        let g = dimension();
        let s = g.snap([203.0, 170.0], 5.0, true).unwrap();
        assert_eq!(s.at, [200.0, 170.0]);
        assert_eq!(s.segs, [3]);
        // Near an extension line and a pad's edge across it, their crossing beyond reach: both on a level
        // ruler, the nearer one alone at any angle
        let g = PageGeometry {
            segs: vec![
                seg([200.0, 100.0], [200.0, 250.0]),
                seg([150.0, 170.0], [250.0, 170.0]),
            ],
            words: Vec::new(),
            prims: Vec::new(),
        };
        let s = g.snap([204.0, 174.5], 5.0, true).unwrap();
        assert_eq!((s.at, s.kind), ([200.0, 170.0], SnapKind::Line));
        assert_eq!(s.segs, [0, 1]);
        let s = g.snap([204.0, 174.5], 5.0, false).unwrap();
        assert_eq!(s.at, [200.0, 174.5]);
    }

    #[test]
    fn the_number_between_the_ends() {
        let g = dimension();
        let n = g.dimension_number([100.0, 200.0], [200.0, 200.0]).unwrap();
        assert_eq!((n.text.as_str(), n.value), ("2.00", 2.0));
        // Measured at the part's height, the number below is still between the ends
        let n = g.dimension_number([100.0, 160.0], [200.0, 160.0]).unwrap();
        assert_eq!(n.text, "2.00");
        // Not between them
        assert!(g.dimension_number([300.0, 0.0], [300.0, 50.0]).is_none());
    }

    #[test]
    fn words_read_as_lengths() {
        for (w, want) in [
            ("2.00", Some("2.00")),
            ("0,65", Some("0,65")),
            ("(1.27)", Some("1.27")),
            ("Ø0.30", Some("0.30")),
            ("R0.25", Some("0.25")),
            ("0.50±0.05", Some("0.50")),
            ("3mm", Some("3")),
            ("4X", None),
            ("0402", Some("0402")),
            ("R1", Some("1")),
            ("SOT23", None),
            ("4223102/D", None),
            ("17.6:1", None),
            ("Package", None),
            ("0", None),
            (".", None),
        ] {
            assert_eq!(length_number(w).map(|n| n.0), want.map(String::from), "{w}");
        }
    }
}
