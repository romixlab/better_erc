//! The board graph: arenas of sheets, symbols, parts, pin instances, nets and labels with small integer ids, and
//! adjacency both ways (a net lists its pins, a part lists its pins, a pin names its net), so every query is
//! O(degree). Display names (designator, net name, pin number, sheet path) live in one place and are resolved
//! through the board.

use crate::fact::{Fact, Facts, Provenance};
use crate::layout::Layout;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! id {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(pub u32);

        impl $name {
            /// Index into the board's arena
            pub fn index(self) -> usize {
                self.0 as usize
            }

            /// Id of an arena index
            pub fn from_index(i: usize) -> Self {
                Self(i as u32)
            }
        }
    };
}

id!(
    /// A board of a design
    BoardId
);
id!(
    /// A part (one designator, all its units)
    PartId
);
id!(
    /// A pin instance on a part
    PinId
);
id!(
    /// A net
    NetId
);
id!(
    /// A sheet instance (a sheet file used twice is two sheets)
    SheetId
);
id!(
    /// A library symbol as placed in the schematic
    SymbolId
);
id!(
    /// A label, power symbol or sheet pin
    LabelId
);
id!(
    /// A variant of the board
    VariantId
);
id!(
    /// A net class
    NetClassId
);

/// What a finding or a query points at
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Subject {
    Board(BoardId),
    Part(BoardId, PartId),
    Pin(BoardId, PinId),
    Net(BoardId, NetId),
    Sheet(BoardId, SheetId),
    Label(BoardId, LabelId),
    Pad(BoardId, crate::layout::PadId),
}

/// A design: one board, or a system of boards joined through connectors
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Design {
    pub boards: Vec<Board>,
    pub links: Vec<BoardLink>,
}

impl Design {
    /// A design of one board
    pub fn single(board: Board) -> Design {
        Design {
            boards: vec![board],
            links: Vec::new(),
        }
    }

    pub fn board(&self, id: BoardId) -> &Board {
        &self.boards[id.index()]
    }
}

/// Two boards mated through a connector on each (from the project file)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoardLink {
    pub a: (BoardId, PartId),
    pub b: (BoardId, PartId),
    pub map: LinkMap,
    pub source: Provenance,
}

/// How the pins of two mated connectors pair up
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LinkMap {
    /// Pin n to pin n
    ByNumber,
    /// Explicit pairs of pin numbers (a mate table, a cable)
    Table(Vec<(String, String)>),
}

/// Which tool and files a board was read from
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SourceInfo {
    /// `kicad`, `kicad-netlist`, `altium`, `orcad`
    pub tool: String,
    /// The tool's version as the files state it (`10.0`, file format `20250114`)
    pub version: Option<String>,
    /// The files read, relative to the project directory, in reading order
    pub files: Vec<String>,
}

/// One board: a KiCad project, or one netlist
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Board {
    pub name: String,
    pub source: SourceInfo,
    /// Sheet instances; the root sheet first
    pub sheets: Vec<Sheet>,
    pub symbols: Vec<SymbolDef>,
    pub parts: Vec<Part>,
    pub pins: Vec<PinInst>,
    pub nets: Vec<Net>,
    pub labels: Vec<Label>,
    pub no_connects: Vec<NoConnect>,
    pub variants: Vec<Variant>,
    pub net_classes: Vec<NetClass>,
    pub settings: ProjectSettings,
    pub layout: Option<Layout>,
}

/// A sheet instance in the hierarchy
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sheet {
    /// The sheet's name in its parent (`MCU`); empty for the root
    pub name: String,
    /// The sheet file, relative to the project directory
    pub file: String,
    pub parent: Option<SheetId>,
    /// Instance path of UUIDs (`/f456.../9c76...`); the root is `/` followed by its UUID, or empty when unknown
    pub uuid_path: String,
    /// The page number shown in KiCad
    pub page: Option<String>,
}

