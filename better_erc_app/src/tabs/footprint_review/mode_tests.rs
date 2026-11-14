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

/// A primary click on the canvas at `a` (screen): pressed and let go without moving
fn click(h: &mut H, a: egui::Pos2) {
    h.hover_at(a);
    h.run();
    h.drag_at(a);
    h.run();
    h.drop_at(a);
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
            Tool::Select,
            Tool::MeasureModel,
            Tool::MeasureAcross,
        ] {
            let k = h.state().keys();
            let key = match m {
                Tool::Move => k.move_tool,
                Tool::Scale => k.ruler,
                Tool::MeasurePdf => k.measure_pdf,
                Tool::MeasureFootprint => k.measure_footprint,
                Tool::Select => k.select,
                Tool::MeasureModel => k.measure_model,
                Tool::MeasureAcross => k.measure_across,
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
        (egui::Key::V, Tool::Select),
        (egui::Key::Y, Tool::MeasureAcross),
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
        "Y measure across",
        "M or Esc: back to Move",
    ] {
        assert!(help.contains(k), "{k}: {help}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Measure footprint: the ends snap to the pads' centres and to the middles of a rectangular pad's sides, never
/// to the drawing; nothing goes into the note by itself: the + note button beside the ruler's label adds it as M1,
/// once; an end moved is a new measurement, M2 when added. The scale's ruler waits meanwhile and is back in Move.
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
    assert_eq!(h.state().rt.note, "", "nothing goes in by itself");
    h.get_by_label("+ note").click();
    h.run();
    assert_eq!(h.state().rt.note.trim(), "M1 = 2.000 mm");
    // Added once: the button says so, and the label names it
    h.get_by_label("in note as M1");
    assert!(h.state().measurement_blocker().is_some());
    h.get_by_label("In the note as M1");
    // The far end onto pad 3's side, 0.6 mm on: a new measurement, M1 stays as it was
    let side = page(&h, [2.6, -1.5]);
    let (from, to) = (screen(&h, rb), near(&h, side));
    drag(&mut h, from, to);
    let [_, rb] = h.state().ruler.unwrap();
    assert!(close(rb, side), "{rb:?} want {side:?}");
    assert_eq!(h.state().rt.note.trim(), "M1 = 2.000 mm");
    // A new ruler, added from the Measure section's button: M2
    h.state_mut().rt.note.push_str("pitch ");
    let (from, to) = (near(&h, p3), near(&h, p2));
    drag(&mut h, from, to);
    h.get_by_label("Add to the note as M2").click();
    h.run();
    assert_eq!(
        h.state().rt.note.trim(),
        "M1 = 2.000 mm pitch M2 = 2.000 mm"
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
    assert_eq!(h.state().rt.note, "");
    h.get_by_label("+ note").click();
    h.run();
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
    h.get_by_label("+ note").click();
    h.run();
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
    h.get_by_label("+ note").click();
    h.run();
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

/// A land pattern page for the Select mode: a dimension line, a circle, a pad and a line far off, in points (y
/// down); at 0.02 mm per point with the footprint's origin at (150, 300) the line is 2 mm long at y -2 mm, the
/// circle 0.5 mm round at (0, -1) mm, the pad 0.4 x 0.2 mm
fn drawing(dir: &Path) -> PathBuf {
    use super::testpdf::{TestPage, circle, write};
    let page = TestPage {
        texts: vec![(50.0, 60.0, 14.0, "Recommended land pattern".into())],
        lines: vec![[100.0, 200.0, 200.0, 200.0], [10.0, 380.0, 40.0, 380.0]],
        rects: vec![[120.0, 320.0, 20.0, 10.0]],
        curves: vec![circle([150.0, 250.0], 25.0)],
        ..Default::default()
    };
    let file = dir.join("SELECT-PART.pdf");
    std::fs::write(&file, write([300.0, 400.0], &[page])).unwrap();
    file
}

/// Select, as a person does it: V into the mode; a click picks the line (again drops it, and picks it back), Shift
/// and a box add the circle and the pad, the line far off stays out; W saves them as a DXF beside the notes file
/// and names it in the note. Read back, the DXF is R12 in the footprint's mm, Y up: the line, the circle and the
/// pad where the drawing has them; turned 90°, the footprint's frame turns with it. Save says why it can't before
/// anything is selected and without a scale.
#[test]
fn select_the_drawing_and_save_it_as_dxf() {
    use super::dxf::{Entity, read};
    let dir = scratch("select");
    let pdf = drawing(&dir);
    let lib = super::tests::symbol_library(&dir, &pdf);
    let mut h = super::tests::harness("fp mode select");
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &lib);
    h.run();
    h.get_by_label("Rev:Mixed").click();
    h.run();
    h.run();
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    {
        let st = h.state_mut();
        assert_eq!(st.page, 0);
        st.scale = Some(Scale {
            mm_per_point: 0.02,
            page: 0,
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
    key(&mut h, egui::Key::V);
    assert_eq!(h.state().tool, Tool::Select);
    assert!(h.state().help_line().starts_with("Mode Select:"));
    assert!(
        h.state()
            .dxf_blocker()
            .is_some_and(|w| w.starts_with("select pieces of the drawing first")),
        "{:?}",
        h.state().dxf_blocker()
    );
    let prims = h.state().page_prims().to_vec();
    let find = |want: [f64; 4]| {
        prims
            .iter()
            .position(|p| {
                let b = p.bbox();
                [b.x0, b.y0, b.x1, b.y1]
                    .iter()
                    .zip(want)
                    .all(|(a, b)| (a - b).abs() < 0.01)
            })
            .unwrap_or_else(|| panic!("{want:?} in {prims:#?}"))
    };
    let line = find([100.0, 200.0, 200.0, 200.0]);
    let round = find([125.0, 225.0, 175.0, 275.0]);
    let pad = find([120.0, 320.0, 140.0, 330.0]);
    // A click near the line picks it, again drops it, once more picks it
    let at = screen(&h, [150.0, 201.5]);
    for want in [vec![line], vec![], vec![line]] {
        click(&mut h, at);
        assert_eq!(h.state().selection(), want.as_slice());
    }
    // Shift and a box round the circle and the pad: added, the far line not
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    let (from, to) = (screen(&h, [110.0, 215.0]), screen(&h, [190.0, 345.0]));
    drag(&mut h, from, to);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run();
    let mut want = vec![line, round, pad];
    want.sort_unstable();
    assert_eq!(h.state().selection(), want.as_slice());
    h.get_by_label("3 selected");
    // Without a scale Save says why
    let scale = h.state_mut().scale.take();
    assert!(
        h.state()
            .dxf_blocker()
            .is_some_and(|w| w.starts_with("set the drawing's scale first")),
        "{:?}",
        h.state().dxf_blocker()
    );
    h.state_mut().scale = scale;
    // W saves it beside the notes file, named in the note
    key(&mut h, egui::Key::W);
    let file = dir.join("Rev_Mixed-p1-1.dxf");
    assert_eq!(h.state().rt.dxf_written, Some(Ok(file.clone())));
    assert_eq!(h.state().rt.note, "DXF `Rev_Mixed-p1-1.dxf` ");
    let text = std::fs::read_to_string(&file).unwrap();
    for line in [
        "999\nsymbol Rev:Mixed\n",
        "999\nfootprint Test:Mixed_Pads\n",
    ] {
        assert!(text.contains(line), "{line:?} in {text}");
    }
    let (version, es) = read(&text).unwrap();
    assert_eq!(version, "AC1009");
    let near = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3;
    let has_line = |es: &[Entity], a: [f64; 2], b: [f64; 2]| {
        es.iter().any(|e| {
            matches!(e, Entity::Line { a: p, b: q } if (near(*p, a) && near(*q, b)) || (near(*p, b) && near(*q, a)))
        })
    };
    assert_eq!(es.len(), 3, "{es:#?}");
    assert!(has_line(&es, [-1.0, 2.0], [1.0, 2.0]), "{es:#?}");
    assert!(
        es.iter().any(|e| matches!(e, Entity::Circle { c, r } if near(*c, [0.0, 1.0]) && (r - 0.5).abs() < 1e-3)),
        "{es:#?}"
    );
    let pad_corners = es
        .iter()
        .find_map(|e| match e {
            Entity::Polyline {
                points,
                closed: true,
            } => Some(points.clone()),
            _ => None,
        })
        .expect("the pad, a closed polyline");
    assert_eq!(pad_corners.len(), 4, "{pad_corners:?}");
    for c in [[-0.6, -0.4], [-0.2, -0.4], [-0.2, -0.6], [-0.6, -0.6]] {
        assert!(
            pad_corners.iter().any(|p| near(*p, c)),
            "{c:?} in {pad_corners:?}"
        );
    }
    // Turned 90°: the footprint's +X points up the page, so the line across the page is upright in the DXF
    key(&mut h, egui::Key::T);
    assert_eq!(h.state().place.rotation, 90.0);
    key(&mut h, egui::Key::W);
    let file2 = dir.join("Rev_Mixed-p1-2.dxf");
    assert_eq!(h.state().rt.dxf_written, Some(Ok(file2.clone())));
    assert_eq!(
        h.state().rt.note,
        "DXF `Rev_Mixed-p1-1.dxf` DXF `Rev_Mixed-p1-2.dxf` "
    );
    let (_, es) = read(&std::fs::read_to_string(&file2).unwrap()).unwrap();
    assert!(has_line(&es, [2.0, 1.0], [2.0, -1.0]), "{es:#?}");
    assert!(
        es.iter()
            .any(|e| matches!(e, Entity::Circle { c, .. } if near(*c, [1.0, 0.0]))),
        "{es:#?}"
    );
    // A click on nothing drops the selection; Esc back to Move
    let empty = screen(&h, [250.0, 100.0]);
    click(&mut h, empty);
    assert!(h.state().selection().is_empty());
    key(&mut h, egui::Key::Escape);
    assert_eq!(h.state().tool, Tool::Move);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The footprint or the model hidden for a moment to see the drawing (VIEW-28): H hides the footprint and shows it
/// again where it was, J the model, Space held hides both until it is let go; the mode bar shows each, the help
/// line names the keys, and a hidden footprint gives the measuring ruler nothing to snap to
#[test]
fn hide_the_overlays_for_a_moment() {
    let dir = scratch("hide");
    let mut h = open("fp mode hide", &dir);
    let pad = h.state().fp_to_page([0.0, -1.5]);
    let pixel = |h: &mut H| {
        over_canvas(h);
        let st = h.state();
        let p = st.to_screen(st.rt.canvas, pad);
        let img = h.render().unwrap();
        let ppp = h.ctx.pixels_per_point();
        img.get_pixel((p.x * ppp) as u32, (p.y * ppp) as u32).0
    };
    let dist = |a: [u8; 4], b: [u8; 4]| (0..3).map(|i| a[i].abs_diff(b[i]) as u32).sum::<u32>();
    let shown = pixel(&mut h);
    let place = h.state().place.clone();
    let help = h.state().help_line();
    for k in [
        "H hides or shows the footprint",
        "J the model",
        "hold Space",
    ] {
        assert!(help.contains(k), "{k}: {help}");
    }
    key(&mut h, egui::Key::H);
    assert!(!h.state().footprint_shown());
    let hidden = pixel(&mut h);
    assert!(dist(shown, hidden) > 60, "{shown:?} -> {hidden:?}");
    assert_eq!(h.state().place, place, "its place kept");
    // Nothing to snap to while hidden
    key(&mut h, egui::Key::G);
    h.hover_at(screen(&h, pad) + egui::vec2(3.0, 3.0));
    h.run();
    assert_eq!(h.state().rt.snap, None);
    key(&mut h, egui::Key::M);
    // The mode bar's button shows it again, where it was
    h.get_by_label("Footprint · H").click();
    h.run();
    assert!(h.state().footprint_shown());
    assert!(dist(shown, pixel(&mut h)) < 10);
    // J: the model
    key(&mut h, egui::Key::J);
    assert!(h.state().rt.model_hidden && !h.state().model_shown());
    key(&mut h, egui::Key::J);
    assert!(h.state().model_shown());
    // Space held: the page alone, until let go
    over_canvas(&mut h);
    let space = |pressed| egui::Event::Key {
        key: egui::Key::Space,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Default::default(),
    };
    h.event(space(true));
    h.run();
    h.run();
    assert!(!h.state().footprint_shown() && !h.state().model_shown());
    h.get_by_label("hold Space: the page alone");
    assert!(dist(hidden, pixel(&mut h)) < 10, "the page alone");
    h.event(space(false));
    h.run();
    h.run();
    assert!(h.state().footprint_shown() && h.state().model_shown());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Measure across (VIEW-29): one ruler from the footprint to the drawing. Each end snaps to whichever is nearer, a
/// pad's centre or side middle or the drawing's lines, and says what it snapped to; with both in reach Ctrl takes
/// the other; the note names both ends' snaps
#[test]
fn measure_from_the_footprint_to_the_drawing() {
    let dir = scratch("across");
    let mut h = open("fp mode across", &dir);
    key(&mut h, egui::Key::Y);
    assert_eq!(h.state().tool, Tool::MeasureAcross);
    // Pad 2 at (0, -1.5) mm is at page (150, 225); the drawing's right pad starts at x 160 (y 220..240)
    let p2 = h.state().fp_to_page([0.0, -1.5]);
    assert_eq!(p2, [150.0, 225.0]);
    let near = |h: &H, p: [f32; 2]| screen(h, p) + egui::vec2(3.0, -3.0);
    let (a, b) = (near(&h, p2), near(&h, [200.0, 200.0]));
    drag(&mut h, a, b);
    let close = |got: [f32; 2], want: [f32; 2]| {
        (got[0] - want[0]).abs() < 1e-3 && (got[1] - want[1]).abs() < 1e-3
    };
    let [ra, rb] = h.state().ruler.unwrap();
    assert!(close(ra, p2) && close(rb, [200.0, 200.0]), "{ra:?} {rb:?}");
    assert_eq!(
        h.state().rt.end_snaps,
        [Some(SnapKind::PadCentre), Some(SnapKind::End)]
    );
    assert_eq!(h.state().rt.note, "", "nothing goes in by itself");
    h.get_by_label("+ note").click();
    h.run();
    let mm = (50.0f64.hypot(25.0)) * 0.02;
    assert_eq!(
        h.state().rt.note.trim(),
        format!("M1 = {mm:.3} mm (pad centre to line end)")
    );
    // Both in reach: the drawing's pad edge at x 160 is 4 points off, pad 2's centre 6: the nearer wins, Ctrl the
    // other
    let st = h.state_mut();
    let at = [156.0, 225.0];
    let snap = st.across_snap(at).unwrap();
    assert_eq!((snap.kind, snap.at[0]), (SnapKind::Line, 160.0));
    st.rt.other_source = true;
    let snap = st.across_snap(at).unwrap();
    assert_eq!((snap.kind, snap.at), (SnapKind::PadCentre, p2));
    // Only one in reach: that one, Ctrl or not
    let snap = st.across_snap([152.0, 225.0]).unwrap();
    assert_eq!(snap.kind, SnapKind::PadCentre);
    st.rt.other_source = false;
    // The mode bar and the Measure section name the mode with its key
    h.get_by_label("Measure across · Y");
    h.get_by_label("Measure across");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// VIEW-35: while the footprint moves its centre snaps to the drawing's lines as a ruler end does. A drag that
/// brings it near the dimension's corner puts it on the corner exactly, the target shown; the arrows nudge it off
/// the corner onto the line once out of reach; Alt frees a drag; `.` and the mode bar's button turn it off and on
#[test]
fn the_footprint_centre_snaps_to_the_drawing() {
    let dir = scratch("centre");
    let mut h = open("fp mode centre", &dir);
    let corner = [100.0, 200.0];
    assert!(h.state().centre_snap_on(), "on by default");
    // A drag from the centre to a few points off the corner: on the corner while held, and after it is let go
    let from = screen(&h, h.state().place.origin);
    let near = screen(&h, [corner[0] + 3.0, corner[1] + 2.0]);
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    for i in 1..=5 {
        h.hover_at(from + (near - from) * (i as f32 / 5.0));
        h.run();
    }
    assert_eq!(h.state().place.origin, corner, "snapped while held");
    let (snap, _) = h
        .state()
        .rt
        .centre_snapped
        .clone()
        .expect("the target shows");
    assert!(
        matches!(snap.kind, SnapKind::End | SnapKind::Crossing),
        "{snap:?}"
    );
    h.drop_at(near);
    h.run();
    assert_eq!(h.state().place.origin, corner, "dropped on the corner");
    // The arrows: one press stays on the corner, a run of them pulls it along the line once out of the corner's
    // reach (10 px at 2 screen points per page point: 5 points)
    over_canvas(&mut h);
    h.key_press(egui::Key::ArrowRight);
    h.run();
    assert_eq!(h.state().place.origin, corner, "one press stays snapped");
    for _ in 0..13 {
        h.key_press(egui::Key::ArrowRight);
        h.run();
    }
    let o = h.state().place.origin;
    assert!(
        (o[0] - 107.0).abs() < 0.01 && o[1] == 200.0,
        "onto the line, 14 half points on: {o:?}"
    );
    // Alt held: the drag's end is where the pointer leaves it
    h.state_mut().place.origin = [150.0, 300.0];
    h.run();
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::ALT));
    let start = screen(&h, [150.0, 300.0]);
    drag(&mut h, start, near);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run();
    let o = h.state().place.origin;
    assert!(
        (o[0] - 103.0).abs() < 0.5 && (o[1] - 202.0).abs() < 0.5,
        "Alt frees it: {o:?}"
    );
    // `.` turns it off: the same drag leaves it where the pointer is; the button turns it on again
    key(&mut h, egui::Key::Period);
    assert!(!h.state().centre_snap_on());
    assert!(
        h.state()
            .help_line()
            .contains("free of the drawing's lines")
    );
    h.state_mut().place.origin = [150.0, 300.0];
    h.run();
    let start = screen(&h, [150.0, 300.0]);
    drag(&mut h, start, near);
    let o = h.state().place.origin;
    assert!(
        (o[0] - 103.0).abs() < 0.5 && (o[1] - 202.0).abs() < 0.5,
        "off: {o:?}"
    );
    h.get_by_label_contains("Centre snap · ").click();
    h.run();
    assert!(h.state().centre_snap_on(), "the button turns it on");
    std::fs::remove_dir_all(&dir).unwrap();
}
