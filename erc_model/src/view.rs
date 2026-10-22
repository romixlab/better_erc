//! A board as one variant mounts it: [`View`] applies a variant's DNP flags and field overrides (KiCad 10
//! variants, OrCAD per-variant BOMs), so a check sees what is fitted and the values that are placed. The base
//! design is the view without a variant.

use crate::design::{Board, Field, PartId, PartVariant, VariantId, field_value};
use crate::fact::Fact;

/// A board seen through one variant (`None`: the base design)
#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    pub board: &'a Board,
    pub variant: Option<VariantId>,
}

impl<'a> View<'a> {
    pub fn new(board: &'a Board, variant: Option<VariantId>) -> View<'a> {
        View { board, variant }
    }

    /// The base design
    pub fn base(board: &'a Board) -> View<'a> {
        View {
            board,
            variant: None,
        }
    }

    /// The variant's name, `None` for the base design
    pub fn variant_name(&self) -> Option<&'a str> {
        self.variant
            .map(|v| self.board.variants[v.index()].name.as_str())
    }

    fn part_variant(&self, part: PartId) -> Option<&'a PartVariant> {
        let v = self.variant?;
        self.board
            .part(part)
            .variants
            .iter()
            .find(|pv| pv.variant == v)
    }

    /// Whether the part is fitted: the variant's DNP flag when it sets one, else the part's own
    pub fn fitted(&self, part: PartId) -> bool {
        self.board.fitted(part, self.variant)
    }

    /// A flag by its KiCad name (`dnp`, `in_bom`, `on_board`, `in_pos_files`, `exclude_from_sim`): the variant's
    /// when it sets it, else the part's; `None` when no source states it (the KiCad default applies)
    pub fn flag(&self, part: PartId, name: &str) -> Option<bool> {
        self.part_variant(part)
            .and_then(|pv| pv.flags.by_name(name))
            .and_then(|f| f.get())
            .or_else(|| self.board.part(part).flags.by_name(name)?.get())
            .copied()
    }

    /// Whether the part goes into the BOM (KiCad's default: yes) and is fitted
    pub fn in_bom(&self, part: PartId) -> bool {
        self.fitted(part) && self.flag(part, "in_bom").unwrap_or(true)
    }

    /// The resolved value of a field: the variant's override when it has one, else the part's own
    pub fn field(&self, part: PartId, name: &str) -> Option<&'a str> {
        self.board.field(part, name, self.variant)
    }

    /// A field's resolved fact with its source: the variant's override first
    pub fn field_fact(&self, part: PartId, name: &str) -> Option<&'a Fact<String>> {
        let find = |fields: &'a [Field]| {
            fields
                .iter()
                .find(|f| f.name == name)
                .and_then(|f| f.value.resolve())
        };
        self.part_variant(part)
            .and_then(|pv| find(&pv.fields))
            .or_else(|| find(&self.board.part(part).fields))
    }

    /// Whether the variant overrides anything on the part (a flag or a field)
    pub fn overridden(&self, part: PartId) -> bool {
        self.part_variant(part).is_some_and(|pv| {
            !pv.fields.is_empty()
                || crate::design::PartFlags::NAMES
                    .iter()
                    .any(|n| pv.flags.by_name(n).is_some_and(|f| !f.is_empty()))
        })
    }

    /// The real components (no `#` virtual parts), fitted or not
    pub fn components(&self) -> impl Iterator<Item = PartId> + 'a {
        let b = self.board;
        (0..b.parts.len())
            .map(PartId::from_index)
            .filter(move |p| b.is_component(*p))
    }

    /// The fitted real components
    pub fn fitted_components(&self) -> impl Iterator<Item = PartId> + 'a {
        let v = *self;
        self.components().filter(move |p| v.fitted(*p))
    }

    /// The resolved Value field (`field_value` of the variant's fields first)
    pub fn value_text(&self, part: PartId) -> Option<&'a str> {
        self.part_variant(part)
            .and_then(|pv| field_value(&pv.fields, "Value"))
            .or_else(|| self.board.part(part).field("Value"))
    }
}

impl Board {
    /// The board as `variant` mounts it
    pub fn view(&self, variant: Option<VariantId>) -> View<'_> {
        View::new(self, variant)
    }
}

#[cfg(test)]
mod tests {
    use crate::design::*;
    use crate::fact::{Fact, Facts, Provenance};

    fn sch() -> Provenance {
        Provenance::Schematic {
            file: "a.kicad_sch".into(),
        }
    }

    fn part(designator: &str, value: &str) -> Part {
        Part {
            designator: designator.into(),
            symbol: None,
            lib_id: "Device:R".into(),
            sheet: SheetId(0),
            units: Vec::new(),
            fields: vec![Field::new("Value", Fact::new(value.into(), sch()))],
            value: Facts::default(),
            flags: PartFlags::default(),
            variants: Vec::new(),
            pins: Vec::new(),
            ties: Vec::new(),
        }
    }

    #[test]
    fn variant_applied() {
        let mut r1 = part("R1", "10k");
        r1.variants.push(PartVariant {
            variant: VariantId(0),
            flags: PartFlags {
                dnp: Facts::one(Fact::new(true, sch())),
                ..PartFlags::default()
            },
            fields: vec![Field::new("Value", Fact::new("4k7".into(), sch()))],
        });
        let mut r2 = part("R2", "1k");
        r2.flags.in_bom = Facts::one(Fact::new(false, sch()));
        let board = Board {
            parts: vec![r1, r2, part("#PWR01", "GND")],
            variants: vec![Variant {
                name: "Lite".into(),
                description: None,
            }],
            ..Board::default()
        };
        let base = board.view(None);
        let lite = board.view(Some(VariantId(0)));
        let (r1, r2) = (PartId(0), PartId(1));
        assert_eq!(lite.variant_name(), Some("Lite"));
        assert!(base.fitted(r1) && !lite.fitted(r1));
        assert_eq!(lite.flag(r1, "dnp"), Some(true));
        assert_eq!(base.flag(r1, "dnp"), None);
        assert_eq!(base.value_text(r1), Some("10k"));
        assert_eq!(lite.value_text(r1), Some("4k7"));
        assert_eq!(
            lite.field_fact(r1, "Value").map(|f| f.value.as_str()),
            Some("4k7")
        );
        assert!(lite.overridden(r1) && !lite.overridden(r2));
        assert!(!base.in_bom(r2), "in_bom no");
        assert_eq!(base.components().count(), 2);
        assert_eq!(lite.fitted_components().collect::<Vec<_>>(), [r2]);
    }
}
