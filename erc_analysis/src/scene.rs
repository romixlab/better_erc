//! Each sheet as the drawing checks see it (`NET-26`): every drawn thing as an [`Item`] with its box, and the wires,
//! pins and labels that touch joined into [`Tie`]s. The geometry names things and links nothing, so this is where
//! a sheet's file meets its [`SheetId`] and a wire learns what it is joined to.

use erc_model::geometry::{Dir, DrawnPin, DrawnSymbol, Rect, Segment, SheetGeometry};
use erc_model::{Board, Pt, SheetId};
use std::collections::BTreeMap;

/// What an [`Item`] is
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemKind {
    /// A part's body
    Body,
    /// A pin's stub, from the body to its connection point
    Pin,
    /// A power or ground symbol with its name
    Power,
    Field,
    PinName,
    PinNumber,
    Label,
    Text,
    TextBox,
    Table,
    /// A sheet symbol's box
    Sheet,
    /// A sheet pin's name
    SheetPin,
    Wire,
    Bus,
}

impl ItemKind {
    /// Text of some sort, as opposed to something drawn
    pub fn is_text(self) -> bool {
        matches!(
            self,
            ItemKind::Field
                | ItemKind::PinName
                | ItemKind::PinNumber
                | ItemKind::Label
                | ItemKind::Text
                | ItemKind::TextBox
                | ItemKind::Table
                | ItemKind::SheetPin
        )
    }

    pub fn is_wire(self) -> bool {
        matches!(self, ItemKind::Wire | ItemKind::Bus)
    }
}

/// One drawn thing with the box it fills
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub kind: ItemKind,
    /// What a person calls it: `R1 body`, `R1 Value`, `label SDA`, `wire of R1.2`
    pub name: String,
    pub rect: Rect,
    /// The index of the symbol or sheet symbol it belongs to, in the scene's geometry
    pub owner: Option<usize>,
    /// The tie a wire, a pin or a label is on
    pub tie: Option<usize>,
    /// The UUID of the file's item, to match against groups
    pub uuid: String,
    /// The line in the file, where the reader knows it
    pub line: Option<u32>,
}

/// A pin in a scene's geometry
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PinRef {
    pub symbol: usize,
    pub pin: usize,
}

/// Wires, pins and labels joined by touching: ends that meet, a junction, a pin or a label on a wire. A wire end on
/// the middle of another wire without a junction does not join (`DRAW-4`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tie {
    /// By index into the geometry's wires
    pub wires: Vec<usize>,
    pub pins: Vec<PinRef>,
    /// By index into the geometry's labels
    pub labels: Vec<usize>,
}

/// One sheet file as the drawing checks see it
#[derive(Clone, Debug, PartialEq)]
pub struct Scene<'a> {
    /// The first sheet drawn from this file; `None` when the board's sheets do not name it
    pub sheet: Option<SheetId>,
    pub geometry: &'a SheetGeometry,
    pub items: Vec<Item>,
    pub ties: Vec<Tie>,
}

