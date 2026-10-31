//! The schematic drawing: what each sheet looks like (`NET-26`), beyond the labels and flags of the board graph.
//! Paper and title block, placed symbols with their body box, pins and fields, labels, wires, buses, junctions,
//! free text, text boxes, tables and graphics, every text with its box computed from KiCad's stroke font. One
//! [`SheetGeometry`] per sheet file: a file used twice is drawn once. Things are named, not linked by id: a sheet
//! by its file, a part by its reference, so the geometry stays valid through [`crate::Board::canonicalize`].

use crate::design::{EcadPinType, LabelKind};
use crate::layout::Pt;
use serde::{Deserialize, Serialize};

/// Every sheet file of a board
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Geometry {
    pub sheets: Vec<SheetGeometry>,
}

/// A box in nanometres, y down; `min` is its top left corner
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct Rect {
    pub min: Pt,
    pub max: Pt,
}

impl Rect {
    /// The box of two corners, in any order
    pub fn of(a: Pt, b: Pt) -> Rect {
        Rect {
            min: Pt::new(a.x.min(b.x), a.y.min(b.y)),
            max: Pt::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    /// The box around points; `None` for no points
    pub fn around(points: impl IntoIterator<Item = Pt>) -> Option<Rect> {
        let mut it = points.into_iter();
        let first = it.next()?;
        Some(it.fold(Rect::of(first, first), |r, p| r.union(Rect::of(p, p))))
    }

    pub fn union(self, o: Rect) -> Rect {
        Rect {
            min: Pt::new(self.min.x.min(o.min.x), self.min.y.min(o.min.y)),
            max: Pt::new(self.max.x.max(o.max.x), self.max.y.max(o.max.y)),
        }
    }

    /// Grown by `margin` on every side
    pub fn grown(self, margin: i64) -> Rect {
        Rect {
            min: Pt::new(self.min.x - margin, self.min.y - margin),
            max: Pt::new(self.max.x + margin, self.max.y + margin),
        }
    }

    /// Whether the two boxes share area or an edge
    pub fn touches(self, o: Rect) -> bool {
        self.min.x <= o.max.x
            && o.min.x <= self.max.x
            && self.min.y <= o.max.y
            && o.min.y <= self.max.y
    }

    /// Whether the two boxes share area; boxes that only touch do not
    pub fn overlaps(self, o: Rect) -> bool {
        self.min.x < o.max.x && o.min.x < self.max.x && self.min.y < o.max.y && o.min.y < self.max.y
    }

    /// The box both share, when they share area
    pub fn intersection(self, o: Rect) -> Option<Rect> {
        self.overlaps(o).then(|| Rect {
            min: Pt::new(self.min.x.max(o.min.x), self.min.y.max(o.min.y)),
            max: Pt::new(self.max.x.min(o.max.x), self.max.y.min(o.max.y)),
        })
    }

    /// The gap between the two boxes, 0 when they touch or overlap
    pub fn gap(self, o: Rect) -> i64 {
        let dx = (o.min.x - self.max.x).max(self.min.x - o.max.x).max(0);
        let dy = (o.min.y - self.max.y).max(self.min.y - o.max.y).max(0);
        dx.max(dy)
    }

    pub fn width(self) -> i64 {
        self.max.x - self.min.x
    }

    pub fn height(self) -> i64 {
        self.max.y - self.min.y
    }

    pub fn center(self) -> Pt {
        Pt::new((self.min.x + self.max.x) / 2, (self.min.y + self.max.y) / 2)
    }

    pub fn contains(self, p: Pt) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
}

/// One of the four directions on the sheet
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

impl Dir {
    /// The unit step, y down
    pub fn step(self) -> (i64, i64) {
        match self {
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
        }
    }

    pub fn opposite(self) -> Dir {
        match self {
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
        }
    }

    pub fn is_horizontal(self) -> bool {
        matches!(self, Dir::Left | Dir::Right)
    }

    pub fn name(self) -> &'static str {
        match self {
            Dir::Left => "left",
            Dir::Right => "right",
            Dir::Up => "up",
            Dir::Down => "down",
        }
    }
}

/// How a text sits on its anchor along its reading direction
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// How a text sits on its anchor across its reading direction
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VAlign {
    Top,
    #[default]
    Center,
    Bottom,
}

/// A colour as a file writes it: red, green, blue 0..255; alpha 0..1
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

impl Colour {
    /// A colour as KiCad's settings write it, `rgb(r, g, b)` or `rgba(r, g, b, a)`; `None` for another text and for
    /// alpha 0, KiCad's "not set"
    pub fn parse(text: &str) -> Option<Colour> {
        let (head, rest) = text.trim().split_once('(')?;
        let mut parts = rest.strip_suffix(')')?.split(',').map(str::trim);
        let mut byte = || parts.next()?.parse::<u8>().ok();
        let (r, g, b) = (byte()?, byte()?, byte()?);
        let a = match (head, parts.next()) {
            ("rgb", None) => 1.0,
            ("rgba", Some(a)) => a.parse().ok()?,
            _ => return None,
        };
        (a > 0.0).then_some(Colour { r, g, b, a })
    }
}

impl std::fmt::Display for Colour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
    }
}

