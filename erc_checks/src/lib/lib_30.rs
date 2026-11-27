//! `LIB-30` Placed symbol whose `mx3:` Datasheet link does not reference exactly its MPN fields

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_parts::datasheet::stale;

pub struct Lib30;

static META: CheckMeta = CheckMeta {
    id: "LIB-30",
    category: "LIB",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

impl Check for Lib30 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A part whose `Datasheet` is an `mx3:` link referencing an MPN field the symbol lacks, or missing one it has
    /// (blank fields count as had); a vendor URL or an empty `Datasheet` is never looked at.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        cx.components()
            .filter_map(|p| {
                let part = b.part(p);
                let link = part.field("Datasheet")?;
                let names = part
                    .fields
                    .iter()
                    .map(|f| f.name.as_str())
                    .chain(part.blank_fields.iter().map(String::as_str));
                let s = stale(link, names)?;
                let refs = |l: &[String]| {
                    l.iter()
                        .map(|n| format!("${{{n}}}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let mut why = Vec::new();
                if !s.unknown.is_empty() {
                    why.push(format!(
                        "references {} the symbol has no field for (KiCad shows them unresolved)",
                        refs(&s.unknown)
                    ));
                }
                if !s.missing.is_empty() {
                    why.push(format!(
                        "misses {} the symbol has, so their part numbers are never looked up",
                        refs(&s.missing)
                    ));
                }
                Some(
                    cx.finding(&META)
                        .part(p)
                        .message(format!(
                            "{}'s Datasheet `{link}` {}",
                            part.designator,
                            why.join(", and ")
                        ))
                        .fix(
                            "close the schematic in KiCad and run `berc fix datasheet-links --write`: each link \
                             becomes `mx3:` and a reference to each MPN field, and the lib_symbols copy takes the \
                             library's Datasheet",
                        )
                        .build(),
                )
            })
            .collect()
    }
}

inventory::submit!(Registered(&Lib30));

#[cfg(test)]
mod tests {
    // U1 right. U2 references Mpn1..Mpn12 with Mpn1..Mpn4 and a blank Mpn12. U3 misses Mpn2, added later. U4 a
    // vendor URL, U5 an empty Datasheet, U6 a literal part number and no MPN field: never findings.
    const SCHEMATIC: &str = "lib-30_datasheet_link";

    #[test]
    fn stale_links() {
        assert_eq!(
            crate::testing::fingerprints(&super::Lib30, SCHEMATIC),
            ["LIB-30:part U2", "LIB-30:part U3"]
        );
    }
}
