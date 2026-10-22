//! `DEC` - decoupling and datasheet passives, on the rails of [`erc_analysis::power`] and the supply pins and
//! capacitors of [`erc_analysis::supply`]

mod dec_1;
mod dec_10;
mod dec_11;
mod dec_12;
mod dec_4;
mod dec_7;
mod dec_8;

use crate::Ctx;
use erc_analysis::supply::caps_to_ground;
use erc_model::{NetId, PartId, PinId, natural_cmp};

/// `R1, R2 and C3`: designators in natural order
pub(crate) fn designators(cx: &Ctx, parts: impl IntoIterator<Item = PartId>) -> String {
    let b = cx.board();
    let mut names: Vec<&str> = parts
        .into_iter()
        .map(|p| b.part(p).designator.as_str())
        .collect();
    names.sort_by(|a, b| natural_cmp(a, b));
    names.dedup();
    and_list(&names)
}

/// `U1.3 (VDD), U1.7 (VDDA)`: pins with their names
pub(crate) fn pin_list(cx: &Ctx, pins: &[PinId]) -> String {
    let b = cx.board();
    let mut names: Vec<String> = pins
        .iter()
        .map(|p| {
            let name = &b.pin(*p).name;
            if name.is_empty() || name == "~" {
                b.pin_name(*p)
            } else {
                format!("{} ({name})", b.pin_name(*p))
            }
        })
        .collect();
    names.sort_by(|a, b| natural_cmp(a, b));
    and_list(&names)
}

/// `a`, `a and b`, `a, b and c`
pub(crate) fn and_list<S: AsRef<str>>(items: &[S]) -> String {
    match items {
        [] => String::new(),
        [one] => one.as_ref().to_string(),
        [rest @ .., last] => format!(
            "{} and {}",
            rest.iter()
                .map(|s| s.as_ref())
                .collect::<Vec<_>>()
                .join(", "),
            last.as_ref()
        ),
    }
}

/// The fitted capacitors from a net to a ground in the variant checked
fn caps(cx: &Ctx, net: NetId) -> Vec<PartId> {
    caps_to_ground(cx.view(), cx.classes(), cx.power(), net)
}
