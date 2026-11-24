//! `TOOL-13`: the fix side of `DRAW-9` and `DRAW-34` for symbol fields. Each symbol whose shown fields overlap
//! something, or come closer than its margin, gets them stacked in one column beside its body, on the side where
//! the column hits nothing, judged by the same pair tests the two checks use. The layout follows one style, the
//! `drawing.fields.*` settings: a column that overlaps nothing but strays from it is placed again.

use super::overlap::{Margins, bare_hit, close_hit, exempt};
use super::{
    FIELDS_BODY_GAP, FIELDS_ENDS_RAISE, FIELDS_GAP_MAX, FIELDS_HIDE_DEFAULTS, FIELDS_PITCH,
    FIELDS_STEP, MM, draw_34,
};
use crate::Ctx;
use crate::meta::Param;
use erc_analysis::scene::{Item, ItemKind as K, Scene};
use erc_model::Pt;
use erc_model::geometry::{DrawnField, DrawnSymbol, HAlign, Rect, VAlign, text_rect};

/// Margin of the drawing frame inside the paper edge, as KiCad's default page layout draws it
const FRAME: i64 = 10_000_000;
/// How far a column may slide along its side when it does not fit centred
const SLIDE_REACH: i64 = 5_000_000;
/// How much farther out from the body a column may stand when it does not fit next to it
const OUT_REACH: i64 = 2_500_000;
/// The finest step fields may be placed on: a finer one multiplies the columns to try past use
const STEP_MIN: f64 = 0.05;

/// The field layout the placer makes, from the `drawing.fields.*` settings, in nanometres
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldStyle {
    /// Baseline-to-baseline distance between the lines of a column
    pub pitch: i64,
    /// Space between a column, or a packed part's Reference and Value, and the body and pins
    pub body_gap: i64,
    /// The grid anchors are placed on, and the step a column slides or moves out by
    pub step: i64,
    /// Fields whose nearest one is farther than this from their body read as another part's: they are placed again
    pub gap_max: i64,
    /// How far above a horizontal packed part's pin line its Reference and Value sit, clear of the wire
    pub ends_raise: i64,
}

/// The rating values the placer may hide when a symbol's fields need room: the values most parts of their kind
/// carry, so a rating left shown reads as a choice. They come from the project's text variables
/// `DEFAULT_R_PACKAGE`, `DEFAULT_R_POWER`, `DEFAULT_C_PACKAGE`, `DEFAULT_C_DIELECTRIC` and `DEFAULT_C_VOLTAGE`,
/// the ones the drawing sheet's Defaults box shows. `None` (no variable, or an empty one): never hidden.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldDefaults {
    pub r_package: Option<String>,
    pub r_power: Option<String>,
    pub c_package: Option<String>,
    pub c_dielectric: Option<String>,
    pub c_voltage: Option<String>,
    /// Hide every shown rating at its default, not only those in the way ([`plan_default_hides`])
    pub always: bool,
}

impl FieldDefaults {
    /// The defaults of the board of `cx`: its project's text variables, and `drawing.fields.hide_defaults`
    pub fn of(cx: &Ctx) -> FieldDefaults {
        let vars = &cx.board().settings.text_vars;
        let var = |name: &str| {
            vars.iter()
                .find(|v| v.key == name)
                .and_then(|v| v.value.get())
                .map(|v| v.trim().to_owned())
                .filter(|v| !v.is_empty())
        };
        FieldDefaults {
            r_package: var("DEFAULT_R_PACKAGE"),
            r_power: var("DEFAULT_R_POWER"),
            c_package: var("DEFAULT_C_PACKAGE"),
            c_dielectric: var("DEFAULT_C_DIELECTRIC"),
            c_voltage: var("DEFAULT_C_VOLTAGE"),
            always: draw_34::field_choice(cx, &FIELDS_HIDE_DEFAULTS) == "always",
        }
    }

    /// Whether the placer may hide `field` of the part `reference`: a resistor's or capacitor's rating that holds
    /// its default, compared without case or spaces
    pub fn hideable(&self, reference: &str, field: &DrawnField) -> bool {
        let prefix: String = reference
            .chars()
            .take_while(char::is_ascii_alphabetic)
            .collect();
        let default = match (prefix.as_str(), field.name.as_str()) {
            ("R", "Package") => &self.r_package,
            ("R", "Power") => &self.r_power,
            ("C", "Package") => &self.c_package,
            ("C", "Dielectric") => &self.c_dielectric,
            ("C", "Voltage") => &self.c_voltage,
            _ => &None,
        };
        let plain = |v: &str| v.split_whitespace().collect::<String>().to_lowercase();
        default
            .as_deref()
            .is_some_and(|d| plain(&field.text.text) == plain(d))
    }
}

/// A `drawing.fields.*` setting out of its range
#[derive(Clone, Debug, PartialEq)]
pub struct StyleError {
    pub setting: &'static str,
    pub value: f64,
    /// The least value it may take, in millimetres
    pub least: f64,
}

impl std::fmt::Display for StyleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} = {}: must be a number of at least {} mm",
            self.setting, self.value, self.least
        )
    }
}

impl std::error::Error for StyleError {}