impl<'a> Scene<'a> {
    /// The sheets of a board, empty when it has no geometry
    pub fn all(board: &'a Board) -> Vec<Scene<'a>> {
        let Some(geometry) = &board.geometry else {
            return Vec::new();
        };
        geometry
            .sheets
            .iter()
            .map(|sheet| Scene::of(board, sheet))
            .collect()
    }

    fn of(board: &Board, g: &'a SheetGeometry) -> Scene<'a> {
        let sheet = board
            .sheets
            .iter()
            .position(|s| s.file == g.file)
            .map(|i| SheetId(i as u32));
        let (ties, of_wire, of_pin, of_label) = ties(g);
        let name_of = |t: usize| tie_name(g, &ties[t]);
        let mut items = Vec::new();
        for (i, s) in g.symbols.iter().enumerate() {
            symbol_items(s, i, &of_pin, &mut items);
        }
        for (i, w) in g.wires.iter().enumerate() {
            items.push(Item {
                kind: ItemKind::Wire,
                name: format!("wire of {}", name_of(of_wire[i])),
                rect: w.bbox(),
                owner: None,
                tie: Some(of_wire[i]),
                uuid: w.uuid.clone(),
                line: w.line,
            });
        }
        for b in &g.buses {
            items.push(Item {
                kind: ItemKind::Bus,
                name: format!("bus at {}", place(b.a)),
                rect: b.bbox(),
                owner: None,
                tie: None,
                uuid: b.uuid.clone(),
                line: b.line,
            });
        }
        for (i, l) in g.labels.iter().enumerate() {
            items.push(Item {
                kind: ItemKind::Label,
                name: format!("label {}", l.text.text),
                rect: l.bbox,
                owner: None,
                tie: Some(of_label[i]),
                uuid: l.uuid.clone(),
                line: l.line,
            });
        }
        let free = |kind, what, text: &str, rect, uuid: &str, line| Item {
            kind,
            name: format!("{what} \"{}\"", gist(text)),
            rect,
            owner: None,
            tie: None,
            uuid: uuid.to_string(),
            line,
        };
        for t in &g.texts {
            let (text, rect) = (&t.text.text, t.text.bbox);
            items.push(free(ItemKind::Text, "text", text, rect, &t.uuid, t.line));
        }
        for t in &g.text_boxes {
            let (text, rect) = (&t.text.text, t.rect);
            items.push(free(
                ItemKind::TextBox,
                "text box",
                text,
                rect,
                &t.uuid,
                t.line,
            ));
        }
        for t in &g.tables {
            let first = t.cells.first().map_or("", |c| c.text.text.as_str());
            items.push(free(
                ItemKind::Table,
                "table",
                first,
                t.rect,
                &t.uuid,
                t.line,
            ));
        }
        for (i, s) in g.sheets.iter().enumerate() {
            let sheet_item = |kind, name, rect| Item {
                kind,
                name,
                rect,
                owner: Some(i),
                tie: None,
                uuid: s.uuid.clone(),
                line: None,
            };
            items.push(sheet_item(
                ItemKind::Sheet,
                format!("sheet {}", s.name),
                s.rect,
            ));
            for p in &s.pins {
                let name = format!("sheet {} pin {}", s.name, p.name);
                items.push(sheet_item(ItemKind::SheetPin, name, p.text.bbox));
            }
        }
        Scene {
            sheet,
            geometry: g,
            items,
            ties,
        }
    }

    /// The tie's name: a label, a power symbol's net, else a pin
    pub fn tie_name(&self, tie: usize) -> String {
        tie_name(self.geometry, &self.ties[tie])
    }

    /// The item of a pin: its stub, or the power symbol for a power symbol's pin
    pub fn pin_item(&self, pin: PinRef) -> Option<&Item> {
        let s = &self.geometry.symbols[pin.symbol];
        let name = format!("{}.{} pin", s.reference, s.pins[pin.pin].number);
        let of_symbol = |i: &&Item| i.owner == Some(pin.symbol);
        let mut own = self.items.iter().filter(of_symbol);
        own.find(|i| {
            if s.power {
                i.kind == ItemKind::Power
            } else {
                i.kind == ItemKind::Pin && i.name == name
            }
        })
    }

    /// Every pin that can take a wire: a power symbol's hidden pin counts
    pub fn pins(&self) -> impl Iterator<Item = (PinRef, &'a DrawnPin)> + 'a {
        let g = self.geometry;
        g.symbols.iter().enumerate().flat_map(|(symbol, s)| {
            let live = s.pins.iter().enumerate();
            live.filter(|(_, p)| s.power || !p.hidden)
                .map(move |(pin, p)| (PinRef { symbol, pin }, p))
        })
    }

    /// How far wire runs straight on from `from` in `dir`, over the wires that continue it: the end of the run
    pub fn run_end(&self, from: Pt, dir: Dir) -> Pt {
        let (dx, dy) = dir.step();
        let beyond = |p: Pt, q: Pt| (q.x - p.x) * dx + (q.y - p.y) * dy;
        let mut end = from;
        loop {
            let along = self
                .geometry
                .wires
                .iter()
                .filter(|w| !w.is_empty() && w.contains(end) && (w.a.x == w.b.x) == (dx == 0));
            let next = along
                .flat_map(|w| [w.a, w.b])
                .filter(|q| beyond(end, *q) > 0)
                .max_by_key(|q| beyond(end, *q));
            match next {
                Some(q) => end = q,
                None => return end,
            }
        }
    }

    /// Whether a wire leaves `p` across `dir`: where a straight run ending there bends or branches
    pub fn bends_at(&self, p: Pt, dir: Dir) -> bool {
        let across = |w: &Segment| (w.a.x == w.b.x) == dir.is_horizontal();
        let wires = self.geometry.wires.iter();
        wires
            .filter(|w| !w.is_empty())
            .any(|w| w.contains(p) && across(w))
    }

    /// Whether the two items are in one group of the sheet
    pub fn grouped(&self, a: &Item, b: &Item) -> bool {
        self.geometry
            .groups
            .iter()
            .any(|g| g.members.contains(&a.uuid) && g.members.contains(&b.uuid))
    }
}

fn symbol_items(
    s: &DrawnSymbol,
    index: usize,
    of_pin: &BTreeMap<PinRef, usize>,
    items: &mut Vec<Item>,
) {
    let item = |kind, name, rect, tie| Item {
        kind,
        name,
        rect,
        owner: Some(index),
        tie,
        uuid: s.uuid.clone(),
        line: s.line,
    };
    if s.power {
        let shown = s
            .fields
            .iter()
            .filter(|f| !f.hidden && !f.text.text.is_empty());
        let rect = shown.fold(s.outline(), |r, f| r.union(f.text.bbox));
        let tie = (0..s.pins.len())
            .find_map(|pin| of_pin.get(&PinRef { symbol: index, pin }))
            .copied();
        items.push(item(
            ItemKind::Power,
            format!("power {}", s.value),
            rect,
            tie,
        ));
        return;
    }
    if let Some(body) = s.body {
        items.push(item(
            ItemKind::Body,
            format!("{} body", s.reference),
            body,
            None,
        ));
    }
    for (i, p) in s.pins.iter().enumerate() {
        if p.hidden {
            continue;
        }
        let tie = of_pin
            .get(&PinRef {
                symbol: index,
                pin: i,
            })
            .copied();
        let name = format!("{}.{}", s.reference, p.number);
        items.push(item(
            ItemKind::Pin,
            format!("{name} pin"),
            Rect::of(p.root(), p.at),
            tie,
        ));
        if let Some(r) = p.name_box {
            items.push(item(ItemKind::PinName, format!("{name} name"), r, tie));
        }
        if let Some(r) = p.number_box {
            items.push(item(ItemKind::PinNumber, format!("{name} number"), r, tie));
        }
    }
    for f in s
        .fields
        .iter()
        .filter(|f| !f.hidden && !f.text.text.is_empty())
    {
        let name = format!("{} {}", s.reference, f.name);
        items.push(item(ItemKind::Field, name, f.text.bbox, None));
    }
}

/// The first line of a text, cut short for a name
fn gist(text: &str) -> String {
    const MAX_CHARS: usize = 24;
    let line = text.lines().next().unwrap_or("").trim();
    if line.chars().count() > MAX_CHARS {
        let cut: String = line.chars().take(MAX_CHARS).collect();
        format!("{cut}...")
    } else {
        line.to_string()
    }
}

/// `x 12.7 y 25.4`, in millimetres
pub fn place(p: Pt) -> String {
    format!("x {} y {}", mm(p.x), mm(p.y))
}

/// Nanometres in millimetres, as short as a position on the sheet allows
pub fn mm(nm: i64) -> f64 {
    (nm as f64 / 1e4).round() / 100.0
}

fn tie_name(g: &SheetGeometry, tie: &Tie) -> String {
    let label = tie.labels.iter().map(|l| &g.labels[*l].text.text).min();
    let power = tie
        .pins
        .iter()
        .map(|p| &g.symbols[p.symbol])
        .find(|s| s.power)
        .map(|s| &s.value);
    if let Some(name) = label.or(power) {
        return name.clone();
    }
    let pin = tie
        .pins
        .iter()
        .map(|p| {
            let s = &g.symbols[p.symbol];
            format!("{}.{}", s.reference, s.pins[p.pin].number)
        })
        .min_by(|a, b| erc_model::natural_cmp(a, b));
    pin.unwrap_or_else(|| {
        let at = tie.wires.first().map(|w| g.wires[*w].a).unwrap_or_default();
        place(at)
    })
}

/// Union-find over wires, pins and labels
struct Joined(Vec<usize>);

impl Joined {
    fn root(&mut self, mut i: usize) -> usize {
        while self.0[i] != i {
            self.0[i] = self.0[self.0[i]];
            i = self.0[i];
        }
        i
    }

    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.root(a), self.root(b));
        // The lower index stays the root, so ties come out in file order
        self.0[a.max(b)] = a.min(b);
    }
}

type TieMaps = (Vec<Tie>, Vec<usize>, BTreeMap<PinRef, usize>, Vec<usize>);

/// The ties of a sheet, and which tie each wire, pin and label is on
fn ties(g: &SheetGeometry) -> TieMaps {
    let wires = g.wires.len();
    let pins: Vec<PinRef> = g
        .symbols
        .iter()
        .enumerate()
        .flat_map(|(symbol, s)| {
            // A power symbol's one pin is hidden and still its connection
            let live = s
                .pins
                .iter()
                .enumerate()
                .filter(|(_, p)| s.power || !p.hidden);
            live.map(move |(pin, _)| PinRef { symbol, pin })
        })
        .collect();
    let at_pin = |r: &PinRef| g.symbols[r.symbol].pins[r.pin].at;
    let labels = wires + pins.len();
    let mut joined = Joined((0..labels + g.labels.len()).collect());
    let mut at_point: BTreeMap<Pt, Vec<usize>> = BTreeMap::new();
    for (i, w) in g.wires.iter().enumerate() {
        at_point.entry(w.a).or_default().push(i);
        at_point.entry(w.b).or_default().push(i);
    }
    for (i, p) in pins.iter().enumerate() {
        at_point.entry(at_pin(p)).or_default().push(wires + i);
    }
    for (i, l) in g.labels.iter().enumerate() {
        at_point.entry(l.at).or_default().push(labels + i);
    }
    for same in at_point.values() {
        for pair in same.windows(2) {
            joined.join(pair[0], pair[1]);
        }
    }
    for j in &g.junctions {
        let on: Vec<usize> = (0..wires).filter(|w| g.wires[*w].contains(*j)).collect();
        for pair in on.windows(2) {
            joined.join(pair[0], pair[1]);
        }
    }
    for (i, w) in g.wires.iter().enumerate() {
        for (k, p) in pins.iter().enumerate() {
            if w.contains(at_pin(p)) {
                joined.join(i, wires + k);
            }
        }
        for (k, l) in g.labels.iter().enumerate() {
            if w.contains(l.at) {
                joined.join(i, labels + k);
            }
        }
    }
    let mut index: BTreeMap<usize, usize> = BTreeMap::new();
    let mut ties: Vec<Tie> = Vec::new();
    let total = labels + g.labels.len();
    let roots: Vec<usize> = (0..total).map(|i| joined.root(i)).collect();
    let mut tie_of = |root: usize, ties: &mut Vec<Tie>| {
        *index.entry(root).or_insert_with(|| {
            ties.push(Tie::default());
            ties.len() - 1
        })
    };
    let mut of_wire = Vec::with_capacity(wires);
    for (i, root) in roots.iter().enumerate().take(wires) {
        let t = tie_of(*root, &mut ties);
        ties[t].wires.push(i);
        of_wire.push(t);
    }
    let mut of_pin = BTreeMap::new();
    for (i, p) in pins.iter().enumerate() {
        let t = tie_of(roots[wires + i], &mut ties);
        ties[t].pins.push(*p);
        of_pin.insert(*p, t);
    }
    let mut of_label = Vec::with_capacity(g.labels.len());
    for i in 0..g.labels.len() {
        let t = tie_of(roots[labels + i], &mut ties);
        ties[t].labels.push(i);
        of_label.push(t);
    }
    (ties, of_wire, of_pin, of_label)
}
