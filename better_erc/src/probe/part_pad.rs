//! Which pads of an ordinary part (neither a test point nor a connector) a hand probe can touch: the ends of a
//! chip passive that is not too small, and the leads of any other part whose pitch is wide enough. A pad under the
//! part's body never is one.

use super::{NotCounted, ProbeOptions};
use erc_analysis::package::{CaseSize, Package};
use erc_model::PartClass;
use erc_model::layout::{FootprintId, Layout, Pad, PadId, Pt};
use std::collections::BTreeSet;

/// A two-terminal passive whose footprint name has no case code counts when its pad centres are at least this
/// share of the minimum size's body length apart: KiCad's 0402 pads are 0.96 mm apart, its 0201 pads 0.64 mm
const CENTRES_PER_LENGTH_MIN: f64 = 0.8;

/// A pad is inside the other pads' box only by more than this, nm: rounding in a turned footprint stays below it
const INSIDE_NM_MIN: i64 = 10_000;

/// Endings of the package words whose terminals are all under the body (`VQFN`, `UFQFPN`, `TFBGA`, `WLCSP`)
const UNDER_BODY_ENDINGS: [&str; 6] = ["QFN", "QFPN", "DFN", "BGA", "LGA", "CSP"];

/// The small outline no-lead package words, whole: an ending of `SON` would also take a vendor's name
const NO_LEAD_WORDS: [&str; 9] = [
    "SON", "WSON", "VSON", "USON", "HSON", "HVSON", "PWSON", "X1SON", "X2SON",
];

/// What a probe can do with one pad
#[derive(Debug, PartialEq)]
pub(super) struct Verdict {
    /// A passive's case code from its footprint name
    pub(super) size: Option<String>,
    /// The pad pitch that decided, mm
    pub(super) pitch_mm: Option<f64>,
    /// Why the pad does not count; none when it counts
    pub(super) not_counted: Option<NotCounted>,
}

/// What a footprint says about its pads
pub(super) struct PartFootprint {
    /// The case code in the footprint name
    size: Option<CaseSize>,
    /// Whether it has exactly two pad numbers
    two_terminals: bool,
    /// The pads under the body
    under_body: BTreeSet<PadId>,
    /// The nearest two pads of different numbers that are not under the body, mm
    pitch_mm: Option<f64>,
}

impl PartFootprint {
    pub(super) fn of(layout: &Layout, id: FootprintId) -> PartFootprint {
        let local = layout.local_footprint(id);
        let numbered: Vec<(usize, &Pad)> = local
            .pads
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.number.is_empty() && p.kind != "np_thru_hole")
            .collect();
        let under_body = if is_under_body_name(&local.footprint.lib_id) {
            vec![true; local.pads.len()]
        } else {
            enclosed(local.pads.len(), &numbered)
        };
        let leads: Vec<&Pad> = numbered
            .iter()
            .filter(|(i, _)| !under_body[*i])
            .map(|(_, p)| *p)
            .collect();
        let mut numbers: Vec<&str> = numbered.iter().map(|(_, p)| p.number.as_str()).collect();
        numbers.sort_unstable();
        numbers.dedup();
        let under_body = layout
            .footprint(id)
            .pads
            .iter()
            .zip(under_body)
            .filter_map(|(pad, under)| under.then_some(*pad))
            .collect();
        PartFootprint {
            size: Package::from_footprint(&local.footprint.lib_id)
                .and_then(|p| p.size)
                .and_then(CaseSize::parse),
            two_terminals: numbers.len() == 2,
            under_body,
            pitch_mm: pitch_mm(&leads),
        }
    }

    /// The verdict on one of the footprint's pads, of a part of `class`
    pub(super) fn judge(&self, pad: PadId, class: PartClass, opts: &ProbeOptions) -> Verdict {
        let verdict = |size: Option<CaseSize>, pitch_mm, not_counted| Verdict {
            size: size.map(|s| s.code().to_owned()),
            pitch_mm,
            not_counted,
        };
        if self.under_body.contains(&pad) {
            return verdict(None, None, Some(NotCounted::UnderBody));
        }
        let passive = self.two_terminals && is_passive(class);
        match self.size.filter(|_| passive) {
            Some(size) => {
                let small = (size < opts.passive_size_min).then(|| NotCounted::Size {
                    size: size.code().to_owned(),
                    min: opts.passive_size_min.code().to_owned(),
                });
                verdict(Some(size), None, small)
            }
            None => {
                let min_mm = if passive {
                    round_um(opts.passive_size_min.length_mm() * CENTRES_PER_LENGTH_MIN)
                } else {
                    opts.part_pitch_mm_min
                };
                let fine = self
                    .pitch_mm
                    .filter(|p| *p < min_mm)
                    .map(|pitch_mm| NotCounted::Pitch { pitch_mm, min_mm });
                verdict(None, self.pitch_mm, fine)
            }
        }
    }
}

/// A resistor, capacitor, inductor, ferrite, diode, LED or fuse
fn is_passive(class: PartClass) -> bool {
    matches!(
        class,
        PartClass::Resistor
            | PartClass::Capacitor { .. }
            | PartClass::Inductor
            | PartClass::FerriteBead
            | PartClass::Diode { .. }
            | PartClass::Fuse { .. }
    )
}