impl FieldStyle {
    /// The style the board's settings give
    pub fn of(cx: &Ctx) -> Result<FieldStyle, StyleError> {
        let get = |p: &Param, least: f64| {
            let value = draw_34::field_setting(cx, p);
            if value.is_finite() && value >= least {
                Ok((value * MM).round() as i64)
            } else {
                Err(StyleError {
                    setting: p.name,
                    value,
                    least,
                })
            }
        };
        Ok(FieldStyle {
            pitch: get(&FIELDS_PITCH, STEP_MIN)?,
            body_gap: get(&FIELDS_BODY_GAP, 0.0)?,
            step: get(&FIELDS_STEP, STEP_MIN)?,
            gap_max: get(&FIELDS_GAP_MAX, 0.0)?,
            ends_raise: get(&FIELDS_ENDS_RAISE, 0.0)?,
        })
    }

    /// `v` rounded up to the step
    fn up(self, v: i64) -> i64 {
        let step = self.step;
        v.div_euclid(step) * step + if v.rem_euclid(step) == 0 { 0 } else { step }
    }

    /// `v` rounded down to the step
    fn down(self, v: i64) -> i64 {
        v.div_euclid(self.step) * self.step
    }

    /// `v` rounded to the nearest step
    fn nearest(self, v: i64) -> i64 {
        self.down(v + self.step / 2)
    }
}

/// Where a symbol's fields go
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// A column on the right of the body
    Right,
    Left,
    Above,
    Below,
    /// A two-pin part packed against its neighbours: Reference and Value at the two ends of the body (before and
    /// after a horizontal part, above and below a vertical one), the other fields hidden
    Ends,
    /// Where they are, the rating fields in the way hidden
    InPlace,
}

impl Side {
    pub fn name(self) -> &'static str {
        match self {
            Side::Right => "right",
            Side::Left => "left",
            Side::Above => "above",
            Side::Below => "below",
            Side::Ends => "at the ends of the part",
            Side::InPlace => "in place",
        }
    }

    fn is_column(self) -> bool {
        matches!(self, Side::Right | Side::Left | Side::Above | Side::Below)
    }
}

/// One field moved; field text is always upright
#[derive(Clone, Debug, PartialEq)]
pub struct FieldMove {
    /// The field's name (`Value`)
    pub field: String,
    /// The anchor: the text is vertically centred on it
    pub at: Pt,
    pub halign: HAlign,
    /// The text's box at its new place
    pub rect: Rect,
}

/// Something a field still overlaps (`DRAW-9`) or comes too close to (`DRAW-34`)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Remaining {
    /// `R1 Value`
    pub field: String,
    /// `C3 body`, `the sheet edge`
    pub other: String,
    /// Within the margins only, not overlapping
    pub close: bool,
}

/// A symbol whose fields overlap something: where they go, or why they stay
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolPlan {
    /// The sheet file, relative to the project directory
    pub file: String,
    pub reference: String,
    /// The UUID of the placed symbol in its file
    pub uuid: String,
    /// Where its fields go; `None` when no place is better than where they are
    pub side: Option<Side>,
    /// Empty when the fields stay
    pub moves: Vec<FieldMove>,
    /// The shown fields to hide, by name
    pub hidden: Vec<String>,
    /// What the fields still overlap or come too close to once every symbol of the sheet is placed
    pub left: Vec<Remaining>,
}

/// The fields of a design to move
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldPlan {
    pub symbols: Vec<SymbolPlan>,
}

/// The `DRAW-9` and `DRAW-34` pairs of a design, all and those with a field in them
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OverlapCount {
    pub draw_9: usize,
    pub draw_34: usize,
    pub draw_9_fields: usize,
    pub draw_34_fields: usize,
}

/// Whether the scene is the sheet named `sheet`: by its name in the parent or by its file, with or without the
/// extension; every scene without a name
fn on_sheet(cx: &Ctx, scene: &Scene, sheet: Option<&str>) -> bool {
    let Some(want) = sheet else { return true };
    let file = &scene.geometry.file;
    let stem = file.strip_suffix(".kicad_sch").unwrap_or(file);
    let name = scene
        .sheet
        .and_then(|s| cx.board().sheets.get(s.0 as usize))
        .map(|s| s.name.as_str());
    file == want || stem == want || name == Some(want)
}

/// Counts what `DRAW-9` and `DRAW-34` find on the board of `cx`, on one sheet or all
pub fn count_overlaps(cx: &Ctx, sheet: Option<&str>) -> OverlapCount {
    let m = draw_34::margins(cx);
    let mut out = OverlapCount::default();
    for scene in cx.scenes().iter().filter(|s| on_sheet(cx, s, sheet)) {
        let field = |a: &Item, b: &Item| a.kind == K::Field || b.kind == K::Field;
        for (a, b, _) in super::overlap::bare(scene, false) {
            out.draw_9 += 1;
            out.draw_9_fields += usize::from(field(a, b));
        }
        for (a, b, _) in super::overlap::close_with(scene, m) {
            out.draw_34 += 1;
            out.draw_34_fields += usize::from(field(a, b));
        }
    }
    out
}

