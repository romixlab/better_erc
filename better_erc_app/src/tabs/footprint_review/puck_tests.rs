//! The SpaceMouse in the review driven as a person would, through a fake spacenavd on a socket of the test's own
//! (never the host's spacenavd or settings file): the puck pans the page and moves the footprint at the same
//! screen speed at every zoom and zooms about the pointer, a tap and a held button's ring do the canvas' actions,
//! every slot names its action and key, and the calibrate step stops both while it measures.
//!
//! Time is the harness's own (0.05 s a frame). The client reads the socket on its own thread, so before a frame
//! the test waits until the client holds what was sent (its axes, its held buttons): the frame then sees all of
//! it at once, whatever the host's load.

#![cfg(unix)]

use super::super::tests::{datasheet, footprint};
use super::*;
use egui::vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use std::time::{Duration, Instant};
use ve_spacemouse::{FakeSpacenavd, Mapping, PieStyle, SpaceMouse};

const GUARD: Duration = Duration::from_secs(10);
const DT: f32 = 0.05;

/// A folder and a socket path of this test's own
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("berc-fp-puck-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A review tab whose SpaceMouse is the fake on `socket`, the fake connected
fn harness(socket: &Path, fake: &FakeSpacenavd) -> Harness<'static, FootprintReview> {
    let settings = FootprintReviewSettings {
        font_dirs: Vec::new(),
        library_search: Vec::new(),
        library_name: Vec::new(),
        ..Default::default()
    };
    let mut review = FootprintReview::with(settings, Arc::new(|_: &str| {}));
    review.rt.puck.socket = Some(socket.to_path_buf());
    let mut cx = Context::new();
    let mut h = Harness::builder()
        .with_size(vec2(1200.0, 900.0))
        .with_step_dt(DT)
        .build_ui_state(
            move |ui, state: &mut FootprintReview| state.ui(ui, &mut cx, Id::new("fp puck")),
            review,
        );
    h.step();
    assert!(fake.wait_for_clients(1, GUARD), "the tab never connected");
    h
}

/// The footprint alone and the test datasheet open
fn open(h: &mut Harness<'static, FootprintReview>, dir: &Path) {
    let ctx = h.ctx.clone();
    let pdf = datasheet(dir);
    h.state_mut().open_path(&ctx, &footprint());
    h.state_mut().open_datasheet(&ctx, &pdf);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.step();
    h.step();
    assert!(!h.state().rt.items.is_empty());
}

fn mouse<'a>(h: &'a Harness<'static, FootprintReview>) -> &'a SpaceMouse {
    h.state().rt.puck.mouse.as_ref().expect("the client runs")
}