/// Whether the footprint's library or name says the package has its terminals under the body
fn is_under_body_name(lib_id: &str) -> bool {
    lib_id.split([':', '_', '-', ' ']).any(|word| {
        // `DFN1006` and `QFN32` are the words `DFN` and `QFN`
        let word = word.trim_end_matches(|c: char| c.is_ascii_digit());
        UNDER_BODY_ENDINGS.iter().any(|e| word.ends_with(e)) || NO_LEAD_WORDS.contains(&word)
    })
}

/// The pads under the body of a package its name does not tell: a surface-mount pad enclosed by the pads of other
/// numbers (an exposed thermal pad, an inner ball), with the holes of the same number enclosed too (its thermal
/// vias)
fn enclosed(len: usize, numbered: &[(usize, &Pad)]) -> Vec<bool> {
    let inside = |pad: &Pad| {
        let others = numbered
            .iter()
            .filter(|(_, o)| o.number != pad.number)
            .map(|(_, o)| o.at);
        strictly_inside(pad.at, others)
    };
    let smd: Vec<&str> = numbered
        .iter()
        .filter(|(_, p)| p.kind != "thru_hole" && inside(p))
        .map(|(_, p)| p.number.as_str())
        .collect();
    let mut out = vec![false; len];
    for (i, pad) in numbered {
        out[*i] = smd.contains(&pad.number.as_str()) && inside(pad);
    }
    out
}

/// Whether `p` is inside the box around `points`, away from each of its edges
fn strictly_inside(p: Pt, points: impl Iterator<Item = Pt>) -> bool {
    let bounds = points.fold(None, |b: Option<(Pt, Pt)>, q| {
        let (lo, hi) = b.unwrap_or((q, q));
        Some((
            Pt::new(lo.x.min(q.x), lo.y.min(q.y)),
            Pt::new(hi.x.max(q.x), hi.y.max(q.y)),
        ))
    });
    bounds.is_some_and(|(lo, hi)| {
        p.x - lo.x > INSIDE_NM_MIN
            && hi.x - p.x > INSIDE_NM_MIN
            && p.y - lo.y > INSIDE_NM_MIN
            && hi.y - p.y > INSIDE_NM_MIN
    })
}

/// The nearest two of the pads with different numbers, centre to centre, mm, rounded to the µm; none with fewer
/// than two numbers
fn pitch_mm(pads: &[&Pad]) -> Option<f64> {
    let mut nearest: Option<f64> = None;
    for (i, a) in pads.iter().enumerate() {
        for b in &pads[i + 1..] {
            if a.number == b.number {
                continue;
            }
            let d = ((a.at.x - b.at.x) as f64).hypot((a.at.y - b.at.y) as f64) / 1e6;
            if nearest.is_none_or(|n| d < n) {
                nearest = Some(d);
            }
        }
    }
    nearest.map(round_um)
}

fn round_um(mm: f64) -> f64 {
    (mm * 1e3).round() / 1e3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(number: &str, kind: &str, x_um: i64, y_um: i64) -> Pad {
        Pad {
            number: number.into(),
            kind: kind.into(),
            at: Pt::new(x_um * 1000, y_um * 1000),
            ..super::super::tests::pad(&["F.Cu", "F.Mask"])
        }
    }

    fn under(pads: &[Pad]) -> Vec<bool> {
        let numbered: Vec<(usize, &Pad)> = pads.iter().enumerate().collect();
        enclosed(pads.len(), &numbered)
    }

    #[test]
    fn an_exposed_pad_and_its_vias_are_enclosed_the_leads_are_not() {
        let mut pads = vec![
            pad("1", "smd", -2475, -1905),
            pad("2", "smd", -2475, 1905),
            pad("3", "smd", 2475, 1905),
            pad("4", "smd", 2475, -1905),
            pad("5", "smd", 0, 0),
            pad("5", "thru_hole", 500, 500),
        ];
        assert_eq!(under(&pads), [false, false, false, false, true, true]);
        // Pins of a through-hole part that happen to lie between others are reached from the other side
        pads[4].kind = "thru_hole".into();
        assert_eq!(under(&pads), [false; 6]);
    }

    #[test]
    fn a_tab_beside_its_leads_is_not_enclosed() {
        let pads = [
            pad("1", "smd", -3150, -2300),
            pad("2", "smd", -3150, 0),
            pad("3", "smd", -3150, 2300),
            pad("2", "smd", 3150, 0),
        ];
        assert_eq!(under(&pads), [false; 4]);
    }

    #[test]
    fn packages_with_their_terminals_under_the_body() {
        for name in [
            "Package_DFN_QFN:QFN-32-1EP_5x5mm_P0.5mm",
            "Lib:TI_RUK0020B_WQFN-20-1EP_3x3mm_P0.4mm",
            "Lib:ST_UFQFPN-20_3x3mm",
            "Package_BGA:Xilinx_FTG256",
            "Lib:TFBGA-100",
            "Package_CSP:WLCSP-8",
            "Package_LGA:Bosch_LGA-14",
            "Package_SON:Texas_X2SON-4",
            "Lib:D_DFN1006",
        ] {
            assert!(is_under_body_name(name), "{name}");
        }
        for name in [
            "Package_SO:TSSOP-20_4.4x6.5mm_P0.65mm",
            "Package_QFP:LQFP-48_7x7mm_P0.5mm",
            "Package_TO_SOT_SMD:SOT-23-6",
            "Antenna:Johanson_2450AT18",
            "Crystal:Crystal_SMD_SeikoEpson_FA238",
            "Resistor_SMD:R_0402_1005Metric",
        ] {
            assert!(!is_under_body_name(name), "{name}");
        }
    }
}