/// Plans the fields of the board of `cx`, on one sheet or all. Symbols whose fields overlap nothing and follow the
/// style are left out; power symbols, free text and everything that is not a field stay where they are.
pub fn plan_fields(cx: &Ctx, sheet: Option<&str>) -> Result<FieldPlan, StyleError> {
    let m = draw_34::margins(cx);
    let style = FieldStyle::of(cx)?;
    let defaults = FieldDefaults::of(cx);
    let mut plan = FieldPlan::default();
    for scene in cx.scenes().iter().filter(|s| on_sheet(cx, s, sheet)) {
        plan_scene(scene, m, style, &defaults, &mut plan.symbols);
    }
    Ok(plan)
}

/// With `drawing.fields.hide_defaults = always`, every shown rating of the board of `cx` that holds its default,
/// hidden where it stands; an empty plan otherwise. The placer runs after it, on the design with them gone.
pub fn plan_default_hides(cx: &Ctx, sheet: Option<&str>) -> Result<FieldPlan, StyleError> {
    let defaults = FieldDefaults::of(cx);
    let mut plan = FieldPlan::default();
    if !defaults.always {
        return Ok(plan);
    }
    for scene in cx.scenes().iter().filter(|s| on_sheet(cx, s, sheet)) {
        for sym in scene.geometry.symbols.iter().filter(|s| !s.power) {
            let hidden: Vec<String> = sym
                .fields
                .iter()
                .filter(|f| !f.hidden && !f.text.text.is_empty())
                .filter(|f| defaults.hideable(&sym.reference, f))
                .map(|f| f.name.clone())
                .collect();
            if hidden.is_empty() {
                continue;
            }
            plan.symbols.push(SymbolPlan {
                file: scene.geometry.file.clone(),
                reference: sym.reference.clone(),
                uuid: sym.uuid.clone(),
                side: Some(Side::InPlace),
                moves: Vec::new(),
                hidden,
                left: Vec::new(),
            });
        }
    }
    Ok(plan)
}

/// The conflicts of a set of boxes: how many pairs, and their summed overlap, so fewer pairs win first
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Cost {
    pairs: usize,
    amount: i64,
}

/// One conflict of a placed field
#[derive(Clone, Debug)]
struct Hit {
    /// Index into the placed boxes
    line: usize,
    other: String,
    /// `None` for the sheet edge
    kind: Option<K>,
    close: bool,
}

/// A sheet as the placer changes it: the scene's items, the fields of moved symbols at their new places
struct Board<'s, 'a> {
    scene: &'s Scene<'a>,
    items: Vec<Item>,
    /// Items no longer drawn: fields hidden
    gone: Vec<bool>,
    /// By symbol: its fields are in the way of something and the placer has not been to it yet
    pending: Vec<bool>,
    margins: Margins,
    style: FieldStyle,
    defaults: FieldDefaults,
    /// Inside the drawing frame; `None` when the paper's size is not known
    frame: Option<Rect>,
}

/// A layout to try: where it puts each field it shows (by index into the shown fields), and which it hides
struct Layout {
    side: Side,
    /// How many steps farther out from the body than the gap a column stands; 0 for any other layout
    out: i64,
    lines: Vec<(usize, Pt, HAlign, Rect)>,
    hidden: Vec<usize>,
}

impl Board<'_, '_> {
    /// The conflicts of a symbol's fields at new boxes `placed` (item index and box), against every other item but
    /// the symbol's fields `own`, against each other and the frame. For a column a wire through the gap between
    /// two lines counts too: a person never stacks fields across a wire.
    fn cost(&self, own: &[usize], placed: &[(usize, Rect)], column: bool) -> (Cost, Vec<Hit>) {
        let mut cost = Cost::default();
        let mut hits = Vec::new();
        let placed: Vec<Item> = placed
            .iter()
            .map(|(i, r)| Item {
                rect: *r,
                ..self.items[*i].clone()
            })
            .collect();
        let mut add = |line: usize, b: Option<&Item>, amount: i64, close: bool| {
            cost.pairs += 1;
            cost.amount += amount;
            hits.push(Hit {
                line,
                other: b.map_or_else(|| "the sheet edge".to_owned(), |b| b.name.clone()),
                kind: b.map(|b| b.kind),
                close,
            });
        };
        // Nothing farther than both margins can be in the way
        let m = self.margins;
        let reach = m.part.max(m.power).max(m.text) + m.text;
        for (k, a) in placed.iter().enumerate() {
            let near = a.rect.grown(reach);
            for (j, b) in self.items.iter().enumerate() {
                if self.gone[j]
                    || !near.touches(b.rect)
                    || own.contains(&j)
                    || exempt(self.scene, a, b)
                {
                    continue;
                }
                if let Some(amount) = bare_hit(a, b, false) {
                    add(k, Some(b), amount, false);
                } else if let Some(amount) = close_hit(self.scene, self.margins, a, b) {
                    add(k, Some(b), amount, true);
                }
            }
            for b in &placed[k + 1..] {
                if let Some(amount) = bare_hit(a, b, false) {
                    add(k, Some(b), amount, false);
                }
            }
            if let Some(frame) = self.frame
                && !(frame.contains(a.rect.min) && frame.contains(a.rect.max))
            {
                add(k, None, a.rect.width() * a.rect.height(), false);
            }
        }
        let hull = placed.iter().map(|p| p.rect).reduce(Rect::union);
        if let (true, Some(hull)) = (column && placed.len() > 1, hull) {
            let across = |w: &Item| {
                w.kind.is_wire()
                    && hull.overlaps(w.rect)
                    && !placed.iter().any(|f| f.rect.overlaps(w.rect))
            };
            for w in self.items.iter().filter(|w| across(w)) {
                let i = hull
                    .intersection(w.rect)
                    .expect("the hull overlaps the wire");
                add(0, Some(w), i.width().max(1) * i.height().max(1), false);
            }
        }
        (cost, hits)
    }