/// What a KiCad colour theme gives the items a wire can touch; `None` for a key the file leaves out
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub wire: Option<Colour>,
    pub pin: Option<Colour>,
    pub label_local: Option<Colour>,
    pub label_global: Option<Colour>,
    pub label_hier: Option<Colour>,
    pub component_outline: Option<Colour>,
}

impl Theme {
    /// A label's colour; a directive label has its own theme key and no text to tell apart
    pub fn label(&self, kind: LabelKind) -> Option<Colour> {
        match kind {
            LabelKind::Local => self.label_local,
            LabelKind::Global => self.label_global,
            LabelKind::Hierarchical => self.label_hier,
            _ => None,
        }
    }
}

/// A text as drawn
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Text {
    /// As shown: markup and escapes gone, lines joined by `\n`
    pub text: String,
    /// The anchor
    pub at: Pt,
    /// Reads upwards, not left to right
    pub vertical: bool,
    /// Character height and width
    pub size: (i64, i64),
    pub bold: bool,
    pub italic: bool,
    pub halign: HAlign,
    pub valign: VAlign,
    pub colour: Option<Colour>,
    /// The box the glyphs fill, line pitch included
    pub bbox: Rect,
}

/// A stroke as a file writes it
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    /// 0 for the theme's default
    pub width: i64,
    /// `solid`, `dash`, `dot`, `dash_dot`, `default`
    pub style: String,
    pub colour: Option<Colour>,
}

/// A pin of a placed symbol
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawnPin {
    pub number: String,
    pub name: String,
    pub etype: EcadPinType,
    /// The connection point
    pub at: Pt,
    /// Which way a wire leaves the pin: from the body out past the connection point
    pub outward: Dir,
    pub length: i64,
    pub hidden: bool,
    /// The pin name as drawn, when shown
    pub name_box: Option<Rect>,
    /// The pin number as drawn, when shown
    pub number_box: Option<Rect>,
}

impl DrawnPin {
    /// The end of the pin at the body
    pub fn root(&self) -> Pt {
        let (dx, dy) = self.outward.step();
        Pt::new(self.at.x - dx * self.length, self.at.y - dy * self.length)
    }
}

/// A field of a placed symbol
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawnField {
    pub name: String,
    pub text: Text,
    pub hidden: bool,
}

/// A placed symbol: one unit of a part, or a power symbol
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawnSymbol {
    /// The reference without a unit letter
    pub reference: String,
    pub lib_id: String,
    pub unit: u32,
    /// A power or ground symbol; `value` names its net
    pub power: bool,
    /// The Value field
    pub value: String,
    pub at: Pt,
    /// Degrees, counter-clockwise as KiCad writes it
    pub rotation: i32,
    pub mirror: Option<char>,
    /// The library graphics as placed, pins left out; `None` for a symbol drawn with pins alone
    pub body: Option<Rect>,
    /// The stroke colour the library graphics set; `None` for the theme's
    pub colour: Option<Colour>,
    pub pins: Vec<DrawnPin>,
    pub fields: Vec<DrawnField>,
    pub uuid: String,
    /// The line of `(symbol` in the file
    pub line: Option<u32>,
}

impl DrawnSymbol {
    /// The body with its pins
    pub fn outline(&self) -> Rect {
        let pins = self.pins.iter().filter(|p| !p.hidden).map(|p| p.at);
        let around = Rect::around(pins.chain([self.at])).unwrap_or_default();
        self.body.map_or(around, |b| {
            if self.pins.iter().any(|p| !p.hidden) {
                b.union(Rect::around(self.pins.iter().map(|p| p.at)).unwrap_or(b))
            } else {
                b
            }
        })
    }
}