/// A symbol as stored in the schematic's library copy (`lib_symbols`)
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SymbolDef {
    /// The name the placed symbols refer to (`Device:R`, or the `lib_name` of a modified copy)
    pub name: String,
    pub description: String,
    pub keywords: String,
    pub footprint_filters: Vec<String>,
    /// Power symbol (its pin names a net instead of being a part)
    pub power: Option<PowerScope>,
    /// Number of units (1 for a single-unit symbol)
    pub units: u32,
    /// Names of units that have one (OrCAD sections `A`, `B`; KiCad `unit_name`), by unit number
    pub unit_names: Vec<(u32, String)>,
    pub pins: Vec<PinDef>,
    /// Other library fields (`Value`, `Footprint`, user fields), in file order
    pub fields: Vec<(String, String)>,
    /// Pins with the same number are connected inside the part (KiCad 10 `duplicate_pin_numbers_are_jumpers`)
    pub duplicate_pins_are_jumpers: bool,
    /// Groups of pin numbers connected inside the part (KiCad 10 `jumper_pin_groups`)
    pub jumper_pin_groups: Vec<Vec<String>>,
    /// Hash of the library copy, to compare with the library (LIB-17)
    pub hash: Option<String>,
}

/// Where a power symbol's net is visible
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PowerScope {
    /// Everywhere (all KiCad versions)
    Global,
    /// In its sheet only (KiCad 10 local power symbols)
    Local,
}

/// A pin of a library symbol
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PinDef {
    pub number: String,
    pub name: String,
    /// The unit it belongs to; 0 for pins common to all units
    pub unit: u32,
    /// The body style (De Morgan); 0 for both
    pub body_style: u32,
    pub etype: EcadPinType,
    /// Graphic style (`line`, `inverted`, `clock`, ...)
    pub style: String,
    pub hidden: bool,
    pub alternates: Vec<PinAlternate>,
}

/// An alternate pin definition (KiCad alternate pin functions)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PinAlternate {
    pub name: String,
    pub etype: EcadPinType,
    pub style: String,
}

/// The electrical type a schematic tool gives a pin, mapped onto KiCad's set
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EcadPinType {
    Input,
    Output,
    Bidirectional,
    TriState,
    Passive,
    Free,
    Unspecified,
    PowerIn,
    PowerOut,
    OpenCollector,
    OpenEmitter,
    NoConnect,
}

impl EcadPinType {
    /// KiCad's name (`power_in`)
    pub fn name(self) -> &'static str {
        match self {
            EcadPinType::Input => "input",
            EcadPinType::Output => "output",
            EcadPinType::Bidirectional => "bidirectional",
            EcadPinType::TriState => "tri_state",
            EcadPinType::Passive => "passive",
            EcadPinType::Free => "free",
            EcadPinType::Unspecified => "unspecified",
            EcadPinType::PowerIn => "power_in",
            EcadPinType::PowerOut => "power_out",
            EcadPinType::OpenCollector => "open_collector",
            EcadPinType::OpenEmitter => "open_emitter",
            EcadPinType::NoConnect => "no_connect",
        }
    }

    /// From KiCad's name, in the schematic (`power_in`) or in the netlist (`power_in`, `free`, `no_connect`)
    pub fn from_name(name: &str) -> Option<EcadPinType> {
        Some(match name {
            "input" => EcadPinType::Input,
            "output" => EcadPinType::Output,
            "bidirectional" => EcadPinType::Bidirectional,
            "tri_state" => EcadPinType::TriState,
            "passive" => EcadPinType::Passive,
            "free" | "unconnected" => EcadPinType::Free,
            "unspecified" => EcadPinType::Unspecified,
            "power_in" => EcadPinType::PowerIn,
            "power_out" => EcadPinType::PowerOut,
            "open_collector" => EcadPinType::OpenCollector,
            "open_emitter" => EcadPinType::OpenEmitter,
            "no_connect" => EcadPinType::NoConnect,
            _ => return None,
        })
    }
}

impl fmt::Display for EcadPinType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A named attribute with every source's statement of it
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub value: Facts<String>,
}

impl Field {
    pub fn new(name: impl Into<String>, fact: Fact<String>) -> Field {
        Field {
            name: name.into(),
            value: Facts::one(fact),
        }
    }
}

/// The resolved value of the field `name` in a list of fields
pub fn field_value<'a>(fields: &'a [Field], name: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|f| f.name == name)
        .and_then(|f| f.value.get())
        .map(String::as_str)
}

