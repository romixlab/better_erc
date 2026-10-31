//! `DRAW` - How the sheet is drawn. The checks read the [`Scene`] of each sheet: item boxes and wire ties, with
//! the settings shared under `drawing.` declared here, named as the table at the top of `docs/checks/draw.md`.

use crate::meta::Param;
use crate::{Ctx, Location};
use erc_analysis::scene::{Item, Scene, mm};
use erc_model::geometry::Rect;
use erc_model::{Pt, Subject};

mod draw_28;
mod draw_29;
mod draw_30;
mod draw_32;
mod draw_34;
mod draw_35;
mod draw_36;
mod draw_40;
mod draw_41;
mod draw_43;
mod draw_44;
mod draw_45;
mod draw_7;
mod draw_9;
mod overlap;
mod section;

const MM: f64 = 1e6;

pub(crate) const CLEARANCE_PART: Param = Param {
    name: "drawing.clearance.part",
    default: 1.27,
    unit: "mm",
    doc: "margin around a symbol's body and its pins",
};

pub(crate) const CLEARANCE_POWER: Param = Param {
    name: "drawing.clearance.power",
    default: 1.27,
    unit: "mm",
    doc: "margin around a power or ground symbol and its name",
};

pub(crate) const CLEARANCE_TEXT: Param = Param {
    name: "drawing.clearance.text",
    default: 0.635,
    unit: "mm",
    doc: "margin around a field, a label, a pin name or number, a note box",
};

pub(crate) const ALIGN_WINDOW: Param = Param {
    name: "drawing.align.window",
    default: 20.32,
    unit: "mm",
    doc: "neighbourhood within which items must align exactly",
};

pub(crate) const ALIGN_SNAP: Param = Param {
    name: "drawing.align.snap",
    default: 5.08,
    unit: "mm",
    doc: "offsets below this (and above zero) are near misses",
};

pub(crate) const PIN_LEAD: Param = Param {
    name: "drawing.pin_lead",
    default: 2.54,
    unit: "mm",
    doc: "straight wire at a pin or power symbol before the first bend",
};

pub(crate) const HEADING_SIZE: Param = Param {
    name: "drawing.heading_size",
    default: 2.54,
    unit: "mm",
    doc: "height of a section heading; larger bold text is a title",
};

pub(crate) const SECTION_REACH: Param = Param {
    name: "drawing.section.reach",
    default: 5.08,
    unit: "mm",
    doc: "how far left of its heading a section's parts may stand",
};

/// A length setting in nanometres
fn nm(cx: &Ctx, meta: &crate::CheckMeta, p: &Param) -> i64 {
    (cx.param(meta, p.name) * MM).round() as i64
}

/// What a finding on a sheet is about: its sheet, or the board when the board's sheets do not name the file
fn sheet_subject(cx: &Ctx, scene: &Scene) -> Subject {
    scene.sheet.map_or(Subject::Board(cx.board_id), |s| {
        Subject::Sheet(cx.board_id, s)
    })
}

/// `R1 Value at x 12.7 y 25.4`
fn at(item: &Item, p: Pt) -> String {
    format!("{} at x {} y {}", item.name, mm(p.x), mm(p.y))
}

/// Where an item is, on the point of interest
fn location(scene: &Scene, item: &Item, p: Pt) -> Location {
    Location {
        file: scene.geometry.file.clone(),
        what: item.name.clone(),
        line: item.line,
        at: Some((p.x as f64 / MM, p.y as f64 / MM)),
    }
}

/// A finding on one item, at a point of it
fn one<'c, 'a>(
    cx: &'c Ctx<'a>,
    meta: &'static crate::CheckMeta,
    scene: &Scene,
    item: &Item,
    p: Pt,
) -> crate::finding::FindingBuilder<'c, 'a> {
    let sheet = sheet_subject(cx, scene);
    cx.finding(meta)
        .subject(sheet)
        .item(sheet, item.name.clone())
        .location(location(scene, item, p))
}

/// A point of the box to name for the item: its centre, or the part of a wire next to `near`
fn near_point(item: &Item, near: Rect) -> Pt {
    if item.kind.is_wire() {
        let c = near.center();
        Pt::new(
            c.x.clamp(item.rect.min.x, item.rect.max.x),
            c.y.clamp(item.rect.min.y, item.rect.max.y),
        )
    } else {
        item.rect.center()
    }
}
