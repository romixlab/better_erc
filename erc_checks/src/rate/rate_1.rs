//! `RATE-1` Capacitor voltage rating margins

use super::{Stress, judge_minimum, judge_rating};
use crate::component::{self, Candidate};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::PartClass;
use erc_model::Unit;

pub struct Rate1;

static META: CheckMeta = CheckMeta {
    id: "RATE-1",
    category: "RATE",
    severity: Severity::Warning,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::PartsLock],
    overlaps: &[],
    params: &[],
};

/// How to fix a miss
const FIX: &str =
    "an MPN of a higher voltage rating, or a lower Vworking where the field overstates it";

impl Check for Rate1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each MPN of a fitted capacitor whose working voltage is known (its `Vworking` field, else its two nets):
    /// rated voltage at least the derating factor of its technology times the working voltage; a `Vreq_min` field
    /// states the rating needed in its place
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let mut out = Vec::new();
        for p in component::fitted_sorted(cx) {
            if !matches!(cx.classes().get(p), PartClass::Capacitor { .. }) {
                continue;
            }
            let stress = component::working_voltage(cx, p).map(Stress::from);
            if let Some(f) = judge_minimum(
                cx,
                &META,
                p,
                "Vreq_min",
                Unit::Volt,
                "rated_voltage",
                stress.as_ref(),
                FIX,
            ) {
                out.extend(f);
                continue;
            }
            let Some(s) = stress else {
                continue;
            };
            out.extend(judge_rating(
                cx,
                &META,
                p,
                &s,
                "rated_voltage",
                derating_key,
                FIX,
            ));
        }
        out
    }
}

/// The derating key of a capacitor MPN's technology; a technology not stated is ceramic when the MPN has a
/// dielectric code
fn derating_key(c: &Candidate) -> Result<&'static str, &'static str> {
    let Some(t) = c.text("technology") else {
        return if c.text("dielectric").is_some() {
            Ok("ceramic_capacitor_v")
        } else {
            Err("technology")
        };
    };
    let t = t.to_ascii_lowercase();
    let has = |w: &str| t.contains(w);
    Ok(if has("ceramic") || has("mlcc") {
        "ceramic_capacitor_v"
    } else if has("tantalum") && has("polymer") {
        "tantalum_polymer_capacitor_v"
    } else if has("tantalum") {
        "tantalum_capacitor_v"
    } else if has("polymer") {
        "polymer_capacitor_v"
    } else if has("electrolytic") || has("aluminium") || has("aluminum") {
        "electrolytic_capacitor_v"
    } else if has("film") {
        "film_capacitor_v"
    } else {
        return Err("technology");
    })
}

inventory::submit!(Registered(&Rate1));

#[cfg(test)]
mod tests {
    use super::Rate1;
    use crate::testing::{report_with, with_lock};

    const SCH: &str = "rate-1_capacitor_voltage";

    #[test]
    fn the_project_factor_decides() {
        let mut opts = with_lock(SCH);
        let r = report_with(&Rate1, SCH, &opts);
        let c3 = r
            .findings
            .iter()
            .find(|f| f.fingerprint == "RATE-1:item C3 Mpn1|part C3")
            .expect("C3 misses at the default 1.5");
        let hash = c3
            .input_hash
            .clone()
            .expect("a miss carries its input hash");
        // 25 V at 20 V working is 1.25: a factor of 1.25 passes it, 1.6 fails it again with another hash
        opts.config.derating.ceramic_capacitor_v = 1.25;
        let r = report_with(&Rate1, SCH, &opts);
        assert!(r.findings.iter().all(|f| !f.fingerprint.contains("C3")));
        opts.config.derating.ceramic_capacitor_v = 1.6;
        let r = report_with(&Rate1, SCH, &opts);
        let again = r
            .findings
            .iter()
            .find(|f| f.fingerprint == "RATE-1:item C3 Mpn1|part C3")
            .expect("C3 misses at 1.6");
        assert_ne!(
            again.input_hash.as_ref(),
            Some(&hash),
            "the factor is an input"
        );
    }

    #[test]
    fn same_inputs_same_hash() {
        let opts = with_lock(SCH);
        let a = report_with(&Rate1, SCH, &opts);
        let b = report_with(&Rate1, SCH, &opts);
        let hashes = |r: &crate::Report| -> Vec<Option<String>> {
            r.findings.iter().map(|f| f.input_hash.clone()).collect()
        };
        assert_eq!(hashes(&a), hashes(&b));
        assert!(hashes(&a).iter().filter(|h| h.is_some()).count() >= 2);
    }
}