/// A part: one designator with all its units
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub designator: String,
    pub symbol: Option<SymbolId>,
    /// The library id as the schematic or netlist names it (`Device:R`)
    pub lib_id: String,
    /// The sheet of the first placed unit
    pub sheet: SheetId,
    pub units: Vec<UnitPlacement>,
    /// Fields in file order: `Value`, `Footprint`, `Datasheet`, `Description` and user fields (`Mpn1`)
    pub fields: Vec<Field>,
    /// The Value field, parsed
    pub value: Facts<Value>,
    pub flags: PartFlags,
    /// Per-variant overrides (KiCad 10 variants)
    pub variants: Vec<PartVariant>,
    pub pins: Vec<PinId>,
    /// Groups of pin numbers connected inside the part: net ties (footprint `net_tie_pad_groups`) and jumpers
    pub ties: Vec<Vec<String>>,
}

impl Part {
    /// The resolved value of a field
    pub fn field(&self, name: &str) -> Option<&str> {
        field_value(&self.fields, name)
    }
}

/// Where one unit of a part is placed
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnitPlacement {
    pub unit: u32,
    pub sheet: SheetId,
    /// The symbol's UUID in its sheet file
    pub uuid: String,
    pub at: Option<Placement>,
}

/// Position, rotation and mirroring of a placed symbol (schematic coordinates, nm, y down)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub at: crate::layout::Pt,
    /// Degrees, counter-clockwise as KiCad writes it
    pub rotation: f64,
    /// `x` or `y` as in KiCad's `(mirror x)`
    pub mirror: Option<char>,
}

/// The KiCad flags of a part; an empty `Facts` means the tool did not say (the KiCad default applies)
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PartFlags {
    pub dnp: Facts<bool>,
    pub in_bom: Facts<bool>,
    pub on_board: Facts<bool>,
    pub in_pos_files: Facts<bool>,
    pub exclude_from_sim: Facts<bool>,
}

/// The flags by their KiCad name, for writers and readers that go over all of them
impl PartFlags {
    pub const NAMES: [&'static str; 5] = [
        "dnp",
        "in_bom",
        "on_board",
        "in_pos_files",
        "exclude_from_sim",
    ];

    pub fn by_name(&self, name: &str) -> Option<&Facts<bool>> {
        Some(match name {
            "dnp" => &self.dnp,
            "in_bom" => &self.in_bom,
            "on_board" => &self.on_board,
            "in_pos_files" => &self.in_pos_files,
            "exclude_from_sim" => &self.exclude_from_sim,
            _ => return None,
        })
    }

    pub fn by_name_mut(&mut self, name: &str) -> Option<&mut Facts<bool>> {
        Some(match name {
            "dnp" => &mut self.dnp,
            "in_bom" => &mut self.in_bom,
            "on_board" => &mut self.on_board,
            "in_pos_files" => &mut self.in_pos_files,
            "exclude_from_sim" => &mut self.exclude_from_sim,
            _ => return None,
        })
    }

    /// Whether the part is fitted: not DNP (KiCad's default is fitted)
    pub fn fitted(&self) -> bool {
        !self.dnp.get().copied().unwrap_or(false)
    }
}

/// What a variant changes on a part; empty facts and no fields mean no change
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartVariant {
    pub variant: VariantId,
    pub flags: PartFlags,
    pub fields: Vec<Field>,
}

/// A pin of a placed part
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PinInst {
    pub part: PartId,
    pub number: String,
    pub name: String,
    /// The unit it is placed with; 0 when not known (a netlist)
    pub unit: u32,
    /// Electrical type: the symbol's, or the alternate's when one is chosen
    pub etype: Facts<EcadPinType>,
    pub hidden: bool,
    pub net: Option<NetId>,
    /// A no-connect flag sits on the pin
    pub no_connect: bool,
    /// The alternate pin function chosen on the placed symbol
    pub alt: Option<String>,
}

/// How a net got its name
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NetKind {
    /// A label or sheet pin names it
    Named,
    /// A power symbol or a hidden power pin names it
    Power,
    /// No name: KiCad's `Net-(R1-Pad2)`
    Auto,
    /// A pin alone, connected to nothing: KiCad's `unconnected-(U1-PA3-Pad12)`
    Unconnected,
}

impl NetKind {
    pub fn name(self) -> &'static str {
        match self {
            NetKind::Named => "named",
            NetKind::Power => "power",
            NetKind::Auto => "auto",
            NetKind::Unconnected => "unconnected",
        }
    }

    pub fn from_name(n: &str) -> Option<NetKind> {
        Some(match n {
            "named" => NetKind::Named,
            "power" => NetKind::Power,
            "auto" => NetKind::Auto,
            "unconnected" => NetKind::Unconnected,
            _ => return None,
        })
    }
}

