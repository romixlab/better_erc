//! The review driven through the UI as a person would: a footprint file, a datasheet PDF the test writes, the page
//! search, the ruler, Accept

use super::testpdf::{TestPage, write};
use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use std::sync::Mutex;

fn footprint() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../erc_kicad/tests/fixtures/footprints/Test.pretty/Mixed_Pads.kicad_mod")
}

/// A datasheet: a cover page, then the land pattern page with a 2.00 mm dimension 100 points long (so the drawing
/// is at 0.02 mm per point, 17.6:1), then a page of package text
fn datasheet(dir: &Path) -> PathBuf {
    let text = |x: f32, y: f32, s: f32, t: &str| (x, y, s, t.to_string());
    let pages = [
        TestPage {
            texts: vec![
                text(50.0, 60.0, 20.0, "TEST-PART"),
                text(50.0, 90.0, 12.0, "Features"),
            ],
            ..Default::default()
        },
        TestPage {
            texts: vec![
                text(50.0, 60.0, 14.0, "Recommended land pattern"),
                text(140.0, 190.0, 10.0, "2.00"),
            ],
            // The dimension line and its ends
            lines: vec![
                [100.0, 200.0, 200.0, 200.0],
                [100.0, 195.0, 100.0, 205.0],
                [200.0, 195.0, 200.0, 205.0],
            ],
            rects: vec![[100.0, 220.0, 40.0, 20.0], [160.0, 220.0, 40.0, 20.0]],
        },
        TestPage {
            texts: vec![text(50.0, 60.0, 12.0, "Package dimensions")],
            ..Default::default()
        },
    ];
    let file = dir.join("TEST-PART.pdf");
    std::fs::write(&file, write([300.0, 400.0], &pages)).unwrap();
    file
}

const ACCEPT: &str = "Accept: the footprint matches the drawing";

fn disabled(h: &Harness<'_, FootprintReview>, label: &str) -> bool {
    use egui_kittest::kittest::NodeT;
    h.get_by_label(label).accesskit_node().is_disabled()
}

fn button<'a>(h: &'a Harness<'_, FootprintReview>, label: &'a str) -> egui_kittest::Node<'a> {
    h.get_by_label(label)
}

