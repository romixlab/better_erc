//! Small vector and rigid-transform maths on `[f64; 3]`, so the public types need no maths crate.

/// A point or a vector, in mm in the model frame unless said otherwise.
pub type Vec3 = [f64; 3];

pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scale(a: Vec3, s: f64) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn norm(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}

pub fn distance(a: Vec3, b: Vec3) -> f64 {
    norm(sub(a, b))
}

/// `a` scaled to length 1; `None` for a zero or non-finite vector.
pub fn normalize(a: Vec3) -> Option<Vec3> {
    let n = norm(a);
    (n > 1e-300 && n.is_finite()).then(|| scale(a, 1.0 / n))
}

pub fn midpoint(a: Vec3, b: Vec3) -> Vec3 {
    scale(add(a, b), 0.5)
}

/// Any unit vector perpendicular to the unit vector `a`.
pub fn perpendicular(a: Vec3) -> Vec3 {
    let other = if a[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    normalize(cross(a, other)).unwrap_or([0.0, 0.0, 1.0])
}

/// A rigid transform (rotation, then translation). `rot` holds the images of the x, y and z axes as columns:
/// `rot[row][col]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub rot: [[f64; 3]; 3],
    pub translation: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Transform = Transform {
        rot: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        translation: [0.0; 3],
    };

    /// The frame with origin `origin` and the unit axes `x`, `y`, `z`: maps local coordinates into the outer frame.
    pub fn from_frame(origin: Vec3, x: Vec3, y: Vec3, z: Vec3) -> Transform {
        Transform {
            rot: [[x[0], y[0], z[0]], [x[1], y[1], z[1]], [x[2], y[2], z[2]]],
            translation: origin,
        }
    }

    pub fn apply_point(&self, p: Vec3) -> Vec3 {
        add(self.apply_vector(p), self.translation)
    }

    pub fn apply_vector(&self, v: Vec3) -> Vec3 {
        let r = &self.rot;
        [
            r[0][0] * v[0] + r[0][1] * v[1] + r[0][2] * v[2],
            r[1][0] * v[0] + r[1][1] * v[1] + r[1][2] * v[2],
            r[2][0] * v[0] + r[2][1] * v[1] + r[2][2] * v[2],
        ]
    }

    /// `self` after `inner`: `self.compose(inner).apply_point(p) == self.apply_point(inner.apply_point(p))`.
    pub fn compose(&self, inner: &Transform) -> Transform {
        let mut rot = [[0.0; 3]; 3];
        for (i, row) in rot.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.rot[i][k] * inner.rot[k][j]).sum();
            }
        }
        Transform {
            rot,
            translation: self.apply_point(inner.translation),
        }
    }

    /// The inverse, for a rotation part that is orthonormal.
    pub fn inverse(&self) -> Transform {
        let mut rot = [[0.0; 3]; 3];
        for (i, row) in rot.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = self.rot[j][i];
            }
        }
        let t = Transform {
            rot,
            translation: [0.0; 3],
        };
        let back = t.apply_vector(self.translation);
        Transform {
            rot,
            translation: scale(back, -1.0),
        }
    }

    /// The same transform with its translation multiplied by `s` (a unit change of the outer frame's lengths).
    pub fn scaled_translation(&self, s: f64) -> Transform {
        Transform {
            rot: self.rot,
            translation: scale(self.translation, s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        distance(a, b) < 1e-12
    }

    #[test]
    fn compose_and_inverse() {
        // 90° about z, then moved by (1, 2, 3)
        let t = Transform::from_frame(
            [1.0, 2.0, 3.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
        );
        assert!(close(t.apply_point([1.0, 0.0, 0.0]), [1.0, 3.0, 3.0]));
        let back = t.inverse();
        assert!(close(
            back.apply_point(t.apply_point([0.3, -0.7, 2.0])),
            [0.3, -0.7, 2.0]
        ));
        let tt = t.compose(&t);
        assert!(close(
            tt.apply_point([1.0, 0.0, 0.0]),
            t.apply_point(t.apply_point([1.0, 0.0, 0.0]))
        ));
    }
}