/// A net
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Net {
    /// The full name as KiCad writes it in the netlist (`/MCU/SWDIO`, `+3V3`, `Net-(R1-Pad2)`)
    pub name: String,
    pub kind: NetKind,
    /// The net class: from the netlist, a directive label or the project's patterns
    pub class: Facts<String>,
    pub pins: Vec<PinId>,
    pub labels: Vec<LabelId>,
}

/// What kind of label
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LabelKind {
    /// Visible in its sheet
    Local,
    /// Visible everywhere
    Global,
    /// Connects to the sheet pin of the same name in the parent
    Hierarchical,
    /// A pin on a sheet symbol, in the parent sheet
    SheetPin,
    /// A power symbol (its text is the net name)
    Power,
    /// A directive label (net class flag); its text is the joined fields
    Directive,
}

impl LabelKind {
    pub fn name(self) -> &'static str {
        match self {
            LabelKind::Local => "local",
            LabelKind::Global => "global",
            LabelKind::Hierarchical => "hierarchical",
            LabelKind::SheetPin => "sheet_pin",
            LabelKind::Power => "power",
            LabelKind::Directive => "directive",
        }
    }

    pub fn from_name(n: &str) -> Option<LabelKind> {
        Some(match n {
            "local" => LabelKind::Local,
            "global" => LabelKind::Global,
            "hierarchical" => LabelKind::Hierarchical,
            "sheet_pin" => LabelKind::SheetPin,
            "power" => LabelKind::Power,
            "directive" => LabelKind::Directive,
            _ => return None,
        })
    }
}

/// A label, power symbol, sheet pin or directive label: the drawing facts WIRE checks need
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Label {
    pub kind: LabelKind,
    pub text: String,
    /// The sheet it is drawn on (for a sheet pin: the parent sheet)
    pub sheet: SheetId,
    pub at: crate::layout::Pt,
    /// `input`, `output`, `bidirectional`, `tri_state`, `passive` (global, hierarchical, sheet pins)
    pub shape: Option<String>,
    /// For a sheet pin: the child sheet it belongs to
    pub child: Option<SheetId>,
    /// Fields of a directive label (`Netclass` = `Power`)
    pub fields: Vec<(String, String)>,
    pub net: Option<NetId>,
    /// Attached to nothing (no wire, pin or other item at its anchor)
    pub dangling: bool,
}

/// A no-connect flag
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoConnect {
    pub sheet: SheetId,
    pub at: crate::layout::Pt,
    /// The pin it sits on, directly or through a wire
    pub pin: Option<PinId>,
}

/// A variant of the board (KiCad 10 variants, OrCAD per-variant BOMs)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    pub name: String,
    pub description: Option<String>,
}

/// A named parameter with its sources (a net class rule, a text variable, an ERC severity)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub key: String,
    pub value: Facts<String>,
}

impl Param {
    pub fn new(key: impl Into<String>, fact: Fact<String>) -> Param {
        Param {
            key: key.into(),
            value: Facts::one(fact),
        }
    }
}

/// A net class from the project file
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetClass {
    pub name: String,
    /// Rules of the class (`clearance`, `track_width`, `diff_pair_gap`, ...), in mm as the project writes them
    pub params: Vec<Param>,
    /// Net name patterns assigned to the class (`netclass_patterns`): wildcards, or a regex between slashes
    pub patterns: Vec<String>,
    /// Nets assigned to the class by name (`netclass_assignments`, written by the board editor)
    pub assigned: Vec<String>,
}

/// What the project file says beyond the drawing
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Text variables (`${REV}`)
    pub text_vars: Vec<Param>,
    /// ERC rule severities as the project states them (`pin_not_connected` = `ignore`)
    pub erc_severities: Vec<Param>,
    /// ERC exclusions as KiCad stores them, each a param with its comment as the value
    pub erc_exclusions: Vec<Param>,
    /// DRC rule severities as the project states them
    pub drc_severities: Vec<Param>,
    /// DRC exclusions, like the ERC ones
    pub drc_exclusions: Vec<Param>,
    /// Board design rules (`min_clearance`, `min_track_width`, ...)
    pub rules: Vec<Param>,
}

impl Board {
    pub fn part(&self, id: PartId) -> &Part {
        &self.parts[id.index()]
    }