#[test]
fn review_a_footprint_over_its_datasheet() {
    let dir = std::env::temp_dir().join(format!("berc-fp-review-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = datasheet(&dir);
    let signals: Arc<Mutex<Vec<String>>> = Arc::default();
    let s = signals.clone();
    let usage: Usage = Arc::new(move |sig: &str| s.lock().unwrap().push(sig.to_string()));
    let settings = FootprintReviewSettings {
        font_dirs: Vec::new(),
        library_search: Vec::new(),
        ..Default::default()
    };
    let mut cx = Context::new();
    let mut h = Harness::builder()
        .with_size(vec2(1200.0, 800.0))
        .build_ui_state(
            move |ui, state: &mut FootprintReview| state.ui(ui, &mut cx, Id::new("fp test")),
            FootprintReview::with(settings, usage),
        );
    h.run();
    // The file pickers are the system's: open what they would give
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &footprint());
    h.state_mut().open_datasheet(&ctx, &pdf);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    let want_hash = better_erc::fp::hash_file(&footprint(), None).unwrap().hash;
    assert_eq!(h.state().rt.sourced.as_ref().unwrap().hash, want_hash);
    // The page search goes to the land pattern page
    assert_eq!(h.state().page, 1, "{:?}", h.state().rt.page_hits);
    h.get_by_label("page 2 of 3");
    // Accept says why it can't be used yet
    assert!(disabled(&h, ACCEPT));
    h.get_by_label("set the drawing's scale with the ruler first");
    // The ruler along the 2.00 mm dimension
    button(&h, "Ruler").click();
    h.run();
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    let (canvas, a, b) = {
        let st = h.state();
        let c = st.rt.canvas;
        (
            c,
            st.to_screen(c, [100.0, 200.0]),
            st.to_screen(c, [200.0, 200.0]),
        )
    };
    assert!(
        canvas.contains(a) && canvas.contains(b),
        "{canvas:?} {a:?} {b:?}"
    );
    h.hover_at(a);
    h.run();
    h.drag_at(a);
    h.run();
    for i in 1..=10 {
        h.hover_at(a + (b - a) * (i as f32 / 10.0));
        h.run();
    }
    h.drop_at(b);
    h.run();
    let [ra, rb] = h.state().ruler.expect("a ruler was drawn");
    for (got, want) in [(ra, [100.0, 200.0]), (rb, [200.0, 200.0])] {
        assert!(
            (got[0] - want[0]).abs() < 0.5 && (got[1] - want[1]).abs() < 0.5,
            "{ra:?} {rb:?}"
        );
    }
    h.state_mut().rt.ruler_mm = "2.00".into();
    h.run();
    button(&h, "Set scale").click();
    h.run();
    let scale = h.state().scale.clone().expect("the scale is set");
    assert!((scale.mm_per_point - 0.02).abs() < 1e-4, "{scale:?}");
    assert_eq!(scale.page, 1);
    // Move the footprint with the keys, turn it, mirror it
    let origin = h.state().place.origin;
    h.hover_at(canvas.center());
    h.run();
    h.key_press(egui::Key::ArrowRight);
    h.key_press(egui::Key::R);
    h.key_press(egui::Key::M);
    h.run();
    let st = h.state();
    assert!(st.place.origin[0] > origin[0]);
    assert_eq!((st.place.rotation, st.place.mirror), (90.0, true));
    // Accept gives the review hash
    assert!(!disabled(&h, ACCEPT));
    button(&h, ACCEPT).click();
    h.run();
    assert_eq!(h.state().rt.accepted.as_deref(), Some(want_hash.as_str()));
    h.get_by_label(&format!("Accepted. {FIELD} = {want_hash} (copied)"));
    let r = &h.state().reviews[0];
    assert_eq!((r.page, r.hash.as_str()), (2, want_hash.as_str()));
    assert_eq!(r.datasheet.as_deref(), Some(pdf.as_path()));
    // Each step records its usage signal
    let got = signals.lock().unwrap().clone();
    for want in ["footprint", "datasheet", "scale", "accept"] {
        assert!(
            got.contains(&format!("app fp-review {want}")),
            "{want}: {got:?}"
        );
    }
    // A picture to look at: BERC_FP_SNAPSHOT names the file
    if let Some(p) = std::env::var_os("BERC_FP_SNAPSHOT") {
        h.render().unwrap().save(p).unwrap();
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A real board: BERC_FP_BOARD names a .kicad_pcb, BERC_FP_REF the footprint to pick (default: the first with a
/// Datasheet field); the library search runs with the default command when it is on PATH. Not run without them.
#[test]
fn real_board() {
    let Some(board) = std::env::var_os("BERC_FP_BOARD") else {
        eprintln!("BERC_FP_BOARD not set: no real board opened");
        return;
    };
    let mut cx = Context::new();
    let mut h = Harness::builder()
        .with_size(vec2(1400.0, 900.0))
        .build_ui_state(
            move |ui, state: &mut FootprintReview| state.ui(ui, &mut cx, Id::new("fp real")),
            FootprintReview::with(FootprintReviewSettings::default(), Arc::new(|_: &str| {})),
        );
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, Path::new(&board));
    let want = std::env::var("BERC_FP_REF").ok();
    let i = {
        let b = h.state().rt.board.as_ref().expect("the board opened");
        b.layout
            .footprints
            .iter()
            .position(|f| match &want {
                Some(r) => &f.reference == r,
                None => better_erc::model::field_value(&f.fields, "Datasheet")
                    .is_some_and(|d| d.starts_with("http")),
            })
            .expect("such a footprint")
    };
    h.state_mut().pick(i);
    h.run();
    eprintln!(
        "{:?} / {:?} / {:?}",
        h.state().rt.sourced.as_ref().map(|s| s.source.describe()),
        h.state().rt.board_note,
        h.state().rt.error
    );
    eprintln!("query {:?}", h.state().rt.query);
    eprintln!("queries {:?}", h.state().rt.queries);
    let r = h
        .state_mut()
        .rt
        .searching
        .take()
        .map(|rx| rx.recv().unwrap());
    eprintln!("library: {r:?}");
    if let Some(Ok((hits, _))) = &r
        && let Some(first) = hits.first()
    {
        h.state_mut().open_datasheet(&ctx, &first.path);
        h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
        h.run();
        h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
        h.run();
        eprintln!("page {} hits {:?}", h.state().page, h.state().rt.page_hits);
    }
    // BERC_FP_SCALE (mm per point) and BERC_FP_AT (`x,y` on screen): the footprint placed there at that scale
    if let Ok(k) = std::env::var("BERC_FP_SCALE") {
        let page = h.state().page;
        h.state_mut().scale = Some(Scale {
            mm_per_point: k.parse().unwrap(),
            page,
            mm: 0.0,
            note: None,
        });
    }
    if let Some((x, y)) = std::env::var("BERC_FP_AT").ok().and_then(|v| {
        let (x, y) = v.split_once(',')?;
        Some((x.parse::<f32>().ok()?, y.parse::<f32>().ok()?))
    }) {
        let c = h.state().rt.canvas;
        h.state_mut().place.origin = h.state().to_page(c, pos2(x, y));
    }
    h.run();
    if let Some(p) = std::env::var_os("BERC_FP_SNAPSHOT") {
        h.render().unwrap().save(p).unwrap();
    }
}

#[test]
fn scale_notes() {
    assert_eq!(
        scale_note("LAND PATTERN EXAMPLE\nSCALE:20X\n"),
        Some(("SCALE:20X".into(), 20.0))
    );
    assert_eq!(scale_note("Scale 10:1"), Some(("Scale 10:1".into(), 10.0)));
    assert_eq!(
        scale_note("not to scale; SCALE: 2.5X"),
        Some(("SCALE: 2.5X".into(), 2.5))
    );
    assert_eq!(scale_note("not to scale"), None);
}