    /// What the boxes overlap of the neighbours: the body of another symbol ([`K::Body`]) before a field of one
    /// whose fields are settled ([`K::Field`]); `None` when a column there fits in the gap to them
    fn hits_neighbour(&self, s: usize, field: &Item, rects: &[Rect]) -> Option<K> {
        let neighbour = |j: usize, b: &Item| {
            let other = b.owner.filter(|o| *o != s);
            !self.gone[j]
                && other.is_some_and(|o| match b.kind {
                    K::Body => true,
                    K::Field => !self.pending[o],
                    _ => false,
                })
        };
        let hits = rects.iter().flat_map(|r| {
            let a = Item {
                rect: *r,
                ..field.clone()
            };
            self.items
                .iter()
                .enumerate()
                .filter(move |(j, b)| {
                    neighbour(*j, b)
                        && !exempt(self.scene, &a, b)
                        && bare_hit(&a, b, false).is_some()
                })
                .map(|(_, b)| b.kind)
        });
        hits.min_by_key(|k| *k != K::Body)
    }

    /// The field items of symbol `s`, hidden ones included
    fn own(&self, s: usize) -> Vec<usize> {
        (0..self.items.len())
            .filter(|i| self.items[*i].owner == Some(s) && self.items[*i].kind == K::Field)
            .collect()
    }

    /// Whether the shown fields `own` all stand farther than [`FieldStyle::gap_max`] from the symbol's outline
    fn detached(&self, sym: &DrawnSymbol, own: &[usize]) -> bool {
        let outline = sym.outline();
        own.iter()
            .filter(|i| !self.gone[**i])
            .map(|i| self.items[*i].rect.gap(outline))
            .min()
            .is_some_and(|g| g > self.style.gap_max)
    }

    /// The shown fields of symbol `s` where they are now
    fn shown_now(&self, own: &[usize]) -> Vec<(usize, Rect)> {
        own.iter()
            .filter(|i| !self.gone[**i])
            .map(|i| (*i, self.items[*i].rect))
            .collect()
    }
}

/// How many times the placer goes over a sheet: a symbol whose fields found no room may find it once its
/// neighbours' fields have moved
const PASSES_MAX: usize = 3;

fn plan_scene(
    scene: &Scene,
    margins: Margins,
    style: FieldStyle,
    defaults: &FieldDefaults,
    out: &mut Vec<SymbolPlan>,
) {
    let g = scene.geometry;
    let (w, h) = g.paper_size;
    let frame = (w > 2 * FRAME && h > 2 * FRAME)
        .then(|| Rect::of(Pt::new(FRAME, FRAME), Pt::new(w - FRAME, h - FRAME)));
    let mut board = Board {
        scene,
        items: scene.items.clone(),
        gone: vec![false; scene.items.len()],
        pending: vec![false; g.symbols.len()],
        margins,
        style,
        defaults: defaults.clone(),
        frame,
    };
    for s in 0..g.symbols.len() {
        let own = board.own(s);
        board.pending[s] = board.cost(&own, &board.shown_now(&own), false).0.pairs > 0;
    }
    let mut plans: Vec<Option<SymbolPlan>> = vec![None; g.symbols.len()];
    for _ in 0..PASSES_MAX {
        let mut changed = false;
        for (s, sym) in g.symbols.iter().enumerate() {
            if !sym.power && place_symbol(&mut board, s, sym, &mut plans[s]) {
                changed = true;
            }
            board.pending[s] = false;
        }
        if !changed {
            break;
        }
    }
    // What is left is judged once everything stands where it ends up
    for (s, plan) in plans.iter_mut().enumerate() {
        let Some(p) = plan else { continue };
        let own = board.own(s);
        let shown = board.shown_now(&own);
        let column = p.side.is_some_and(Side::is_column);
        let (_, hits) = board.cost(&own, &shown, column);
        p.left = hits
            .into_iter()
            .map(|h| Remaining {
                field: board.items[shown[h.line].0].name.clone(),
                other: h.other,
                close: h.close,
            })
            .collect();
    }
    out.extend(plans.into_iter().flatten());
}