    pub fn pin(&self, id: PinId) -> &PinInst {
        &self.pins[id.index()]
    }

    pub fn net(&self, id: NetId) -> &Net {
        &self.nets[id.index()]
    }

    pub fn sheet(&self, id: SheetId) -> &Sheet {
        &self.sheets[id.index()]
    }

    pub fn label(&self, id: LabelId) -> &Label {
        &self.labels[id.index()]
    }

    pub fn part_by_designator(&self, designator: &str) -> Option<PartId> {
        self.parts
            .iter()
            .position(|p| p.designator == designator)
            .map(PartId::from_index)
    }

    pub fn net_by_name(&self, name: &str) -> Option<NetId> {
        self.nets
            .iter()
            .position(|n| n.name == name)
            .map(NetId::from_index)
    }

    pub fn variant_by_name(&self, name: &str) -> Option<VariantId> {
        self.variants
            .iter()
            .position(|v| v.name == name)
            .map(VariantId::from_index)
    }

    /// The pins of a part with this number (usually one; connectors repeat `MP`)
    pub fn pins_by_number<'a>(
        &'a self,
        part: PartId,
        number: &'a str,
    ) -> impl Iterator<Item = PinId> + 'a {
        self.part(part)
            .pins
            .iter()
            .copied()
            .filter(move |p| self.pin(*p).number == number)
    }

    /// `/MCU/Power/` for a sheet two levels down, `/` for the root
    pub fn sheet_path(&self, id: SheetId) -> String {
        let mut names = Vec::new();
        let mut cur = Some(id);
        while let Some(c) = cur {
            let s = self.sheet(c);
            if s.parent.is_some() {
                names.push(s.name.as_str());
            }
            cur = s.parent;
        }
        names.reverse();
        let mut path = String::from("/");
        for n in names {
            path += n;
            path.push('/');
        }
        path
    }

    /// `R1.2`: designator and pin number
    pub fn pin_name(&self, id: PinId) -> String {
        let p = self.pin(id);
        format!("{}.{}", self.part(p.part).designator, p.number)
    }

    /// A real component: not a virtual `#` part (power symbols, PWR_FLAG), not excluded from the board
    pub fn is_component(&self, id: PartId) -> bool {
        let p = self.part(id);
        !p.designator.starts_with('#') && p.flags.on_board.get().copied().unwrap_or(true)
    }

    /// The variant's override of a part, if it has one
    fn part_variant(&self, id: PartId, variant: Option<VariantId>) -> Option<&PartVariant> {
        let v = variant?;
        self.part(id).variants.iter().find(|pv| pv.variant == v)
    }

    /// Whether the part is fitted in `variant` (`None`: the base design): the variant's DNP flag when it sets
    /// one, else the part's own
    pub fn fitted(&self, id: PartId, variant: Option<VariantId>) -> bool {
        match self
            .part_variant(id, variant)
            .and_then(|pv| pv.flags.dnp.get())
        {
            Some(dnp) => !dnp,
            None => self.part(id).flags.fitted(),
        }
    }

    /// The resolved value of a field in `variant`: the variant's override when it has one, else the part's own
    pub fn field(&self, id: PartId, name: &str, variant: Option<VariantId>) -> Option<&str> {
        self.part_variant(id, variant)
            .and_then(|pv| field_value(&pv.fields, name))
            .or_else(|| self.part(id).field(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn variant_views() {
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
        r2.flags.dnp = Facts::one(Fact::new(true, sch()));
        let board = Board {
            parts: vec![r1, r2, part("#PWR01", "GND")],
            variants: vec![Variant {
                name: "Lite".into(),
                description: None,
            }],
            ..Board::default()
        };
        let (r1, r2, pwr) = (PartId(0), PartId(1), PartId(2));
        let lite = Some(VariantId(0));
        assert!(board.fitted(r1, None));
        assert!(!board.fitted(r1, lite));
        assert!(!board.fitted(r2, None));
        assert!(!board.fitted(r2, lite), "no override: the base flag");
        assert_eq!(board.field(r1, "Value", None), Some("10k"));
        assert_eq!(board.field(r1, "Value", lite), Some("4k7"));
        assert_eq!(board.field(r2, "Value", lite), Some("1k"));
        assert!(board.is_component(r1));
        assert!(!board.is_component(pwr));
    }
}
