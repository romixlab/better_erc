//! How far apart two colours look: CIEDE2000 (Sharma, Wu, Dalal 2005) on CIELAB under D65, from sRGB.

use erc_model::geometry::Colour;

/// CIELAB of an sRGB colour, alpha left out
fn lab(c: Colour) -> [f64; 3] {
    let linear = |v: u8| {
        let v = v as f64 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (linear(c.r), linear(c.g), linear(c.b));
    let x = (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / 0.95047;
    let y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
    let z = (0.0193339 * r + 0.119192 * g + 0.9503041 * b) / 1.08883;
    let f = |t: f64| {
        if t > 216.0 / 24389.0 {
            t.cbrt()
        } else {
            (24389.0 / 27.0 * t + 16.0) / 116.0
        }
    };
    [
        116.0 * f(y) - 16.0,
        500.0 * (f(x) - f(y)),
        200.0 * (f(y) - f(z)),
    ]
}

/// CIEDE2000 between two CIELAB colours
fn ciede2000([l1, a1, b1]: [f64; 3], [l2, a2, b2]: [f64; 3]) -> f64 {
    const POW25_7: f64 = 6103515625.0;
    let g_of = |c: f64| 0.5 * (1.0 - (c.powi(7) / (c.powi(7) + POW25_7)).sqrt());
    let g = g_of((a1.hypot(b1) + a2.hypot(b2)) / 2.0);
    let (a1, a2) = ((1.0 + g) * a1, (1.0 + g) * a2);
    let (c1, c2) = (a1.hypot(b1), a2.hypot(b2));
    let hue = |a: f64, b: f64| {
        if a == 0.0 && b == 0.0 {
            0.0
        } else {
            b.atan2(a).to_degrees().rem_euclid(360.0)
        }
    };
    let (h1, h2) = (hue(a1, b1), hue(a2, b2));
    let achromatic = c1 * c2 == 0.0;
    let dh = if achromatic {
        0.0
    } else if (h2 - h1).abs() <= 180.0 {
        h2 - h1
    } else if h2 > h1 {
        h2 - h1 - 360.0
    } else {
        h2 - h1 + 360.0
    };
    let d_big_h = 2.0 * (c1 * c2).sqrt() * (dh / 2.0).to_radians().sin();
    let (d_l, d_c) = (l2 - l1, c2 - c1);
    let (l_mean, c_mean) = ((l1 + l2) / 2.0, (c1 + c2) / 2.0);
    let h_mean = if achromatic {
        h1 + h2
    } else if (h1 - h2).abs() <= 180.0 {
        (h1 + h2) / 2.0
    } else if h1 + h2 < 360.0 {
        (h1 + h2 + 360.0) / 2.0
    } else {
        (h1 + h2 - 360.0) / 2.0
    };
    let cos = |deg: f64| deg.to_radians().cos();
    let t =
        1.0 - 0.17 * cos(h_mean - 30.0) + 0.24 * cos(2.0 * h_mean) + 0.32 * cos(3.0 * h_mean + 6.0)
            - 0.20 * cos(4.0 * h_mean - 63.0);
    let d_theta = 30.0 * (-((h_mean - 275.0) / 25.0).powi(2)).exp();
    let r_c = 2.0 * (c_mean.powi(7) / (c_mean.powi(7) + POW25_7)).sqrt();
    let l_off = (l_mean - 50.0).powi(2);
    let s_l = 1.0 + 0.015 * l_off / (20.0 + l_off).sqrt();
    let s_c = 1.0 + 0.045 * c_mean;
    let s_h = 1.0 + 0.015 * c_mean * t;
    let r_t = -(2.0 * d_theta).to_radians().sin() * r_c;
    let (l, c, h) = (d_l / s_l, d_c / s_c, d_big_h / s_h);
    (l * l + c * c + h * h + r_t * c * h).sqrt()
}

/// The CIEDE2000 difference of two sRGB colours
pub(super) fn delta_e(a: Colour, b: Colour) -> f64 {
    ciede2000(lab(a), lab(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(r: u8, g: u8, b: u8) -> Colour {
        Colour { r, g, b, a: 1.0 }
    }

    /// Pairs of Sharma's test data, with what they must give
    #[test]
    fn sharmas_table() {
        let pairs = [
            ([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
            ([50.0, 2.5, 0.0], [50.0, 0.0, -2.5], 4.3065),
            ([50.0, 2.5, 0.0], [73.0, 25.0, -18.0], 27.1492),
            ([50.0, 2.5, 0.0], [50.0, 3.1736, 0.5854], 1.0),
            (
                [60.2574, -34.0099, 36.2677],
                [60.4626, -34.1751, 39.4387],
                1.2644,
            ),
            (
                [22.7233, 20.0904, -46.694],
                [23.0331, 14.973, -42.5619],
                2.0373,
            ),
            (
                [90.8027, -2.0831, 1.441],
                [91.1528, -1.6435, 0.0447],
                1.4441,
            ),
            ([2.0776, 0.0795, -1.135], [0.9033, -0.0636, -0.5514], 0.9082),
        ];
        for (a, b, want) in pairs {
            let got = ciede2000(a, b);
            assert!((got - want).abs() < 1e-4, "{a:?} {b:?}: {got}, not {want}");
            assert!((ciede2000(b, a) - want).abs() < 1e-4);
        }
    }

    #[test]
    fn the_guides_numbers() {
        let wire = rgb(74, 74, 74);
        let pairs = [
            (wire, rgb(150, 108, 0), 29.0),
            (rgb(168, 168, 168), rgb(232, 196, 82), 26.9),
            (rgb(0, 115, 30), rgb(60, 170, 24), 20.5),
            (rgb(175, 50, 10), rgb(232, 113, 10), 22.9),
            (rgb(150, 108, 0), rgb(200, 20, 28), 32.9),
            (rgb(232, 196, 82), rgb(242, 76, 68), 43.7),
        ];
        for (a, b, want) in pairs {
            let got = delta_e(a, b);
            assert!((got - want).abs() < 0.06, "{a} {b}: {got}, not {want}");
        }
    }
}
