//! The canvas' modes driven as a person would: Move, Scale, Measure PDF and Measure footprint, each shown and each
//! left by Esc, the Move key or its own key again; the measuring rulers snap (the drawing's lines, the pads) and
//! put their lengths into the review note.

use super::tests::{datasheet, footprint, harness};
use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

type H = Harness<'static, FootprintReview>;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("berc-fp-mode-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The footprint and the datasheet open, the drawing at 0.02 mm per point, 2 screen points per page point
fn open(name: &'static str, dir: &Path) -> H {
    let pdf = datasheet(dir);
    let mut h = harness(name);
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &footprint());
    h.state_mut().open_datasheet(&ctx, &pdf);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    let st = h.state_mut();
    st.scale = Some(Scale {
        mm_per_point: 0.02,
        page: st.page,
        mm: 2.0,
        note: None,
    });
    st.zoom = 2.0;
    st.pan = [0.0, 0.0];
    st.place.origin = [150.0, 300.0];
    h.run();
    h
}

fn screen(h: &H, p: [f32; 2]) -> egui::Pos2 {
    let st = h.state();
    st.to_screen(st.rt.canvas, p)
}

fn over_canvas(h: &mut H) {
    h.hover_at(h.state().rt.canvas.center());
    h.run();
}

fn key(h: &mut H, k: egui::Key) {
    over_canvas(h);
    h.key_press(k);
    h.run();
}

/// A primary drag on the canvas from `a` to `b` (screen)
fn drag(h: &mut H, a: egui::Pos2, b: egui::Pos2) {
    h.hover_at(a);
    h.run();
    h.drag_at(a);
    h.run();
    for i in 1..=5 {
        h.hover_at(a + (b - a) * (i as f32 / 5.0));
        h.run();
    }
    h.drop_at(b);
    h.run();
}