/// Moves the fields of symbol `s` when they conflict with something, by the first that clears them of: hiding
/// rating fields that only touch a power symbol, a full column, Reference and Value at the ends of a packed
/// two-pin part, a column shortened by its last rating fields; else the full column that conflicts least, when
/// it conflicts less than where they are. Fields clear of everything are placed again when they stray from the
/// style (see [`restyle`]). `plan` keeps the last change. Whether the fields changed.
fn place_symbol(
    board: &mut Board,
    s: usize,
    sym: &DrawnSymbol,
    plan: &mut Option<SymbolPlan>,
) -> bool {
    // Hidden fields are gone: a symbol whose fields were hidden keeps that layout
    if plan
        .as_ref()
        .is_some_and(|p| matches!(p.side, Some(Side::Ends | Side::InPlace)))
    {
        return false;
    }
    let own = board.own(s);
    let column = plan
        .as_ref()
        .is_some_and(|p| p.side.is_some_and(Side::is_column));
    let (now, now_hits) = board.cost(&own, &board.shown_now(&own), column);
    let detached = board.detached(sym, &own);
    let shown: Vec<&DrawnField> = sym
        .fields
        .iter()
        .filter(|f| !f.hidden && !f.text.text.is_empty())
        .collect();
    debug_assert_eq!(shown.len(), own.len(), "one field item per shown field");
    let order = column_order(&shown);
    if now.pairs == 0 && !detached {
        // Fields the placer moved follow the style; others are placed again only when they stray from it
        let restyled = plan
            .is_none()
            .then(|| restyle(board, sym, &shown, &own, &order))
            .flatten();
        return restyled.is_some_and(|l| {
            apply(board, sym, &shown, &own, plan, l);
            true
        });
    }
    // Only a rating that holds its default may be hidden: any other value is a choice the reader must see
    let defaults = board.defaults.clone();
    let hideable = |k: usize| defaults.hideable(&sym.reference, shown[k]);
    let score = |l: &Layout| board.cost(&own, &placed(&own, l), l.side.is_column()).0;
    // A packed horizontal two-pin part always takes its ends: a short column squeezed above a row of them reads
    // as the neighbour's
    let ends_first = (two_pin(sym) == Some(true))
        .then(|| ends_layout(board, s, sym, &shown, &own, &order))
        .flatten();
    let style = board.style;
    let chosen = ends_first
        .or_else(|| in_place(&now_hits, &hideable))
        .or_else(|| {
            // Full columns, then each without one more hideable field, from the column's end
            let droppable: Vec<usize> = order.iter().copied().filter(|k| hideable(*k)).collect();
            let mut fallback: Option<(Cost, Layout)> = None;
            for drop in 0..=droppable.len() {
                let dropped = &droppable[droppable.len() - drop..];
                let kept: Vec<usize> = order
                    .iter()
                    .copied()
                    .filter(|k| !dropped.contains(k))
                    .collect();
                let mut best: Option<(Cost, Layout)> = None;
                for l in columns(style, sym, &shown, &kept, dropped) {
                    let c = score(&l);
                    if best.as_ref().is_none_or(|b| c < b.0) {
                        best = Some((c, l));
                    }
                }
                let best = best?;
                if best.0.pairs == 0 {
                    return Some(best.1);
                }
                if drop == 0 {
                    // A packed part takes its ends when nothing full fits
                    if let Some(ends) = ends_layout(board, s, sym, &shown, &own, &order) {
                        let c = score(&ends);
                        let horizontal = two_pin(sym) == Some(true);
                        // Before and after a horizontal part is its style even with a neighbour close; above and
                        // below a vertical one only where the spot is clear
                        if c.pairs == 0 || (horizontal && c < now) {
                            return Some(ends);
                        }
                    }
                    fallback = Some(best);
                }
            }
            fallback.filter(|f| f.0 < now || detached).map(|f| f.1)
        });
    match chosen {
        // Fields the placer has not been to that stand where it would put them again stay out of the plan
        Some(l) if plan.is_none() && unchanged(&shown, &l) => false,
        Some(l) => {
            apply(board, sym, &shown, &own, plan, l);
            true
        }
        None => {
            // Nothing is clearer: a column off the style still takes it where that is no worse
            let restyled = plan
                .is_none()
                .then(|| restyle(board, sym, &shown, &own, &order))
                .flatten();
            if let Some(l) = restyled {
                apply(board, sym, &shown, &own, plan, l);
                return true;
            }
            plan.get_or_insert_with(|| new_plan(board, sym));
            false
        }
    }
}

/// The boxes a layout gives the symbol's field items `own`
fn placed(own: &[usize], l: &Layout) -> Vec<(usize, Rect)> {
    l.lines.iter().map(|(k, _, _, r)| (own[*k], *r)).collect()
}

/// A plan for a symbol whose fields stay where they are
fn new_plan(board: &Board, sym: &DrawnSymbol) -> SymbolPlan {
    SymbolPlan {
        file: board.scene.geometry.file.clone(),
        reference: sym.reference.clone(),
        uuid: sym.uuid.clone(),
        side: None,
        moves: Vec::new(),
        hidden: Vec::new(),
        left: Vec::new(),
    }
}