/// Waits (real time, for the client's thread) until `cond` holds of the client
fn until(h: &Harness<'static, FootprintReview>, what: &str, cond: impl Fn(&SpaceMouse) -> bool) {
    let deadline = Instant::now() + GUARD;
    while !cond(mouse(h)) {
        assert!(Instant::now() < deadline, "never happened: {what}");
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// The puck held at `axes`, then one frame
fn push(h: &mut Harness<'static, FootprintReview>, fake: &FakeSpacenavd, axes: [i32; 6]) {
    fake.send(Event::motion(axes));
    until(h, "the motion arrived", |m| m.axes() == axes);
    h.step();
}

/// A button down or up, then one frame
fn button(h: &mut Harness<'static, FootprintReview>, fake: &FakeSpacenavd, n: i32, down: bool) {
    fake.send(Event::button(n, down));
    until(h, "the button arrived", |m| m.held().contains(&n) == down);
    h.step();
}

/// The screen points a full push moves the content in a frame, by the default mapping
fn full_push_step() -> f32 {
    Mapping::default().speed.pan * DT
}

#[test]
fn the_puck_pans_the_page_and_moves_the_footprint_at_one_screen_speed() {
    let dir = scratch("pan");
    let socket = dir.join("spnav.sock");
    let fake = FakeSpacenavd::start(&socket).unwrap();
    let mut h = harness(&socket, &fake);
    open(&mut h, &dir);
    let full = Mapping::default().full_scale as i32;
    let step = full_push_step();
    // The page follows the hand: slide right, the page moves right by the same screen points at every zoom
    for zoom in [1.0, 8.0, 0.25] {
        h.state_mut().zoom = zoom;
        let before = h.state().pan;
        push(&mut h, &fake, [full, 0, 0, 0, 0, 0]);
        let after = h.state().pan;
        assert!(
            (after[0] - before[0] - step).abs() < 0.01 && after[1] == before[1],
            "zoom {zoom}: {before:?} → {after:?}, want +{step}"
        );
    }
    // Lift up: the page moves up
    let before = h.state().pan;
    push(&mut h, &fake, [0, full, 0, 0, 0, 0]);
    assert!((h.state().pan[1] - (before[1] - step)).abs() < 0.01);
    // At rest nothing moves
    push(&mut h, &fake, [0; 6]);
    let rest = h.state().pan;
    h.step();
    assert_eq!(h.state().pan, rest);
    // The footprint instead: the same screen speed, in page points by the zoom; the page stays
    h.state_mut().puck = PuckMoves::Footprint;
    for zoom in [1.0, 4.0] {
        h.state_mut().zoom = zoom;
        let (pan, origin) = (h.state().pan, h.state().place.origin);
        push(&mut h, &fake, [full, 0, 0, 0, 0, 0]);
        let st = h.state();
        assert_eq!(st.pan, pan);
        let moved = (st.place.origin[0] - origin[0]) * zoom;
        assert!(
            (moved - step).abs() < 0.01,
            "zoom {zoom}: {moved} screen points"
        );
    }
    push(&mut h, &fake, [0; 6]);
    // Pull towards you (raw z, the default's "grow"): the view zooms in about the pointer, the page point under it
    // stays
    let canvas = h.state().rt.canvas;
    let at = canvas.center() + vec2(120.0, -80.0);
    h.hover_at(at);
    h.step();
    let (zoom, under) = (h.state().zoom, h.state().to_page(canvas, at));
    push(&mut h, &fake, [0, 0, full, 0, 0, 0]);
    let st = h.state();
    let want = zoom * (Mapping::default().speed.zoom * DT).exp();
    assert!((st.zoom - want).abs() < 1e-4, "{} want {want}", st.zoom);
    let still = st.to_page(canvas, at);
    assert!(
        (still[0] - under[0]).abs() < 0.01 && (still[1] - under[1]).abs() < 0.01,
        "{under:?} → {still:?}"
    );
    push(&mut h, &fake, [0; 6]);
    // The client first, then the fake (its socket is in the folder)
    drop(h);
    drop(fake);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn taps_and_rings_do_the_canvas_actions() {
    let dir = scratch("pie");
    let socket = dir.join("spnav.sock");
    let fake = FakeSpacenavd::start(&socket).unwrap();
    let mut h = harness(&socket, &fake);
    open(&mut h, &dir);
    let canvas = h.state().rt.canvas;
    let (left, right) = {
        let b = Mapping::default().buttons;
        (b.left, b.right)
    };
    // A tap of the left button lays the ruler's first end at the pointer, a second tap its other end (level, well
    // above the drawing's lines, so neither snaps)
    let a = canvas.center() + vec2(-100.0, -250.0);
    let b = canvas.center() + vec2(100.0, -250.0);
    h.hover_at(a);
    h.step();
    let want_a = h.state().to_page(canvas, a);
    button(&mut h, &fake, left, true);
    button(&mut h, &fake, left, false);
    assert!(
        h.state().rt.ruler_placing,
        "the other end follows the pointer"
    );
    h.hover_at(b);
    h.step();
    let want_b = h.state().to_page(canvas, b);
    button(&mut h, &fake, left, true);
    button(&mut h, &fake, left, false);
    let st = h.state();
    assert!(!st.rt.ruler_placing);
    let [ra, rb] = st.ruler.expect("a ruler");
    for (got, want) in [(ra, want_a), (rb, want_b)] {
        assert!(
            (got[0] - want[0]).abs() < 1.0 && (got[1] - want[1]).abs() < 1.0,
            "{ra:?} {rb:?}, want {want_a:?} {want_b:?}"
        );
    }
    // A tap of the right one: the puck moves the footprint, again the page
    button(&mut h, &fake, right, true);
    button(&mut h, &fake, right, false);
    assert_eq!(h.state().puck, PuckMoves::Footprint);
    button(&mut h, &fake, right, true);
    button(&mut h, &fake, right, false);
    assert_eq!(h.state().puck, PuckMoves::Page);
    // Held: the right ring opens at the pointer; out through its top slice turns the footprint, and while still
    // held, back through the centre and out again turns it once more
    let o = canvas.center();
    h.hover_at(o);
    h.step();
    button(&mut h, &fake, right, true);
    for _ in 0..8 {
        h.step();
    }
    assert!(h.state().rt.puck.pie.is_open(), "the ring opened");
    let outer = PieStyle::default().outer_radius;
    for _ in 0..2 {
        for r in [60.0, 120.0, outer + 20.0] {
            h.hover_at(o - vec2(0.0, r));
            h.step();
        }
        h.hover_at(o);
        h.step();
    }
    assert_eq!(h.state().place.rotation, 180.0);
    button(&mut h, &fake, right, false);
    assert!(!h.state().rt.puck.pie.is_open());
    // Both held: Accept is disabled for a footprint alone (a preview), so crossing its slice does nothing
    button(&mut h, &fake, left, true);
    button(&mut h, &fake, right, true);
    for _ in 0..8 {
        h.step();
    }
    assert_eq!(
        h.state().rt.puck.pie.ring(),
        Some(ve_spacemouse::Ring::Both)
    );
    h.hover_at(o - vec2(0.0, outer + 20.0));
    h.step();
    assert_eq!(h.state().accept_blocker().as_deref(), Some(PREVIEW_WHY));
    assert!(h.state().rt.confirm.is_none());
    button(&mut h, &fake, left, false);
    button(&mut h, &fake, right, false);
    // The client first, then the fake (its socket is in the folder)
    drop(h);
    drop(fake);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Accept's reason for a footprint alone, shown under the ring while the pointer is on its slice
const PREVIEW_WHY: &str = "a footprint opened alone is a preview: there is no symbol to put the review hash on; pick \
                           its symbol from a library or a board's list";

/// Every slot names its action and its key (from the settings), and a disabled one says why
#[test]
fn every_slot_names_its_action_and_key() {
    let settings = FootprintReviewSettings {
        keys: crate::settings::Keys {
            rotate: "Space".into(),
            blink: "".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let review = FootprintReview::with(settings, Arc::new(|_: &str| {}));
    let menu = review.pie_menu();
    let slots: Vec<_> = [menu.center_left.as_ref(), menu.center_right.as_ref()]
        .into_iter()
        .flatten()
        .chain(
            [&menu.ring_left, &menu.ring_right, &menu.ring_both]
                .into_iter()
                .flat_map(|r| r.iter().flatten()),
        )
        .collect();
    assert_eq!(slots.len(), 2 + 10 + 5 + 4);
    for s in &slots {
        let (name, key) = s.label.split_once(" · ").expect(&s.label);
        assert!(!name.is_empty() && !key.is_empty(), "{}", s.label);
        assert!(!s.icon.is_empty(), "{}", s.label);
        if let Some(why) = &s.disabled {
            assert!(why.len() > 10, "{}: {why}", s.label);
        }
    }
    let label = |a: Action| {
        slots
            .iter()
            .find(|s| s.action == a)
            .map(|s| s.label.clone())
            .unwrap()
    };
    assert_eq!(label(Action::Rotate), "Turn 90° · Space");
    assert_eq!(label(Action::Blink), "Blink · no key");
    assert_eq!(label(Action::Ruler), "Ruler · R");
    assert_eq!(label(Action::Accept), "Accept · A");
    assert_eq!(label(Action::Mode(Tool::Move)), "Back to Move · M");
    assert_eq!(label(Action::Mode(Tool::MeasurePdf)), "Measure the PDF · E");
    assert_eq!(
        label(Action::Mode(Tool::MeasureFootprint)),
        "Measure the footprint · G"
    );
    // Nothing open: only what the puck moves can be done; the rest says why
    for s in &slots {
        assert_eq!(
            s.disabled.is_none(),
            s.action == Action::Puck,
            "{}",
            s.label
        );
    }
    let why = |a: Action| {
        slots
            .iter()
            .find(|s| s.action == a)
            .and_then(|s| s.disabled.clone())
            .unwrap()
    };
    assert_eq!(
        why(Action::Rotate),
        "No footprint shown: pick a symbol (left)"
    );
    assert_eq!(why(Action::FullWidth), "Open the datasheet first (left)");
    assert_eq!(
        why(Action::Mode(Tool::Move)),
        "In Move already: the mouse moves the footprint"
    );
    assert_eq!(
        why(Action::Accept),
        "pick a symbol first: its footprint and datasheet follow"
    );
}

/// Calibrate from the tab: while its window is open the puck and the buttons are measurements, not navigation or
/// menus; Cancel closes it. The mapping goes into the settings file, the rest of the file kept.
#[test]
fn calibrate_from_the_tab_and_the_mapping_saved() {
    let dir = scratch("calibrate");
    let socket = dir.join("spnav.sock");
    let fake = FakeSpacenavd::start(&socket).unwrap();
    let mut h = harness(&socket, &fake);
    h.get_by_label("SpaceMouse").click();
    h.step();
    h.get_by_label("Calibrate…").click();
    h.step();
    h.step();
    h.get_by_label("Calibrate the SpaceMouse");
    let full = Mapping::default().full_scale as i32;
    let pan = h.state().pan;
    push(&mut h, &fake, [full, 0, 0, 0, 0, 0]);
    push(&mut h, &fake, [0; 6]);
    assert_eq!(h.state().pan, pan, "no navigation while calibrating");
    let b = Mapping::default().buttons.right;
    button(&mut h, &fake, b, true);
    for _ in 0..8 {
        h.step();
    }
    assert!(
        !h.state().rt.puck.pie.is_open(),
        "no menu while calibrating"
    );
    button(&mut h, &fake, b, false);
    h.get_by_label("Cancel").click();
    h.step();
    assert!(h.state().rt.puck.calibration.is_none());
    // The mapping into the settings file: [spacemouse], the review's settings left as they are
    let file = dir.join("config.toml");
    std::fs::write(&file, "[footprint_review]\nblink_hz = 5.0\n").unwrap();
    h.state_mut().rt.settings_file = Some(file.clone());
    h.state_mut().rt.puck.mapping.speed.pan = 3000.0;
    h.state_mut().save_mapping();
    let (saved, error) = Settings::load_from(&file);
    assert_eq!(error, None);
    assert_eq!(saved.spacemouse.speed.pan, 3000.0);
    assert_eq!(saved.spacemouse, h.state().rt.puck.mapping);
    assert_eq!(saved.footprint_review.blink_hz, 5.0);
    // A file that can't be read is left alone
    std::fs::write(&file, "[footprint_review]\nblink_hz = \"fast\"\n").unwrap();
    h.state_mut().save_mapping();
    assert!(
        h.state()
            .rt
            .puck
            .note
            .as_deref()
            .unwrap()
            .starts_with("not saved, the file is left as it is"),
        "{:?}",
        h.state().rt.puck.note
    );
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "[footprint_review]\nblink_hz = \"fast\"\n"
    );
    // The client first, then the fake (its socket is in the folder)
    drop(h);
    drop(fake);
    std::fs::remove_dir_all(&dir).unwrap();
}