/// Each mode is left back to Move by Esc, by the Move key and by its own key again, and the mouse then moves the
/// footprint again; the mode is on the canvas' bar and in the help line at every step
#[test]
fn every_mode_has_a_way_back_to_move() {
    let dir = scratch("back");
    let mut h = open("fp mode back", &dir);
    let shown = |h: &H, t: Tool| {
        let help = h.state().help_line();
        assert!(help.starts_with(&format!("Mode {}:", t.name())), "{help}");
        // The bar's button of the mode on is the selected one
        use egui_kittest::kittest::NodeT;
        for m in [
            Tool::Move,
            Tool::Scale,
            Tool::MeasurePdf,
            Tool::MeasureFootprint,
        ] {
            let k = h.state().keys();
            let key = match m {
                Tool::Move => k.move_tool,
                Tool::Scale => k.ruler,
                Tool::MeasurePdf => k.measure_pdf,
                Tool::MeasureFootprint => k.measure_footprint,
            };
            let label = format!("{} · {}", m.name(), key_name(key));
            let node = h.get_by_label(&label);
            let on = node.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True);
            assert_eq!(on, m == t, "{} while in {}", m.name(), t.name());
        }
    };
    shown(&h, Tool::Move);
    // R: the ruler's first end, Scale; Esc: back to Move, the ruler step over
    key(&mut h, egui::Key::R);
    assert_eq!(h.state().tool, Tool::Scale);
    shown(&h, Tool::Scale);
    key(&mut h, egui::Key::Escape);
    assert_eq!(h.state().tool, Tool::Move);
    assert!(!h.state().rt.ruler_placing);
    shown(&h, Tool::Move);
    // The mouse moves the footprint again
    let origin = h.state().place.origin;
    let c = h.state().rt.canvas.center();
    drag(&mut h, c, c + egui::vec2(40.0, 0.0));
    let moved = h.state().place.origin;
    assert!(
        (moved[0] - origin[0] - 20.0).abs() < 0.5,
        "{origin:?} → {moved:?}"
    );
    // Each measuring mode: its key in, M out, its key in, its key again out, its key in, Esc out
    for (k, t) in [
        (egui::Key::E, Tool::MeasurePdf),
        (egui::Key::G, Tool::MeasureFootprint),
    ] {
        key(&mut h, k);
        assert_eq!(h.state().tool, t);
        shown(&h, t);
        key(&mut h, egui::Key::M);
        assert_eq!(h.state().tool, Tool::Move, "M leaves {}", t.name());
        key(&mut h, k);
        key(&mut h, k);
        assert_eq!(
            h.state().tool,
            Tool::Move,
            "its key again leaves {}",
            t.name()
        );
        key(&mut h, k);
        key(&mut h, egui::Key::Escape);
        assert_eq!(h.state().tool, Tool::Move, "Esc leaves {}", t.name());
    }
    // The bar's buttons: into a mode, and the same button again back to Move
    h.get_by_label("Measure footprint · G").click();
    h.run();
    assert_eq!(h.state().tool, Tool::MeasureFootprint);
    h.get_by_label("Measure footprint · G").click();
    h.run();
    assert_eq!(h.state().tool, Tool::Move);
    h.get_by_label("Scale · R").click();
    h.run();
    assert_eq!(h.state().tool, Tool::Scale);
    h.get_by_label("Move · M").click();
    h.run();
    assert_eq!(h.state().tool, Tool::Move);
    let help = h.state().help_line();
    for k in [
        "Enter write a review note",
        "M move",
        "R ruler",
        "E measure the PDF",
        "G measure the footprint",
        "M or Esc: back to Move",
    ] {
        assert!(help.contains(k), "{k}: {help}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Measure footprint: the ends snap to the pads' centres and to the middles of a rectangular pad's sides, never
/// to the drawing; the length goes into the note as M1, an end moved changes M1, a new ruler is M2. The scale's
/// ruler waits meanwhile and is back in Move.
#[test]
fn measure_the_footprint_on_its_pads() {
    let dir = scratch("fp");
    let mut h = open("fp mode measure fp", &dir);
    let scale_ruler = Some([[100.0, 200.0], [200.0, 200.0]]);
    h.state_mut().ruler = scale_ruler;
    key(&mut h, egui::Key::G);
    assert_eq!(h.state().ruler, None, "the measuring ruler, none yet");
    // Pad 2 at (0, -1.5) mm, pad 3 at (2, -1.5) turned 90°, 1.2 mm square
    let page = |h: &H, p: [f64; 2]| h.state().fp_to_page(p);
    let (p2, p3) = (page(&h, [0.0, -1.5]), page(&h, [2.0, -1.5]));
    let near = |h: &H, p: [f32; 2]| screen(h, p) + egui::vec2(4.0, -3.0);
    let (a, b) = (near(&h, p2), near(&h, p3));
    h.hover_at(a);
    h.run();
    let snap = h
        .state()
        .rt
        .snap
        .clone()
        .expect("the pad centre shows before a drag");
    assert_eq!((snap.at, snap.kind), (p2, SnapKind::PadCentre));
    drag(&mut h, a, b);
    let close = |got: [f32; 2], want: [f32; 2]| {
        (got[0] - want[0]).abs() < 1e-3 && (got[1] - want[1]).abs() < 1e-3
    };
    let [ra, rb] = h.state().ruler.unwrap();
    assert!(close(ra, p2) && close(rb, p3), "{ra:?} {rb:?}");
    assert_eq!(h.state().rt.note.trim(), "M1 = 2.000 mm");
    // The far end onto pad 3's side, 0.6 mm on: M1 changes in the note
    let side = page(&h, [2.6, -1.5]);
    let (from, to) = (screen(&h, rb), near(&h, side));
    drag(&mut h, from, to);
    let [_, rb] = h.state().ruler.unwrap();
    assert!(close(rb, side), "{rb:?} want {side:?}");
    assert_eq!(h.state().rt.note.trim(), "M1 = 2.600 mm");
    // A new ruler: M2
    h.state_mut().rt.note.push_str("pitch ");
    let (from, to) = (near(&h, p3), near(&h, p2));
    drag(&mut h, from, to);
    assert_eq!(
        h.state().rt.note.trim(),
        "M1 = 2.600 mm pitch M2 = 2.000 mm"
    );
    // Back in Move: the scale's ruler, the measuring one kept for the next time
    key(&mut h, egui::Key::M);
    assert_eq!(h.state().ruler, scale_ruler);
    key(&mut h, egui::Key::G);
    assert!(
        h.state()
            .ruler
            .is_some_and(|[a, b]| close(a, p3) && close(b, p2))
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Measure PDF: the ruler snaps to the drawing's lines as the scale's does, its length goes into the note, and S
/// and D set no scale while measuring
#[test]
fn measure_the_pdf_on_its_lines() {
    let dir = scratch("pdf");
    let mut h = open("fp mode measure pdf", &dir);
    key(&mut h, egui::Key::E);
    assert_eq!(h.state().tool, Tool::MeasurePdf);
    let (a, b) = ([100.0, 200.0], [200.0, 200.0]);
    let (from, to) = (
        screen(&h, [a[0] + 3.0, a[1] - 2.0]),
        screen(&h, [b[0] - 3.0, b[1] + 2.0]),
    );
    drag(&mut h, from, to);
    assert_eq!(h.state().ruler, Some([a, b]));
    assert_eq!(h.state().rt.note.trim(), "M1 = 2.000 mm");
    let scale = h.state().scale.clone();
    key(&mut h, egui::Key::D);
    key(&mut h, egui::Key::S);
    assert!(!h.state().rt.length_box, "no length box while measuring");
    assert_eq!(h.state().scale, scale);
    assert_eq!(h.state().tool, Tool::MeasurePdf);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A tab saved up to 0.4.36 in the ruler opens in the Scale mode
#[test]
fn a_saved_ruler_tool_is_the_scale_mode() {
    let t: Tool = serde_json::from_str("\"Ruler\"").unwrap();
    assert_eq!(t, Tool::Scale);
}

/// A review note without leaving the tab: a measurement of the footprint goes into the note, the note key gives
/// the field the keyboard, Enter appends the note to the notes file beside the library (in the test's own
/// folder), with the symbol, the footprint, the datasheet as the library names it and its page; the field is then
/// empty and the next note's measurements count from M1 again
#[test]
fn a_note_with_a_measurement_into_the_notes_file() {
    let dir = scratch("note");
    let pdf = datasheet(&dir);
    let lib = super::tests::symbol_library(&dir, &pdf);
    let mut h = harness("fp mode note");
    let asked: Arc<std::sync::Mutex<Vec<PathBuf>>> = Arc::default();
    let a = asked.clone();
    h.state_mut().rt.namer = Some(Arc::new(move |p: &Path| {
        a.lock().unwrap().push(p.to_path_buf());
        Ok(Some("`tpm d TEST-DS rev2`".into()))
    }));
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &lib);
    h.run();
    h.get_by_label("Rev:Mixed").click();
    h.run();
    h.run();
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    let file = dir.join("footprint-review-notes.md");
    assert_eq!(h.state().notes_file(), Some(file.clone()));
    assert_eq!(
        h.state().note_blocker().as_deref(),
        Some("write the note first")
    );
    {
        let st = h.state_mut();
        st.scale = Some(Scale {
            mm_per_point: 0.02,
            page: st.page,
            mm: 2.0,
            note: None,
        });
        st.zoom = 2.0;
        st.pan = [0.0, 0.0];
        st.place = Placement {
            origin: [150.0, 300.0],
            ..Default::default()
        };
    }
    h.run();
    key(&mut h, egui::Key::G);
    let (p2, p3) = (
        h.state().fp_to_page([0.0, -1.5]),
        h.state().fp_to_page([2.0, -1.5]),
    );
    let (from, to) = (
        screen(&h, p2) + egui::vec2(3.0, 3.0),
        screen(&h, p3) + egui::vec2(-3.0, 3.0),
    );
    drag(&mut h, from, to);
    assert_eq!(h.state().rt.note, "M1 = 2.000 mm ");
    // Enter over the canvas: the note field takes the keyboard, the text goes after the measurement
    key(&mut h, egui::Key::Enter);
    h.run();
    h.event(egui::Event::Text("pitch as drawn".into()));
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(h.state().rt.note_written, Some(Ok(file.clone())));
    let text = std::fs::read_to_string(&file).unwrap();
    let page = h.state().page + 1;
    assert!(
        text.starts_with("# Footprint review notes\n\n## Rev:Mixed · 20"),
        "{text}"
    );
    for line in [
        "- Symbol: `Rev:Mixed`\n".to_string(),
        "- Footprint: `Test:Mixed_Pads`\n".to_string(),
        format!("- Datasheet: `tpm d TEST-DS rev2`, page {page}\n"),
        "\nM1 = 2.000 mm pitch as drawn\n".to_string(),
    ] {
        assert!(text.contains(&line), "{line:?} in {text}");
    }
    assert_eq!(*asked.lock().unwrap(), vec![pdf.clone()]);
    assert_eq!(h.state().rt.note, "");
    h.get_by_label("added to footprint-review-notes.md");
    // The next note: a new ruler, from pad 4 at (0, 1) mm to pad 2, is M1 again, and the note goes under the first
    let p4 = h.state().fp_to_page([0.0, 1.0]);
    let (from, to) = (
        screen(&h, p4) + egui::vec2(3.0, 3.0),
        screen(&h, p2) + egui::vec2(3.0, 3.0),
    );
    drag(&mut h, from, to);
    assert_eq!(h.state().rt.note, "M1 = 2.500 mm ");
    h.get_by_label("Add (Enter)").click();
    h.run();
    let text = std::fs::read_to_string(&file).unwrap();
    assert_eq!(text.matches("## Rev:Mixed").count(), 2, "{text}");
    assert_eq!(
        text.matches("# Footprint review notes").count(),
        1,
        "{text}"
    );
    assert_eq!(
        asked.lock().unwrap().len(),
        1,
        "the library asked once per datasheet"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