/// Puts the symbol's fields where layout `l` says, on the board and in its plan
fn apply(
    board: &mut Board,
    sym: &DrawnSymbol,
    shown: &[&DrawnField],
    own: &[usize],
    plan: &mut Option<SymbolPlan>,
    l: Layout,
) {
    for (i, r) in placed(own, &l) {
        board.items[i].rect = r;
    }
    for k in &l.hidden {
        board.gone[own[*k]] = true;
    }
    let p = plan.get_or_insert_with(|| new_plan(board, sym));
    p.side = Some(l.side);
    p.hidden = l.hidden.iter().map(|k| shown[*k].name.clone()).collect();
    p.moves = l
        .lines
        .iter()
        .map(|(k, at, halign, rect)| FieldMove {
            field: shown[*k].name.clone(),
            at: *at,
            halign: *halign,
            rect: *rect,
        })
        .collect();
}

/// For fields that stand in a column, or at a packed part's ends, off the style: the same layout in style, where
/// it conflicts no more than the fields do now, judged the same way (a wire through a column counts). A one- or
/// two-pin part's column takes the first of its [`sides`] that fits next to the body; any other keeps its side when
/// it fits there, else takes the first side where it does; then a step or more farther out. `None` when the fields
/// follow the style, stand some other way, or nothing in style is as clear.
fn restyle(
    board: &Board,
    sym: &DrawnSymbol,
    shown: &[&DrawnField],
    own: &[usize],
    order: &[usize],
) -> Option<Layout> {
    let now = shape(sym, shown)?;
    let style = board.style;
    let cost = board.cost(own, &board.shown_now(own), now.is_column()).0;
    let clear = |l: &Layout| board.cost(own, &placed(own, l), l.side.is_column()).0 <= cost;
    let same = |l: &Layout| unchanged(shown, l);
    if now == Side::Ends {
        let l = ends_lines(style, &board.defaults, sym, shown)?;
        return (!same(&l) && clear(&l)).then_some(l);
    }
    let mut all = columns(style, sym, shown, order, &[]);
    // In style means centred on its side as well: the first column of a side is the centred one, next to the body
    if all
        .iter()
        .find(|l| l.out == 0 && l.side == now)
        .is_some_and(same)
    {
        return None;
    }
    if sides_fixed(sym) {
        all.sort_by_key(|l| l.out);
    } else {
        all.sort_by_key(|l| (l.out, l.side != now));
    }
    let l = all.into_iter().find(clear)?;
    (!same(&l)).then_some(l)
}

/// Whether layout `l` leaves the fields `shown` where they stand, as drawn in the file
fn unchanged(shown: &[&DrawnField], l: &Layout) -> bool {
    l.hidden.is_empty()
        && l.lines.iter().all(|(k, at, halign, _)| {
            let t = &shown[*k].text;
            t.at == *at && t.halign == *halign && !t.vertical && t.valign == VAlign::Center
        })
}

/// How the shown fields stand when they make a layout like the placer's: a two-pin part's ends (Reference before
/// and Value after a horizontal part on one line, centred above and below a vertical one) or a column: two or more
/// upright lines on one anchor line with one justification, clear of the body on one side, or a part's one field.
/// Other fields of a column's part may stand anywhere: the placer puts them into the column.
fn shape(sym: &DrawnSymbol, shown: &[&DrawnField]) -> Option<Side> {
    let body = sym.outline();
    if let (Some(horizontal), [a, b]) = (two_pin(sym), shown) {
        let rv = match (a.name.as_str(), b.name.as_str()) {
            ("Reference", "Value") => Some((a, b)),
            ("Value", "Reference") => Some((b, a)),
            _ => None,
        };
        let ends = rv.is_some_and(|(r, v)| {
            if horizontal {
                let line = sym.body.unwrap_or(body);
                r.text.halign == HAlign::Right
                    && v.text.halign == HAlign::Left
                    && r.text.at.y == v.text.at.y
                    && r.text.bbox.max.x <= line.min.x
                    && v.text.bbox.min.x >= line.max.x
            } else {
                r.text.halign == HAlign::Center
                    && v.text.halign == HAlign::Center
                    && r.text.bbox.max.y <= body.min.y
                    && v.text.bbox.min.y >= body.max.y
            }
        });
        if ends {
            return Some(Side::Ends);
        }
    }
    let anchor = |f: &DrawnField| (f.text.halign, f.text.at.x);
    let line = shown
        .iter()
        .filter(|f| !f.text.vertical && f.text.valign == VAlign::Center)
        .max_by_key(|f| shown.iter().filter(|g| anchor(g) == anchor(f)).count())?;
    let column: Vec<Rect> = shown
        .iter()
        .filter(|f| !f.text.vertical && anchor(f) == anchor(line))
        .map(|f| f.text.bbox)
        .collect();
    if column.len() < shown.len().min(2) {
        return None;
    }
    let hull = column.into_iter().reduce(Rect::union)?;
    // The side it stands farthest out on, a corner of it over the body's box at most
    if hull.contains(body.center()) {
        return None;
    }
    [
        (hull.min.x - body.max.x, Side::Right),
        (body.min.x - hull.max.x, Side::Left),
        (body.min.y - hull.max.y, Side::Above),
        (hull.min.y - body.max.y, Side::Below),
    ]
    .into_iter()
    .max_by_key(|(gap, _)| *gap)
    .map(|(_, side)| side)
}

