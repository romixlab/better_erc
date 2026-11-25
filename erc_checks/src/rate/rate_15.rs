//! `RATE-15` Inductor saturation current below the converter's peak current

use super::{field_stress, judge_rating};
use crate::component;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::PartClass;
use erc_model::Unit;

pub struct Rate15;

static META: CheckMeta = CheckMeta {
    id: "RATE-15",
    category: "RATE",
    severity: Severity::Error,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::PartsLock],
    overlaps: &[],
    params: &[],
};

impl Check for Rate15 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each MPN of a fitted inductor with an `Ipeak` field: saturation current at least `derating.inductor_isat`
    /// times the peak current
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let mut out = Vec::new();
        for p in component::fitted_sorted(cx) {
            if cx.classes().get(p) != PartClass::Inductor {
                continue;
            }
            let Some(s) = field_stress(cx, p, "Ipeak", Unit::Ampere, "peak") else {
                continue;
            };
            out.extend(judge_rating(
                cx,
                &META,
                p,
                &s,
                "saturation_current",
                |_| Ok("inductor_isat"),
                "an inductor whose saturation current is above the peak with margin, or more inductance for less ripple",
            ));
        }
        out
    }
}

inventory::submit!(Registered(&Rate15));