/// A label, as drawn
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawnLabel {
    pub kind: LabelKind,
    /// The connection point
    pub at: Pt,
    /// `input`, `output`, ... for a global or hierarchical label
    pub shape: Option<String>,
    /// The name; [`Text::at`] is where it starts to be drawn, not the connection point
    pub text: Text,
    /// Everything drawn: the name and, for a global or hierarchical label, its flag
    pub bbox: Rect,
    /// The direction the text runs from the connection point
    pub runs: Dir,
    pub uuid: String,
    pub line: Option<u32>,
}

/// A free text item
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawnText {
    pub text: Text,
    pub uuid: String,
    pub line: Option<u32>,
}

/// A text box or a table cell
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawnTextBox {
    pub rect: Rect,
    pub stroke: Stroke,
    pub fill: Option<Colour>,
    /// The text inside, as shown; its box is the text's extent, inside `rect` when it fits
    pub text: Text,
    pub uuid: String,
    pub line: Option<u32>,
}

/// A table
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawnTable {
    pub rect: Rect,
    pub cells: Vec<DrawnTextBox>,
    pub uuid: String,
    pub line: Option<u32>,
}

/// A graphic line or shape on the sheet
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawnGraphic {
    /// `polyline`, `rectangle`, `circle`, `arc`, `bezier`
    pub kind: String,
    /// The points as written; a rectangle by its two corners
    pub points: Vec<Pt>,
    pub bbox: Rect,
    pub stroke: Stroke,
    pub fill: Option<Colour>,
    pub uuid: String,
    pub line: Option<u32>,
}

/// A wire or a bus segment
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub a: Pt,
    pub b: Pt,
    pub stroke: Stroke,
    pub uuid: String,
    pub line: Option<u32>,
}

impl Segment {
    pub fn bbox(&self) -> Rect {
        Rect::of(self.a, self.b)
    }

    /// Manhattan length (the diagonal of a diagonal wire counts both ways)
    pub fn len(&self) -> i64 {
        (self.a.x - self.b.x).abs() + (self.a.y - self.b.y).abs()
    }

    pub fn is_empty(&self) -> bool {
        self.a == self.b
    }

    /// Whether `p` lies on the segment, ends included
    pub fn contains(&self, p: Pt) -> bool {
        let (a, b) = (self.a, self.b);
        let cross =
            (b.x - a.x) as i128 * (p.y - a.y) as i128 - (b.y - a.y) as i128 * (p.x - a.x) as i128;
        cross == 0 && self.bbox().contains(p)
    }
}

/// A pin of a sheet symbol
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawnSheetPin {
    pub name: String,
    pub shape: String,
    pub at: Pt,
    /// The side of the sheet box it sits on
    pub side: Dir,
    pub text: Text,
}

/// A sheet symbol: a box in the parent sheet
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawnSheet {
    pub rect: Rect,
    pub name: String,
    pub file: String,
    pub pins: Vec<DrawnSheetPin>,
    pub fields: Vec<DrawnField>,
    pub uuid: String,
}

/// A group of items, by the items' UUIDs
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DrawnGroup {
    pub name: String,
    pub members: Vec<String>,
    pub uuid: String,
}

/// The title block of a sheet
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TitleBlock {
    pub title: String,
    pub date: String,
    pub rev: String,
    pub company: String,
    /// Comment number and text
    pub comments: Vec<(u32, String)>,
}

/// One sheet file as drawn
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SheetGeometry {
    /// The sheet file, relative to the project directory, as [`crate::Sheet::file`] says it
    pub file: String,
    /// `A4`, `A3`, `User`
    pub paper: String,
    /// Width and height of the sheet in nanometres, as turned
    pub paper_size: (i64, i64),
    pub title_block: TitleBlock,
    pub symbols: Vec<DrawnSymbol>,
    pub wires: Vec<Segment>,
    pub buses: Vec<Segment>,
    pub junctions: Vec<Pt>,
    pub no_connects: Vec<Pt>,
    pub labels: Vec<DrawnLabel>,
    pub texts: Vec<DrawnText>,
    pub text_boxes: Vec<DrawnTextBox>,
    pub tables: Vec<DrawnTable>,
    pub graphics: Vec<DrawnGraphic>,
    pub sheets: Vec<DrawnSheet>,
    pub groups: Vec<DrawnGroup>,
}