/// When hideable rating fields touch a power symbol and nothing else overlaps the fields (closeness alone is no
/// reason to move a column that fits): those fields hidden, the rest left where it is
fn in_place(hits: &[Hit], hideable: &dyn Fn(usize) -> bool) -> Option<Layout> {
    let on_power = |h: &Hit| h.kind == Some(K::Power) && hideable(h.line);
    if !hits.iter().any(on_power) || !hits.iter().all(|h| on_power(h) || h.close) {
        return None;
    }
    let mut hidden: Vec<usize> = hits
        .iter()
        .filter(|h| on_power(h))
        .map(|h| h.line)
        .collect();
    hidden.sort_unstable();
    hidden.dedup();
    Some(Layout {
        side: Side::InPlace,
        out: 0,
        lines: Vec::new(),
        hidden,
    })
}

/// Every column of the fields `kept` (by index into `shown`, in column order), the fields `dropped` hidden
fn columns(
    style: FieldStyle,
    sym: &DrawnSymbol,
    shown: &[&DrawnField],
    kept: &[usize],
    dropped: &[usize],
) -> Vec<Layout> {
    let fields: Vec<&DrawnField> = kept.iter().map(|k| shown[*k]).collect();
    candidates(style, sym, &fields)
        .into_iter()
        .map(|(side, out, places)| Layout {
            side,
            out,
            lines: kept
                .iter()
                .zip(places)
                .map(|(k, (at, halign, rect))| (*k, at, halign, rect))
                .collect(),
            hidden: dropped.to_vec(),
        })
        .collect()
}

/// Reference prefixes of the two-pin parts that get their fields at their ends when packed: resistors,
/// capacitors, diodes and LEDs, inductors, ferrite beads
const TWO_PIN_PREFIXES: &[&str] = &["R", "C", "D", "LED", "LD", "L", "FB"];

/// Whether the symbol is a two-pin part, and lying horizontally
fn two_pin(sym: &DrawnSymbol) -> Option<bool> {
    let prefix: String = sym
        .reference
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect();
    if !TWO_PIN_PREFIXES.contains(&prefix.as_str()) {
        return None;
    }
    let pins: Vec<_> = sym.pins.iter().filter(|p| !p.hidden).collect();
    let [a, b] = pins.as_slice() else { return None };
    let (ha, hb) = (a.outward.is_horizontal(), b.outward.is_horizontal());
    (ha == hb && a.outward != b.outward).then_some(ha)
}

/// For a two-pin part packed against its neighbours (the full column centred on a side across its axis overlaps
/// another part's body, or both such columns overlap a body or settled fields): its [`ends_lines`]. `None` for any other symbol,
/// or one without Reference and Value shown.
fn ends_layout(
    board: &Board,
    s: usize,
    sym: &DrawnSymbol,
    shown: &[&DrawnField],
    own: &[usize],
    order: &[usize],
) -> Option<Layout> {
    let horizontal = two_pin(sym)?;
    let reference = shown.iter().position(|f| f.name == "Reference")?;
    let across = if horizontal {
        [Side::Above, Side::Below]
    } else {
        [Side::Right, Side::Left]
    };
    let field = &board.items[own[reference]];
    let all = columns(board.style, sym, shown, order, &[]);
    let hits = across.map(|side| {
        // The first column of each side is the one centred on the body
        all.iter().find(|l| l.side == side).and_then(|l| {
            let rects: Vec<Rect> = l.lines.iter().map(|line| line.3).collect();
            board.hits_neighbour(s, field, &rects)
        })
    });
    // Another part's body across it, or something settled on both sides
    let packed = hits.contains(&Some(K::Body)) || hits.iter().all(Option::is_some);
    if !packed {
        return None;
    }
    ends_lines(board.style, &board.defaults, sym, shown)
}

/// Reference and Value of a two-pin part at the ends of its body, upright. A horizontal part has them before and
/// after the body on the line above its axis, so they sit beside its own wire; a vertical one has them centred
/// above and below its pins. Every other field hidden. `None` for any other symbol, one without Reference and
/// Value shown, or one with another field shown that holds no default ([`FieldDefaults::hideable`]).
fn ends_lines(
    style: FieldStyle,
    defaults: &FieldDefaults,
    sym: &DrawnSymbol,
    shown: &[&DrawnField],
) -> Option<Layout> {
    let horizontal = two_pin(sym)?;
    let reference = shown.iter().position(|f| f.name == "Reference")?;
    let value = shown.iter().position(|f| f.name == "Value")?;
    let others_go = (0..shown.len())
        .filter(|k| *k != reference && *k != value)
        .all(|k| defaults.hideable(&sym.reference, shown[k]));
    if !others_go {
        return None;
    }
    let line = |k: usize, at: Pt, halign: HAlign| {
        let rect = text_rect(at, false, shown[k].text.extent(), halign, VAlign::Center);
        (k, at, halign, rect)
    };
    let gap = style.body_gap;
    let lines = if horizontal {
        let body = sym.body.unwrap_or_else(|| sym.outline());
        let y = style.nearest(body.center().y - style.ends_raise);
        vec![
            line(
                reference,
                Pt::new(style.down(body.min.x - gap), y),
                HAlign::Right,
            ),
            line(value, Pt::new(style.up(body.max.x + gap), y), HAlign::Left),
        ]
    } else {
        let outline = sym.outline();
        let x = outline.center().x;
        let half = |k: usize| shown[k].text.extent().1 / 2;
        vec![
            line(
                reference,
                Pt::new(x, style.down(outline.min.y - gap - half(reference))),
                HAlign::Center,
            ),
            line(
                value,
                Pt::new(x, style.up(outline.max.y + gap + half(value))),
                HAlign::Center,
            ),
        ]
    };
    let hidden = (0..shown.len())
        .filter(|k| *k != reference && *k != value)
        .collect();
    Some(Layout {
        side: Side::Ends,
        out: 0,
        lines,
        hidden,
    })
}

/// The fields in column order, by index into `shown`: Reference, Value, then the rest top to bottom as they
/// stand now
fn column_order(shown: &[&DrawnField]) -> Vec<usize> {
    let rank = |f: &DrawnField| match f.name.as_str() {
        "Reference" => 0,
        "Value" => 1,
        _ => 2,
    };
    let mut order: Vec<usize> = (0..shown.len()).collect();
    order.sort_by_key(|k| {
        let at = shown[*k].text.bbox.min;
        (rank(shown[*k]), at.y, at.x)
    });
    order
}

/// The sides a column tries, best first: a horizontal two-pin part's above and below, off the line its wires run
/// along; every other part's right, left, above, below
fn sides(sym: &DrawnSymbol) -> [Side; 4] {
    if two_pin(sym) == Some(true) {
        [Side::Above, Side::Below, Side::Right, Side::Left]
    } else {
        [Side::Right, Side::Left, Side::Above, Side::Below]
    }
}

/// Whether the symbol's fields belong on the first of its [`sides`] that fits, whatever side they stand on now:
/// a two-pin part, or a one-pin part such as a test point
fn sides_fixed(sym: &DrawnSymbol) -> bool {
    two_pin(sym).is_some() || sym.pins.iter().filter(|p| !p.hidden).count() == 1
}

/// A column's anchors, alignments and boxes, line by line
type Column = Vec<(Pt, HAlign, Rect)>;

/// Every column to try, best first, with how many steps out it stands: centred on each of the [`sides`], then slid
/// along each side, then the same a step or more farther out. A one- or two-pin part tries every slide on a side
/// before the next side. Lines are [`FieldStyle::pitch`]
/// apart whatever the text's size. A column above or below is centred on the body itself, off the step grid when
/// the body's centre is.
fn candidates(
    style: FieldStyle,
    sym: &DrawnSymbol,
    fields: &[&DrawnField],
) -> Vec<(Side, i64, Column)> {
    let body = sym.outline();
    let extents: Vec<(i64, i64)> = fields.iter().map(|f| f.text.extent()).collect();
    let tallest = extents.iter().map(|e| e.1).max().unwrap_or(style.step);
    let (pitch, gap) = (style.pitch, style.body_gap);
    let n = fields.len() as i64;
    let middle = body.center();
    let top_centred = style.nearest(middle.y - (n - 1) * pitch / 2);
    let column = |side: Side, slide: i64, out: i64| -> Column {
        let (x0, y0, halign) = match side {
            Side::Right => (
                style.up(body.max.x + gap) + out,
                top_centred + slide,
                HAlign::Left,
            ),
            Side::Left => (
                style.down(body.min.x - gap) - out,
                top_centred + slide,
                HAlign::Right,
            ),
            Side::Above => {
                let last = style.down(body.min.y - gap - tallest / 2) - out;
                (middle.x + slide, last - (n - 1) * pitch, HAlign::Center)
            }
            Side::Ends | Side::InPlace => unreachable!("a column stands on one of the four sides"),
            Side::Below => {
                let first = style.up(body.max.y + gap + tallest / 2) + out;
                (middle.x + slide, first, HAlign::Center)
            }
        };
        extents
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let at = Pt::new(x0, y0 + i as i64 * pitch);
                (at, halign, text_rect(at, false, *e, halign, VAlign::Center))
            })
            .collect()
    };
    let step = style.step;
    let slides: Vec<i64> = std::iter::once(0)
        .chain((1..=SLIDE_REACH / step).flat_map(|k| [k * step, -k * step]))
        .collect();
    let mut all = Vec::new();
    for o in 0..=OUT_REACH / step {
        if sides_fixed(sym) {
            for s in sides(sym) {
                for slide in &slides {
                    all.push((s, o, column(s, *slide, o * step)));
                }
            }
        } else {
            for slide in &slides {
                for s in sides(sym) {
                    all.push((s, o, column(s, *slide, o * step)));
                }
            }
        }
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding() {
        let style = FieldStyle {
            pitch: 2_000_000,
            body_gap: 635_000,
            step: 250_000,
            gap_max: 2_000_000,
            ends_raise: 1_270_000,
        };
        assert_eq!(style.up(250_001), 500_000);
        assert_eq!(style.up(-200_000), 0);
        assert_eq!(style.down(-1), -250_000);
        assert_eq!(style.nearest(370_000), 250_000);
        assert_eq!(style.nearest(375_000), 500_000);
    }
}
