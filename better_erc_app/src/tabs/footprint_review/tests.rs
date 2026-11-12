//! The review driven through the UI as a person would: a library symbol from a library the test writes, its
//! footprint and its datasheet PDF (written by the test too) following it, the page search, the ruler, Accept
//! writing the review hash into the symbol; a footprint opened alone as a preview that can't be accepted

use super::testpdf::{TestPage, write};
use super::*;
use crate::settings::{BlendMode, LayerColors, Rgb};
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
            ..Default::default()
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

const PREVIEW: &str = "a footprint opened alone is a preview: there is no symbol to put the review hash on; pick \
                       its symbol from a library or a board's list";

/// The test footprints' library, Test.pretty, by a `fp-lib-table` in `dir`
fn footprint_table(dir: &Path) {
    let pretty = footprint().parent().unwrap().to_path_buf();
    std::fs::write(
        dir.join("fp-lib-table"),
        format!(
            "(fp_lib_table (version 7)\n  (lib (name \"Test\")(type \"KiCad\")(uri \"{}\")(options \"\")(descr \"\")))\n",
            pretty.display()
        ),
    )
    .unwrap();
}

/// A library symbol in KiCad's layout, with these extra properties
fn symbol_text(name: &str, props: &[(&str, &str)]) -> String {
    let mut out = format!(
        "  (symbol \"{name}\" (in_bom yes) (on_board yes)\n    (property \"Reference\" \"U\" (at 0 0 0) (effects (font (size 1.27 1.27))))\n"
    );
    for (k, v) in props {
        out.push_str(&format!(
            "    (property \"{k}\" \"{v}\" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))\n"
        ));
    }
    out.push_str(&format!(
        "    (symbol \"{name}_1_1\" (pin passive line (at 0 0 0) (length 1.27) (name \"A\" (effects (font (size 1.27 1.27)))) (number \"1\" (effects (font (size 1.27 1.27)))))))\n"
    ));
    out
}

/// A symbol library in `dir`, `Rev.kicad_sym`: `Mixed` with footprint Test:Mixed_Pads and the datasheet `pdf`,
/// `Generic` without a footprint; the footprints through a `fp-lib-table` beside it
fn symbol_library(dir: &Path, pdf: &Path) -> PathBuf {
    footprint_table(dir);
    let lib = dir.join("Rev.kicad_sym");
    std::fs::write(
        &lib,
        format!(
            "(kicad_symbol_lib (version 20231120) (generator \"kicad_symbol_editor\")\n{}{})\n",
            symbol_text(
                "Mixed",
                &[
                    ("Footprint", "Test:Mixed_Pads"),
                    ("Datasheet", &pdf.display().to_string()),
                    ("MPN", "TEST-PART"),
                ]
            ),
            symbol_text("Generic", &[("Datasheet", "~")]),
        ),
    )
    .unwrap();
    lib
}

fn disabled(h: &Harness<'_, FootprintReview>, label: &str) -> bool {
    use egui_kittest::kittest::NodeT;
    h.get_by_label(label).accesskit_node().is_disabled()
}

fn button<'a>(h: &'a Harness<'_, FootprintReview>, label: &'a str) -> egui_kittest::Node<'a> {
    h.get_by_label(label)
}

/// A symbol picked from a symbol library: its footprint and its datasheet follow, the page search finds the land
/// pattern, the ruler sets the scale, Accept writes the review hash into the library symbol; a symbol without a
/// footprint is refused with the reason
#[test]
fn review_a_symbol_from_its_library() {
    let dir = std::env::temp_dir().join(format!("berc-fp-review-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = datasheet(&dir);
    let lib = symbol_library(&dir, &pdf);
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
        .with_size(vec2(1200.0, 1400.0))
        .build_ui_state(
            move |ui, state: &mut FootprintReview| state.ui(ui, &mut cx, Id::new("fp test")),
            FootprintReview::with(settings, usage),
        );
    h.run();
    // The file picker is the system's: open what it would give
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &lib);
    h.run();
    h.get_by_label("2 symbols in Rev, 1 to review");
    // Nothing picked yet: Accept says to pick a symbol
    assert!(disabled(&h, ACCEPT));
    h.get_by_label("pick a symbol first: its footprint and datasheet follow");
    // The symbol: its footprint follows, and its datasheet from its Datasheet field
    h.get_by_label("Rev:Mixed").click();
    h.run();
    h.run();
    assert_eq!(h.state().datasheet.as_deref(), Some(pdf.as_path()));
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    let want_hash = better_erc::fp::hash_file(&footprint(), None).unwrap().hash;
    assert_eq!(h.state().rt.sourced.as_ref().unwrap().hash, want_hash);
    h.get_by_label("Test:Mixed_Pads");
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
    assert_eq!(
        h.state().tool,
        Tool::Move,
        "the scale set, the footprint moves again"
    );
    assert!((scale.mm_per_point - 0.02).abs() < 1e-4, "{scale:?}");
    assert_eq!(scale.page, 1);
    // Move the footprint with the keys, turn it, mirror it
    let origin = h.state().place.origin;
    h.hover_at(canvas.center());
    h.run();
    h.key_press(egui::Key::ArrowRight);
    h.key_press(egui::Key::T);
    h.key_press(egui::Key::M);
    h.run();
    let st = h.state();
    assert!(st.place.origin[0] > origin[0]);
    assert_eq!((st.place.rotation, st.place.mirror), (90.0, true));
    // Accept shows what it writes where, and writes the review hash into the library symbol once confirmed
    assert!(!disabled(&h, ACCEPT));
    button(&h, ACCEPT).click();
    h.run();
    h.get_by_label("Write the review hash?");
    h.get_by_label("Rev.kicad_sym (library)");
    h.get_by_label(&format!("(none) → {want_hash}"));
    assert!(h.state().rt.accepted.is_none(), "nothing accepted yet");
    button(&h, "Write (Enter)").click();
    h.run();
    assert_eq!(h.state().rt.accepted.as_deref(), Some(want_hash.as_str()));
    h.get_by_label(&format!("Accepted. {FIELD} = {want_hash} (copied)"));
    assert!(
        matches!(h.state().rt.written, Some(Ok(_))),
        "{:?}",
        h.state().rt.written
    );
    h.get_by_label("Rev:Mixed: written into");
    h.get_by_label("  Rev.kicad_sym (library)");
    let text = std::fs::read_to_string(&lib).unwrap();
    assert!(
        text.contains(&format!("(property \"Footprint_Review\" \"{want_hash}\"")),
        "{text}"
    );
    let r = &h.state().reviews[0];
    assert_eq!((r.page, r.hash.as_str()), (2, want_hash.as_str()));
    assert_eq!(r.datasheet.as_deref(), Some(pdf.as_path()));
    // berc fp check --lib agrees, and the list says so without reading the library again
    let check = better_erc::fp::check_libraries(&LibCheckOptions {
        libraries: vec![lib.display().to_string()],
        project: dir.clone(),
        search: LibrarySearch::default(),
    })
    .unwrap();
    let mixed = check
        .symbols
        .iter()
        .find(|e| e.symbol == "Rev:Mixed")
        .unwrap();
    assert_eq!(mixed.status, SymbolStatus::Ok);
    h.get_by_label(&format!("{want_hash}, the same: reviewed"));
    assert_eq!(
        h.state().rt.symbols.as_ref().unwrap().rows[1].status,
        SymbolStatus::Ok
    );
    // Nothing else needs a review
    assert!(disabled(&h, "Next to review ▶"));
    // A symbol without a footprint: refused, with the reason
    h.get_by_label("Rev:Generic").click();
    h.run();
    let why = "Rev:Generic sets no footprint (its Footprint field is empty): nothing to review, and no review \
               hash to give it";
    assert!(disabled(&h, ACCEPT));
    assert_eq!(h.state().accept_blocker().as_deref(), Some(why));
    assert!(h.state().rt.items.is_empty());
    // Each step records its usage signal
    let got = signals.lock().unwrap().clone();
    for want in ["library", "symbol", "datasheet", "scale", "accept", "write"] {
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

/// A footprint opened alone is a preview: drawn over the datasheet, but Accept is off with the reason
#[test]
fn a_footprint_alone_is_a_preview() {
    let dir = std::env::temp_dir().join(format!("berc-fp-preview-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = datasheet(&dir);
    let settings = FootprintReviewSettings {
        font_dirs: Vec::new(),
        library_search: Vec::new(),
        ..Default::default()
    };
    let mut cx = Context::new();
    let mut h = Harness::builder()
        .with_size(vec2(1200.0, 1400.0))
        .build_ui_state(
            move |ui, state: &mut FootprintReview| state.ui(ui, &mut cx, Id::new("fp preview")),
            FootprintReview::with(settings, Arc::new(|_: &str| {})),
        );
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &footprint());
    h.state_mut().open_datasheet(&ctx, &pdf);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    let page = h.state().page;
    h.state_mut().scale = Some(Scale {
        mm_per_point: 0.02,
        page,
        mm: 2.0,
        note: None,
    });
    h.run();
    assert!(!h.state().rt.items.is_empty());
    h.get_by_label("Preview: no symbol to write the review hash into");
    assert!(disabled(&h, ACCEPT));
    h.get_by_label(PREVIEW);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A real board: BERC_FP_BOARD names a KiCad project or board, BERC_FP_SYMBOL the library symbol to review
/// (default: the first in the list to review); the library search runs with the default command when it is on
/// PATH. Not run without them. BERC_FP_BLEND picks a blend mode; BERC_FP_PICK a board footprint by reference to
/// preview instead of a symbol; BERC_FP_SHOW layer groups to show too (`Mask` or `Paste` picks it as the one of
/// copper/mask/paste shown); BERC_FP_INVERT inverts the page.
#[test]
fn real_board() {
    let Some(board) = std::env::var_os("BERC_FP_BOARD") else {
        eprintln!("BERC_FP_BOARD not set: no real board opened");
        return;
    };
    // BERC_FP_BLEND (normal, multiply, screen, difference): that blend mode, through the GPU pass
    let blend: Option<BlendMode> = std::env::var("BERC_FP_BLEND").ok().map(|m| {
        serde_json::from_value(serde_json::Value::String(m))
            .expect("a blend mode: normal, multiply, screen, difference")
    });
    let mut cx = Context::new();
    let mut b = Harness::builder().with_size(vec2(1400.0, 900.0));
    let rs = blend.map(|_| {
        egui_kittest::wgpu::create_render_state(
            egui_kittest::wgpu::default_wgpu_setup(),
            eframe::egui_wgpu::RendererOptions::PREDICTABLE,
        )
    });
    if let Some(rs) = &rs {
        b = b.renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(
            rs.clone(),
        ));
    }
    let mut review =
        FootprintReview::with(FootprintReviewSettings::default(), Arc::new(|_: &str| {}));
    // The real board's libraries: this PC's global tables too
    review.rt.config = better_erc::fp::kicad_config_dir();
    let mut h = b.build_ui_state(
        move |ui, state: &mut FootprintReview| state.ui(ui, &mut cx, Id::new("fp real")),
        review,
    );
    if let Some(rs) = &rs {
        blend::register(&h.ctx, rs, 1, None);
    }
    h.state_mut().blend = blend;
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, Path::new(&board));
    let want = std::env::var("BERC_FP_SYMBOL").ok();
    let pick = std::env::var("BERC_FP_PICK").ok().map(|r| {
        let b = h.state().rt.board.as_ref().expect("the board opened");
        b.layout
            .footprints
            .iter()
            .position(|f| f.reference == r)
            .expect("such a reference")
    });
    if let Ok(show) = std::env::var("BERC_FP_SHOW") {
        let show: Vec<&str> = show.split(',').collect();
        h.state_mut().hidden.retain(|k| !show.contains(&k.as_str()));
        if let Some(k) = VIEWS.into_iter().find(|k| show.contains(&k.name())) {
            h.state_mut().cu_shown = k;
        }
    }
    if std::env::var_os("BERC_FP_INVERT").is_some() {
        h.state_mut().invert = Some(true);
    }
    let i = {
        let l = h
            .state()
            .rt
            .symbols
            .as_ref()
            .expect("the symbol list opened");
        eprintln!("{}", l.title);
        l.rows
            .iter()
            .position(|r| match &want {
                Some(s) => &r.symbol == s,
                None => r.needs_review(),
            })
            .expect("such a symbol")
    };
    match pick {
        Some(f) => h.state_mut().pick(f),
        None => h.state_mut().select_symbol(i, true),
    }
    h.run();
    eprintln!(
        "{:?} / {:?} / {:?} / accept: {:?}",
        h.state().rt.sourced.as_ref().map(|s| s.source.describe()),
        h.state().rt.symbol_note,
        h.state().rt.error,
        h.state().accept_blocker()
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

/// The blend modes on the GPU: a pad drawn white over the page's white paper is black in Difference, white in
/// Normal and Screen; a red pad is red in Multiply over paper and dark over the drawing's grey pad
#[test]
fn blend_modes_mix_with_the_page() {
    use eframe::egui_wgpu;
    let dir = std::env::temp_dir().join(format!("berc-fp-blend-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = datasheet(&dir);
    let settings = FootprintReviewSettings {
        font_dirs: Vec::new(),
        library_search: Vec::new(),
        ..Default::default()
    };
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut cx = Context::new();
    let mut h = Harness::builder()
        .with_size(vec2(1200.0, 1400.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(
            rs.clone(),
        ))
        .build_ui_state(
            move |ui, state: &mut FootprintReview| state.ui(ui, &mut cx, Id::new("fp blend")),
            FootprintReview::with(settings, Arc::new(|_: &str| {})),
        );
    blend::register(&h.ctx, &rs, 1, None);
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &footprint());
    h.state_mut().open_datasheet(&ctx, &pdf);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    // The first numbered pad, opaque, at 0.02 mm per point
    let st = h.state_mut();
    st.scale = Some(Scale {
        mm_per_point: 0.02,
        page: 1,
        mm: 2.0,
        note: None,
    });
    st.opacity = Some(1.0);
    let pad = st
        .rt
        .items
        .iter()
        .find_map(|i| match i {
            Item::Number { at, .. } => Some(*at),
            _ => None,
        })
        .unwrap();
    // Put the pad's middle on a page point
    let place = |st: &mut FootprintReview, at: [f32; 2]| {
        st.place.origin = [0.0, 0.0];
        let off = st.fp_to_page(pad);
        st.place.origin = [at[0] - off[0], at[1] - off[1]];
    };
    let pixel =
        |h: &mut Harness<'_, FootprintReview>, mode: BlendMode, rgb: [u8; 3], at: [f32; 2]| {
            let st = h.state_mut();
            st.blend = Some(mode);
            st.rt.settings.colors.copper = Rgb(rgb);
            place(st, at);
            h.run();
            let st = h.state();
            let p = st.to_screen(st.rt.canvas, at);
            let img = h.render().unwrap();
            let ppp = h.ctx.pixels_per_point();
            let px = img.get_pixel((p.x * ppp) as u32, (p.y * ppp) as u32).0;
            [px[0], px[1], px[2]]
        };
    let paper = [150.0, 330.0];
    // The drawing's black pad (filled 100..140 x 220..240)
    let ink = [120.0, 230.0];
    let near = |got: [u8; 3], want: [u8; 3]| got.iter().zip(want).all(|(g, w)| g.abs_diff(w) <= 12);
    let white = [255, 255, 255];
    let red = [220, 30, 30];
    for (mode, rgb, at, want) in [
        (BlendMode::Difference, white, paper, [0, 0, 0]),
        (BlendMode::Normal, white, paper, white),
        (BlendMode::Screen, red, paper, white),
        (BlendMode::Multiply, red, paper, red),
        // The drawing's pads are grey 0.2: 0.2 x red
        (BlendMode::Multiply, red, ink, [44, 6, 6]),
        (BlendMode::Difference, white, ink, [204, 204, 204]),
    ] {
        let got = pixel(&mut h, mode, rgb, at);
        assert!(
            near(got, want),
            "{mode:?} {rgb:?} at {at:?}: {got:?}, want {want:?}"
        );
    }
    if let Some(p) = std::env::var_os("BERC_FP_SNAPSHOT") {
        h.render().unwrap().save(p).unwrap();
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A KiCad project in `dir` around the board of placed.kicad_pcb: its Mixed_Pads footprints U7-U12 placed for
/// library symbol `Rev:Mixed`, with the library file and a sym-lib-table; returns the board
fn project(dir: &Path) -> PathBuf {
    let fixtures =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../erc_kicad/tests/fixtures/footprints");
    let board = dir.join("rev.kicad_pcb");
    std::fs::copy(fixtures.join("placed.kicad_pcb"), &board).unwrap();
    let symbol = |name: &str| {
        format!(
            "(symbol \"{name}\" (in_bom yes) (on_board yes)\n    (property \"Reference\" \"U\" (at 0 0 0) (effects (font (size 1.27 1.27))))\n    (property \"Value\" \"Mixed\" (at 0 0 0) (effects (font (size 1.27 1.27))))\n    (property \"Footprint\" \"Test:Mixed_Pads\" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))\n    (symbol \"Mixed_1_1\" (pin passive line (at 0 0 0) (length 1.27) (name \"A\" (effects (font (size 1.27 1.27)))) (number \"1\" (effects (font (size 1.27 1.27)))))))\n"
        )
    };
    let root = "b0000000-0000-4000-8000-000000000000";
    let placed: String = (7..=12)
        .map(|n| {
            format!(
                "  (symbol (lib_id \"Rev:Mixed\") (at {x} 50 0) (unit 1) (in_bom yes) (on_board yes) (dnp no) (uuid \"b0000000-0000-4000-8000-0000000001{n:02}\")\n    (property \"Reference\" \"U{n}\" (at {x} 45 0) (effects (font (size 1.27 1.27))))\n    (property \"Value\" \"Mixed\" (at {x} 55 0) (effects (font (size 1.27 1.27))))\n    (property \"Footprint\" \"Test:Mixed_Pads\" (at {x} 50 0) (effects (font (size 1.27 1.27)) (hide yes)))\n    (pin \"1\" (uuid \"b0000000-0000-4000-8000-0000000002{n:02}\"))\n    (instances (project \"rev\" (path \"/{root}\" (reference \"U{n}\") (unit 1)))))\n",
                x = 20 * n
            )
        })
        .collect();
    std::fs::write(
        dir.join("rev.kicad_sch"),
        format!(
            "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n  (uuid \"{root}\")\n  (paper \"A4\")\n  (lib_symbols {})\n{placed}  (sheet_instances (path \"/\" (page \"1\"))))\n",
            symbol("Rev:Mixed")
        ),
    )
    .unwrap();
    std::fs::write(dir.join("rev.kicad_pro"), "{}\n").unwrap();
    footprint_table(dir);
    std::fs::write(
        dir.join("Rev.kicad_sym"),
        format!(
            "(kicad_symbol_lib (version 20231120) (generator \"kicad_symbol_editor\")\n  {})\n",
            symbol("Mixed")
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("sym-lib-table"),
        "(sym_lib_table (version 7)\n  (lib (name \"Rev\")(type \"KiCad\")(uri \"${KIPRJMOD}/Rev.kicad_sym\")(options \"\")(descr \"\")))\n",
    )
    .unwrap();
    board
}

/// A symbol from a board's list: Accept writes its footprint's review hash into the library symbol and the
/// schematic's copy, and `berc fp check` then finds the board's footprints ok; the board's own footprint list is a
/// preview
#[test]
fn accept_writes_into_the_symbols() {
    let dir = std::env::temp_dir().join(format!("berc-fp-write-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let board = project(&dir);
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
        .with_size(vec2(1200.0, 1400.0))
        .build_ui_state(
            move |ui, state: &mut FootprintReview| state.ui(ui, &mut cx, Id::new("fp write")),
            FootprintReview::with(settings, usage),
        );
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &board);
    h.run();
    h.get_by_label("1 library symbols on rev.kicad_pcb, 1 to review");
    // A board footprint from the preview list: Accept is off
    h.get_by_label_contains("library footprints on rev.kicad_pcb (preview)")
        .click();
    h.run();
    h.get_by_label("Mixed_Pads  (U7, U8, U9, U10, U11, U12)")
        .click();
    h.run();
    assert!(h.state().rt.target.is_none());
    // The symbol the board's footprints come from
    h.get_by_label("Rev:Mixed").click();
    h.run();
    h.get_by_label("U7, U8, U9, U10, U11, U12");
    h.state_mut().open_datasheet(&ctx, &pdf);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    let page = h.state().page;
    h.state_mut().scale = Some(Scale {
        mm_per_point: 0.02,
        page,
        mm: 2.0,
        note: None,
    });
    h.run();
    let hash = h.state().rt.sourced.as_ref().unwrap().hash.clone();
    let files = ["Rev.kicad_sym", "rev.kicad_sch"].map(|f| dir.join(f));
    let before = files.clone().map(|f| std::fs::read_to_string(f).unwrap());
    // Accept asks first: each file, the field, old and new value; Esc writes nothing
    button(&h, ACCEPT).click();
    h.run();
    h.get_by_label("Rev.kicad_sym (library)");
    h.get_by_label("rev.kicad_sch (schematic copy)");
    assert_eq!(h.get_all_by_label(&format!("(none) → {hash}")).count(), 2);
    assert!(h.query_by_label_contains("open in KiCad").is_none());
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().rt.confirm.is_none());
    assert!(h.state().rt.accepted.is_none() && h.state().rt.written.is_none());
    assert_eq!(
        before,
        files.clone().map(|f| std::fs::read_to_string(f).unwrap())
    );
    // KiCad running, no lock file beside the files: no warning
    h.state_mut().rt.kicad_running = || true;
    let warned = |h: &mut Harness<'_, FootprintReview>| {
        button(h, ACCEPT).click();
        h.run();
        let w = h.query_by_label_contains("open in KiCad").is_some();
        h.key_press(egui::Key::Escape);
        h.run();
        w
    };
    assert!(!warned(&mut h), "KiCad running alone is no warning");
    // Stale locks from this computer: a dead process id; KiCad's own (no pid) with no KiCad running here
    let lock = dir.join("~rev.kicad_sch.lck");
    let host = this_host().unwrap_or_default();
    let record = |pid: Option<u64>| {
        let mut j = serde_json::json!({"username": "someone", "hostname": host});
        if let Some(p) = pid {
            j["pid"] = p.into();
        }
        j.to_string()
    };
    std::fs::write(&lock, "").unwrap();
    assert!(!warned(&mut h), "an empty lock is stale, as KiCad takes it");
    std::fs::write(&lock, record(Some(0x7fff_fff0))).unwrap();
    assert!(!warned(&mut h), "a dead process holds nothing");
    std::fs::write(&lock, record(None)).unwrap();
    h.state_mut().rt.kicad_running = || false;
    assert!(!warned(&mut h), "no KiCad here holds it");
    // Held: this process' id, then KiCad's own with KiCad running; the warning names the lock file. Enter writes
    std::fs::write(&lock, record(Some(std::process::id().into()))).unwrap();
    assert!(warned(&mut h), "a live process holds it");
    std::fs::write(&lock, record(None)).unwrap();
    h.state_mut().rt.kicad_running = || true;
    button(&h, ACCEPT).click();
    h.run();
    h.get_by_label_contains(&format!(
        "rev.kicad_sch is open in KiCad (its lock file ~rev.kicad_sch.lck, someone@{host})"
    ));
    assert!(h.query_by_label_contains("KiCad is running").is_none());
    assert!(h.query_by_label_contains("Rev.kicad_sym is open").is_none());
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(
        matches!(h.state().rt.written, Some(Ok(_))),
        "{:?}",
        h.state().rt.written
    );
    h.get_by_label("Rev:Mixed: written into");
    h.get_by_label("  Rev.kicad_sym (library)");
    h.get_by_label("  rev.kicad_sch (schematic copy)");
    let want = format!("(property \"Footprint_Review\" \"{hash}\"");
    for f in ["Rev.kicad_sym", "rev.kicad_sch"] {
        let text = std::fs::read_to_string(dir.join(f)).unwrap();
        assert!(text.contains(&want), "{f}: {text}");
    }
    let check = better_erc::fp::check(&dir, &better_erc::fp::LibrarySearch::default()).unwrap();
    for e in check.footprints.iter().filter(|e| e.symbol.is_some()) {
        assert!(e.is_ok(), "{e:?}");
    }
    assert!(
        signals
            .lock()
            .unwrap()
            .contains(&"app fp-review write".to_string())
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A harness on a review tab without fonts or library search
fn harness(name: &'static str) -> Harness<'static, FootprintReview> {
    let settings = FootprintReviewSettings {
        font_dirs: Vec::new(),
        library_search: Vec::new(),
        ..Default::default()
    };
    let mut cx = Context::new();
    Harness::builder()
        .with_size(vec2(1200.0, 1400.0))
        .build_ui_state(
            move |ui, state: &mut FootprintReview| state.ui(ui, &mut cx, Id::new(name)),
            FootprintReview::with(settings, Arc::new(|_: &str| {})),
        )
}

/// A WQFN-20 land pattern in `dir` shaped as U401's on b153a (3 x 3 mm, 0.4 mm pitch, a 1.7 mm exposed pad): pins
/// whose mask opening is the pad itself, an exposed pad with a mask margin of -0.05 mm and its back copper, four
/// paste windows inside it and five thermal vias
fn wqfn_footprint(dir: &Path) -> PathBuf {
    let pad = |n: usize, x: f64, y: f64, w: f64, h: f64| {
        format!(
            "  (pad \"{n}\" smd roundrect (at {x} {y}) (size {w} {h}) (layers \"F.Cu\" \"F.Mask\" \"F.Paste\") \
             (roundrect_rratio 0.25))\n"
        )
    };
    let mut text = String::from(
        "(footprint \"Test_WQFN-20-1EP_3x3mm_P0.4mm_EP1.7x1.7mm_ThermalVias\" (version 20241229) \
         (generator \"pcbnew\") (layer \"F.Cu\")\n  (attr smd)\n  (fp_rect (start -1.9 -1.9) (end 1.9 1.9) \
         (stroke (width 0.05) (type solid)) (fill no) (layer \"F.CrtYd\"))\n",
    );
    // Pins 1-5 down the left, 6-10 along the bottom, 11-15 up the right, 16-20 along the top
    for i in 0..5 {
        let d = -0.8 + 0.4 * i as f64;
        text += &pad(1 + i, -1.4, d, 0.7, 0.2);
        text += &pad(6 + i, d, 1.4, 0.2, 0.7);
        text += &pad(11 + i, 1.4, -d, 0.7, 0.2);
        text += &pad(16 + i, -d, -1.4, 0.2, 0.7);
    }
    text += "  (pad \"21\" smd rect (at 0 0) (size 1.7 1.7) (property pad_prop_heatsink) \
             (layers \"F.Cu\" \"F.Mask\") (solder_mask_margin -0.05))\n";
    text += "  (pad \"21\" smd rect (at 0 0) (size 1.7 1.7) (property pad_prop_heatsink) (layers \"B.Cu\"))\n";
    for (x, y) in [
        (-0.425, -0.425),
        (-0.425, 0.425),
        (0.425, -0.425),
        (0.425, 0.425),
    ] {
        text += &format!(
            "  (pad \"\" smd roundrect (at {x} {y}) (size 0.75 0.75) (layers \"F.Paste\") (roundrect_rratio 0.25))\n"
        );
    }
    for (x, y) in [(0.0, 0.0), (-0.6, 0.0), (0.6, 0.0), (0.0, -0.6), (0.0, 0.6)] {
        text += &format!(
            "  (pad \"21\" thru_hole circle (at {x} {y}) (size 0.5 0.5) (drill 0.2) \
             (property pad_prop_heatsink) (layers \"*.Cu\"))\n"
        );
    }
    text += ")\n";
    let file = dir.join("Test_WQFN-20-1EP.kicad_mod");
    std::fs::write(&file, text).unwrap();
    file
}

/// Picking Mask or Paste (the radio choice over copper, mask and paste) draws the pads' mask openings or paste
/// apertures on screen, over the pads, one at a time: checked on the rendered pixels of a real land pattern,
/// opaque, where an opening under its pad would not show at all
#[test]
fn mask_and_paste_shown_one_at_a_time() {
    let dir = std::env::temp_dir().join(format!("berc-fp-mask-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = datasheet(&dir);
    let mut h = harness("fp mask");
    h.run();
    let ctx = h.ctx.clone();
    let fp = wqfn_footprint(&dir);
    h.state_mut().open_path(&ctx, &fp);
    h.state_mut().open_datasheet(&ctx, &pdf);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    // 3 mm of footprint over 300 points, opaque, its middle on the page's paper
    let st = h.state_mut();
    st.scale = Some(Scale {
        mm_per_point: 0.01,
        page: 1,
        mm: 2.0,
        note: None,
    });
    st.opacity = Some(1.0);
    st.place.origin = [300.0, 400.0];
    let colors = LayerColors::default();
    // Points on the openings' outlines (footprint mm), each where no other shown line runs
    let probes = [
        // Pin 1's mask opening is the pad's own outline (no margin): its top edge
        (Kind::Mask, [-1.4, -0.9], colors.mask),
        // The exposed pad's mask opening, 0.05 mm inside it: its left edge between the paste windows
        (Kind::Mask, [-0.8, 0.0], colors.mask),
        // A paste window inside the exposed pad: its inner edge
        (Kind::Paste, [-0.05, -0.425], colors.paste),
        // Pin 1's paste aperture: its top edge
        (Kind::Paste, [-1.4, -0.9], colors.paste),
    ];
    // Whether a pixel within one point of `at` has colour `want`
    let near =
        |h: &Harness<'_, FootprintReview>, img: &image::RgbaImage, at: [f64; 2], want: Rgb| {
            let st = h.state();
            let p = st.to_screen(st.rt.canvas, st.fp_to_page(at));
            let ppp = h.ctx.pixels_per_point();
            let (x, y) = ((p.x * ppp) as i64, (p.y * ppp) as i64);
            (-2..=2).any(|dx| {
                (-2..=2).any(|dy| {
                    let px = img.get_pixel((x + dx) as u32, (y + dy) as u32).0;
                    px.iter().zip(want.0).all(|(g, w)| g.abs_diff(w) <= 24)
                })
            })
        };
    // At first (Copper picked): neither mask nor paste openings show
    let img = h.render().unwrap();
    for (k, at, want) in probes {
        assert!(!near(&h, &img, at, want), "{k:?} at {at:?} hidden at first");
    }
    for kind in [Kind::Mask, Kind::Paste] {
        h.get_by_label(kind.name()).click();
        h.run();
        let img = h.render().unwrap();
        if let Some(p) = std::env::var_os("BERC_FP_SNAPSHOT") {
            img.save(Path::new(&p).with_extension(format!("{}.png", kind.name())))
                .unwrap();
        }
        for (k, at, want) in probes {
            if k == kind {
                assert!(
                    near(&h, &img, at, want),
                    "{kind:?} at {at:?} shown once picked"
                );
            } else {
                assert!(
                    !near(&h, &img, at, want),
                    "{k:?} at {at:?} hidden while {kind:?} is the one picked"
                );
            }
        }
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A review of the WQFN-20 land pattern in `dir` over the test datasheet's land pattern page: 3 mm of footprint
/// over 100 points of blank paper below the drawing, opaque, 400 px across in the view
fn wqfn_review(name: &'static str, dir: &Path) -> Harness<'static, FootprintReview> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    let pdf = datasheet(dir);
    let mut h = harness(name);
    h.run();
    let ctx = h.ctx.clone();
    let fp = wqfn_footprint(dir);
    h.state_mut().open_path(&ctx, &fp);
    h.state_mut().open_datasheet(&ctx, &pdf);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    let st = h.state_mut();
    st.scale = Some(Scale {
        mm_per_point: 0.03,
        page: 1,
        mm: 2.0,
        note: None,
    });
    st.opacity = Some(1.0);
    st.place.origin = [150.0, 320.0];
    st.zoom = 4.0;
    st.pan = [-308.0, -988.0];
    h.run();
    h
}

/// The view picked (Copper, Mask or Paste by its button) and rendered, saved when `BERC_FP_SNAPSHOT` names a file
fn render_view(h: &mut Harness<'_, FootprintReview>, view: Kind) -> image::RgbaImage {
    h.get_by_label(view.name()).click();
    h.run();
    let img = h.render().unwrap();
    if let Some(p) = std::env::var_os("BERC_FP_SNAPSHOT") {
        img.save(Path::new(&p).with_extension(format!("{}.png", view.name())))
            .unwrap();
    }
    img
}

/// The rendered pixel at footprint point `at` (mm)
fn pixel_at(h: &Harness<'_, FootprintReview>, img: &image::RgbaImage, at: [f64; 2]) -> [u8; 3] {
    let st = h.state();
    let p = st.to_screen(st.rt.canvas, st.fp_to_page(at));
    let ppp = h.ctx.pixels_per_point();
    let px = img.get_pixel((p.x * ppp) as u32, (p.y * ppp) as u32).0;
    [px[0], px[1], px[2]]
}

/// The rendered pixels over the footprint's land pattern (3.2 mm square around its middle)
fn footprint_pixels(h: &Harness<'_, FootprintReview>, img: &image::RgbaImage) -> Vec<[u8; 3]> {
    let st = h.state();
    let ppp = h.ctx.pixels_per_point();
    let corner = |at: [f64; 2]| {
        let p = st.to_screen(st.rt.canvas, st.fp_to_page(at));
        ((p.x * ppp) as u32, (p.y * ppp) as u32)
    };
    let ((x0, y0), (x1, y1)) = (corner([-1.6, -1.6]), corner([1.6, 1.6]));
    let mut out = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let px = img.get_pixel(x, y).0;
            out.push([px[0], px[1], px[2]]);
        }
    }
    out
}

fn same_colour(got: [u8; 3], want: Rgb) -> bool {
    got.iter().zip(want.0).all(|(g, w)| g.abs_diff(w) <= 24)
}

/// Paste gray: a neutral gray near the paste colour (not the page's anti-aliased red edges, which are not neutral)
fn paste_gray(got: [u8; 3], paste: Rgb) -> bool {
    let spread = got.iter().max().unwrap() - got.iter().min().unwrap();
    spread <= 6 && got.iter().zip(paste.0).all(|(g, w)| g.abs_diff(w) <= 12)
}

/// Some points inside the land pattern's pads (mm), away from the holes and the pad numbers: pin 1 beside its
/// number, pin 11, the exposed pad inside a paste window and between the vias
const INSIDE_PADS: [[f64; 2]; 4] = [[-1.65, -0.8], [1.65, 0.8], [-0.425, -0.425], [0.3, 0.3]];

/// The Paste view shows the paste apertures alone, filled gray: between the exposed pad's four paste windows
/// the page shows, not a pad
#[test]
fn paste_view_shows_only_the_apertures() {
    let dir = std::env::temp_dir().join(format!("berc-fp-paste-{}", std::process::id()));
    let mut h = wqfn_review("fp paste", &dir);
    let colors = LayerColors::default();
    let img = render_view(&mut h, Kind::Paste);
    // Between the windows, on the exposed pad's copper: the page's white paper
    for at in [[0.0, 0.425], [-0.425, 0.0]] {
        let got = pixel_at(&h, &img, at);
        assert!(got.iter().all(|c| *c >= 225), "the page at {at:?}: {got:?}");
    }
    // Inside a window and inside pin 1's aperture: filled gray
    for at in [[-0.425, -0.425], [0.3, 0.3], [-1.65, -0.8]] {
        let got = pixel_at(&h, &img, at);
        assert!(paste_gray(got, colors.paste), "paste at {at:?}: {got:?}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The Mask view shows the mask openings alone, filled pink: no pixel over the land pattern has the copper colour
#[test]
fn mask_view_shows_only_the_openings() {
    let dir = std::env::temp_dir().join(format!("berc-fp-mask-view-{}", std::process::id()));
    let mut h = wqfn_review("fp mask view", &dir);
    let colors = LayerColors::default();
    let img = render_view(&mut h, Kind::Mask);
    let red = footprint_pixels(&h, &img)
        .into_iter()
        .filter(|p| same_colour(*p, colors.copper))
        .count();
    assert_eq!(red, 0, "pixels in the copper colour in the Mask view");
    for at in INSIDE_PADS {
        let got = pixel_at(&h, &img, at);
        assert!(same_colour(got, colors.mask), "mask at {at:?}: {got:?}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The Copper view shows the copper alone in the copper colour (front and back pads, vias): no pink or gray pixel
/// over the land pattern
#[test]
fn copper_view_is_copper_alone() {
    let dir = std::env::temp_dir().join(format!("berc-fp-copper-{}", std::process::id()));
    let mut h = wqfn_review("fp copper", &dir);
    let colors = LayerColors::default();
    let img = render_view(&mut h, Kind::Copper);
    let other = footprint_pixels(&h, &img)
        .into_iter()
        .filter(|p| same_colour(*p, colors.mask) || paste_gray(*p, colors.paste))
        .count();
    assert_eq!(other, 0, "pink or gray pixels in the Copper view");
    for at in INSIDE_PADS {
        let got = pixel_at(&h, &img, at);
        assert!(same_colour(got, colors.copper), "copper at {at:?}: {got:?}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
/// Invert page (I) inverts the page's own colours, not the footprint's: a blank spot goes from near white to
/// near black and back; the default comes from the settings
#[test]
fn invert_flips_the_page() {
    let dir = std::env::temp_dir().join(format!("berc-fp-invert-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = datasheet(&dir);
    let mut h = harness("fp invert");
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_datasheet(&ctx, &pdf);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    assert!(!h.state().page_inverted(), "off by default");
    // A blank corner of the land pattern page (bottom left, far from its text, dimension and rects)
    let ppp = h.ctx.pixels_per_point();
    let st = h.state();
    let at = st.to_screen(st.rt.canvas, [20.0, 380.0]);
    let at = pos2(at.x * ppp, at.y * ppp);
    let near = |img: &image::RgbaImage, want: u8| {
        let px = img.get_pixel(at.x as u32, at.y as u32).0;
        px[..3].iter().all(|g| g.abs_diff(want) <= 24)
    };
    let img = h.render().unwrap();
    assert!(near(&img, 255), "white before inverting");
    h.get_by_label("Invert page").click();
    h.run();
    assert!(h.state().page_inverted());
    let img = h.render().unwrap();
    assert!(near(&img, 0), "black once inverted");
    h.get_by_label("Invert page").click();
    h.run();
    assert!(!h.state().page_inverted());
    let img = h.render().unwrap();
    assert!(near(&img, 255), "white again");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Another part picked while a datasheet is open: its own datasheet opens, and a part without one leaves no
/// datasheet open (not the last part's)
#[test]
fn another_part_another_datasheet() {
    let dir = std::env::temp_dir().join(format!("berc-fp-switch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = datasheet(&dir);
    let other = dir.join("OTHER-PART.pdf");
    std::fs::copy(&pdf, &other).unwrap();
    footprint_table(&dir);
    let lib = dir.join("Rev.kicad_sym");
    std::fs::write(
        &lib,
        format!(
            "(kicad_symbol_lib (version 20231120) (generator \"kicad_symbol_editor\")\n{}{}{})\n",
            symbol_text(
                "Mixed",
                &[
                    ("Footprint", "Test:Mixed_Pads"),
                    ("Datasheet", &pdf.display().to_string())
                ]
            ),
            symbol_text(
                "Other",
                &[
                    ("Footprint", "Test:Mixed_Pads"),
                    ("Datasheet", &other.display().to_string())
                ]
            ),
            symbol_text("Sot", &[("Footprint", "Test:Asym_SOT")]),
        ),
    )
    .unwrap();
    let mut h = harness("fp switch");
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &lib);
    h.run();
    h.get_by_label("Rev:Mixed").click();
    h.run();
    h.run();
    assert_eq!(h.state().datasheet.as_deref(), Some(pdf.as_path()));
    assert!(h.state().rt.pdf.as_ref().is_some_and(|p| p.path == pdf));
    h.get_by_label("Rev:Other").click();
    h.run();
    h.run();
    assert_eq!(h.state().datasheet.as_deref(), Some(other.as_path()));
    assert!(h.state().rt.pdf.as_ref().is_some_and(|p| p.path == other));
    h.get_by_label("Rev:Sot").click();
    h.run();
    h.run();
    assert_eq!(h.state().datasheet, None);
    assert!(
        h.state().rt.pdf.is_none(),
        "the last part's datasheet is closed"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The ruler by keys: R at one end of the dimension and R at the other place it coarsely; Z zooms in on the first
/// end and a click puts it exactly, Z on the other end likewise; S goes back to the view and the length field, empty,
/// takes the typed length; Enter sets the scale and the footprint is the one to move again. F shows the page at
/// the view's full width. The help line names the keys.
#[test]
fn ruler_by_keys() {
    let dir = std::env::temp_dir().join(format!("berc-fp-keys-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = datasheet(&dir);
    let mut h = harness("fp keys");
    // The keys alone: the ends go where the pointer is, not to the drawing's lines
    h.state_mut().rt.settings.line_snap_px = 0.0;
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &footprint());
    h.state_mut().open_datasheet(&ctx, &pdf);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    assert_eq!(h.state().page, 1);
    let screen = |h: &Harness<'_, FootprintReview>, p: [f32; 2]| {
        let st = h.state();
        st.to_screen(st.rt.canvas, p)
    };
    let (a, b) = ([100.0, 200.0], [200.0, 200.0]);
    // Coarse: a few points off each end
    h.hover_at(screen(&h, [a[0] + 3.0, a[1] - 2.0]));
    h.run();
    h.key_press(egui::Key::R);
    h.run();
    assert_eq!(h.state().tool, Tool::Ruler);
    assert!(h.state().rt.ruler_placing);
    h.hover_at(screen(&h, [b[0] - 3.0, b[1] + 2.0]));
    h.run();
    h.key_press(egui::Key::R);
    h.run();
    assert!(!h.state().rt.ruler_placing);
    let coarse = h.state().ruler.unwrap();
    assert!((coarse[1][0] - (b[0] - 3.0)).abs() < 0.5, "{coarse:?}");
    // Exact: Z on each end, a click where it belongs
    let zoom = h.state().zoom;
    for (i, at) in [(0, a), (1, b)] {
        h.key_press(egui::Key::Z);
        h.run();
        assert_eq!(h.state().rt.ruler_focus, Some(i));
        assert!(h.state().zoom > zoom * 4.0, "zoomed in on end {i}");
        let p = screen(&h, at);
        assert!(h.state().rt.canvas.contains(p), "end {i} in view");
        h.hover_at(p);
        h.run();
        h.drag_at(p);
        h.drop_at(p);
        h.run();
        // The drop takes the pointer off the window: back over the canvas for the next key
        h.hover_at(p);
        h.run();
    }
    let [ra, rb] = h.state().ruler.unwrap();
    for (got, want) in [(ra, a), (rb, b)] {
        assert!(
            (got[0] - want[0]).abs() < 0.1 && (got[1] - want[1]).abs() < 0.1,
            "{ra:?} {rb:?}"
        );
    }
    // S: back to the view, the length typed straight away into the box in the middle of the canvas, emptied
    h.state_mut().rt.ruler_mm = "9".into();
    h.key_press(egui::Key::S);
    h.run();
    h.run();
    assert_eq!(h.state().rt.ruler_focus, None);
    assert!((h.state().zoom - zoom).abs() < 1e-3);
    assert!(h.state().rt.length_box, "the length box asks");
    assert!(h.state().rt.typing_mm, "the length field has the keyboard");
    assert!(
        h.state()
            .ruler_hint()
            .unwrap()
            .contains("Enter sets the scale")
    );
    h.event(egui::Event::Text("2.00".into()));
    h.key_press(egui::Key::Enter);
    h.run();
    let scale = h.state().scale.clone().expect("the scale is set");
    assert!((scale.mm_per_point - 0.02).abs() < 1e-4, "{scale:?}");
    assert_eq!(h.state().tool, Tool::Move);
    // F: the page across the view's width
    h.hover_at(h.state().rt.canvas.center());
    h.run();
    h.key_press(egui::Key::F);
    h.run();
    let st = h.state();
    let w = st.rt.pdf.as_ref().unwrap().page_size(st.page).unwrap()[0];
    assert!((w * st.zoom - st.rt.canvas.width()).abs() < 0.5);
    for k in [
        "F full width",
        "R ruler",
        "Z zoom on a ruler end",
        "S type the ruler's length",
        "D the dimension's number sets it",
        "T turn 90°",
    ] {
        assert!(st.help_line().contains(k), "{k}: {}", st.help_line());
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// S asks the ruler's length in a box in the middle of the canvas, the number typed shown large and in focus there
/// (not in the side panel): typing 1.6 and Enter sets the scale; Esc leaves the box without one
#[test]
fn ruler_length_box() {
    let dir = std::env::temp_dir().join(format!("berc-fp-length-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = datasheet(&dir);
    let mut h = harness("fp length");
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &footprint());
    h.state_mut().open_datasheet(&ctx, &pdf);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    // The ruler along the dimension, 100 points
    h.state_mut().ruler = Some([[100.0, 200.0], [200.0, 200.0]]);
    let over_canvas = |h: &mut Harness<'_, FootprintReview>| {
        h.hover_at(h.state().rt.canvas.center());
        h.run();
    };
    // Esc: no scale, the box gone
    over_canvas(&mut h);
    h.key_press(egui::Key::S);
    h.run();
    assert!(h.state().rt.length_box);
    h.event(egui::Event::Text("3".into()));
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(!h.state().rt.length_box);
    assert!(h.state().scale.is_none(), "Esc sets no scale");
    // S, 1.6, Enter: the scale
    over_canvas(&mut h);
    h.key_press(egui::Key::S);
    h.run();
    h.run();
    let canvas = h.state().rt.canvas;
    // The text field over the canvas, not the side panel's
    let at = h
        .query_all_by_role(egui::accesskit::Role::TextInput)
        .map(|n| n.rect())
        .find(|r| canvas.contains(r.center()))
        .expect("a text field over the canvas");
    assert!(
        (at.center() - canvas.center()).length() < 120.0,
        "the length field in the middle of the canvas: {at:?} in {canvas:?}"
    );
    assert!(at.height() >= 40.0, "the number typed large: {at:?}");
    h.event(egui::Event::Text("1.6".into()));
    h.run();
    assert_eq!(h.state().rt.ruler_mm, "1.6");
    if let Some(p) = std::env::var_os("BERC_FP_SNAPSHOT") {
        h.render().unwrap().save(p).unwrap();
    }
    h.key_press(egui::Key::Enter);
    h.run();
    let scale = h.state().scale.clone().expect("the scale is set");
    assert!((scale.mm_per_point - 0.016).abs() < 1e-6, "{scale:?}");
    assert!(!h.state().rt.length_box);
    assert_eq!(h.state().tool, Tool::Move);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The ruler snaps level: the left end of a horizontal ruler dragged diagonally goes where it is dragged and the
/// other end takes its Y; Shift held lets the end go at any angle, the other staying
#[test]
fn ruler_snaps_level() {
    let dir = std::env::temp_dir().join(format!("berc-fp-snap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = datasheet(&dir);
    let mut h = harness("fp snap");
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_datasheet(&ctx, &pdf);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    let screen = |h: &Harness<'_, FootprintReview>, p: [f32; 2]| {
        let st = h.state();
        st.to_screen(st.rt.canvas, p)
    };
    let drag = |h: &mut Harness<'_, FootprintReview>, from: [f32; 2], to: [f32; 2], shift: bool| {
        let (a, b) = (screen(h, from), screen(h, to));
        let m = if shift {
            egui::Modifiers::SHIFT
        } else {
            egui::Modifiers::NONE
        };
        h.event(egui::Event::ModifiersChanged(m));
        h.hover_at(a);
        h.run();
        h.drag_at(a);
        h.run();
        for t in [0.3, 0.7, 1.0] {
            h.event(egui::Event::PointerMoved(a + (b - a) * t));
            h.run();
        }
        h.drop_at(b);
        h.run();
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run();
    };
    h.state_mut().tool = Tool::Ruler;
    h.state_mut().ruler = Some([[100.0, 200.0], [200.0, 200.0]]);
    h.run();
    drag(&mut h, [100.0, 200.0], [110.0, 230.0], false);
    let [a, b] = h.state().ruler.unwrap();
    assert!(
        (a[0] - 110.0).abs() < 0.5 && (a[1] - 230.0).abs() < 0.5,
        "{a:?}"
    );
    assert!(
        (b[0] - 200.0).abs() < 0.5,
        "the other end keeps its X: {b:?}"
    );
    assert_eq!(a[1], b[1], "both ends keep the same Y");
    // Shift: any angle
    drag(&mut h, [110.0, 230.0], [100.0, 250.0], true);
    let [a, b] = h.state().ruler.unwrap();
    assert!((a[1] - 250.0).abs() < 0.5, "{a:?}");
    assert!((b[1] - 230.0).abs() < 0.5, "the other end stays: {b:?}");
    assert!(
        h.state()
            .help_line()
            .contains("the ruler snaps level, Shift: any angle")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The ruler snaps to the drawing's lines: over the dimension's corner the target shows before R; R there puts the
/// first end on the corner exactly, and the other, a few points off the far corner, goes onto it; the number the
/// dimension prints is offered as a chip, and D sets the scale from it, as does a click on the chip. Alt held
/// leaves an end where the pointer is.
#[test]
fn ruler_snaps_to_the_drawing() {
    let dir = std::env::temp_dir().join(format!("berc-fp-lines-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = datasheet(&dir);
    let mut h = harness("fp lines");
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_datasheet(&ctx, &pdf);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    assert_eq!(h.state().page, 1);
    // 2 screen points per page point: the snap's 10 pixels reach 5 points
    h.state_mut().zoom = 2.0;
    h.state_mut().pan = [0.0, 0.0];
    h.state_mut().tool = Tool::Ruler;
    h.run();
    let screen = |h: &Harness<'_, FootprintReview>, p: [f32; 2]| {
        let st = h.state();
        st.to_screen(st.rt.canvas, p)
    };
    let (a, b) = ([100.0, 200.0], [200.0, 200.0]);
    h.hover_at(screen(&h, [a[0] + 3.0, a[1] - 2.0]));
    h.run();
    let snap = h
        .state()
        .rt
        .snap
        .clone()
        .expect("the target shows before R");
    assert_eq!(snap.at, a);
    h.key_press(egui::Key::R);
    h.run();
    h.hover_at(screen(&h, [b[0] - 3.0, b[1] + 2.0]));
    h.run();
    assert_eq!(
        h.state().ruler.unwrap()[1],
        b,
        "the other end follows onto the corner"
    );
    if let Some(p) = std::env::var_os("BERC_FP_SNAPSHOT") {
        h.render().unwrap().save(p).unwrap();
    }
    h.key_press(egui::Key::R);
    h.run();
    assert_eq!(h.state().ruler, Some([a, b]));
    // The dimension's number: D sets the scale from it
    assert_eq!(h.state().dimension_number().unwrap().text, "2.00");
    h.key_press(egui::Key::D);
    h.run();
    let scale = h.state().scale.clone().expect("the scale is set");
    assert!((scale.mm_per_point - 0.02).abs() < 1e-6, "{scale:?}");
    assert_eq!(scale.mm, 2.0);
    assert_eq!(h.state().tool, Tool::Move);
    // A click on the chip does the same
    h.state_mut().scale = None;
    h.run();
    h.get_by_label("2.00 mm · D").click();
    h.run();
    assert!(h.state().scale.is_some(), "the chip sets the scale");
    // Alt: the end where the pointer is
    h.state_mut().tool = Tool::Ruler;
    let off = [a[0] + 3.0, a[1] - 2.0];
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::ALT));
    h.hover_at(screen(&h, off));
    h.run();
    assert!(h.state().rt.snap.is_none(), "no target with Alt");
    h.key_press(egui::Key::R);
    h.run();
    let first = h.state().ruler.unwrap()[0];
    assert!(
        (first[0] - off[0]).abs() < 0.01 && (first[1] - off[1]).abs() < 0.01,
        "{first:?}"
    );
    assert!(
        h.state()
            .help_line()
            .contains("its ends snap to the drawing's lines, Alt: free")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A part without a Datasheet field: the library search runs for its part number and the first datasheet it finds
/// opens by itself, shown in the found panel as the one open
#[test]
fn library_search_opens_the_first_hit() {
    let dir = std::env::temp_dir().join(format!("berc-fp-search-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = datasheet(&dir);
    footprint_table(&dir);
    let lib = dir.join("Rev.kicad_sym");
    std::fs::write(
        &lib,
        format!(
            "(kicad_symbol_lib (version 20231120) (generator \"kicad_symbol_editor\")\n{})\n",
            symbol_text(
                "Sot",
                &[("Footprint", "Test:Asym_SOT"), ("MPN", "TEST-PART")]
            ),
        ),
    )
    .unwrap();
    // A library search of the test's own, in the process: the test datasheet for any words, the words kept
    let asked: Arc<Mutex<Vec<String>>> = Arc::default();
    let searcher: Searcher = {
        let (pdf, asked) = (pdf.clone(), asked.clone());
        Arc::new(move |words: &[String]| {
            asked.lock().unwrap().push(words.join(" "));
            Ok(vec![LibraryHit {
                path: pdf.clone(),
                page: None,
            }])
        })
    };
    let mut h = harness("fp search");
    h.state_mut().rt.searcher = Some(searcher);
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &lib);
    h.run();
    h.get_by_label("Rev:Sot").click();
    h.run();
    assert_eq!(h.state().rt.query, "TEST-PART");
    // Until the search has answered and the tab has taken the answer: the search thread may answer within a run
    // or after it, so wait on its reply when it is still out
    while let Some(rx) = h.state_mut().rt.searching.take() {
        let reply = rx.recv().expect("the search answers");
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(reply).unwrap();
        h.state_mut().rt.searching = Some(rx);
        h.run();
    }
    h.run();
    assert_eq!(
        asked.lock().unwrap().first().map(String::as_str),
        Some("TEST-PART")
    );
    assert_eq!(h.state().datasheet.as_deref(), Some(pdf.as_path()));
    h.get_by_label("Found in the library (1), the first opens by itself:");
    h.get_by_label("📄 TEST-PART.pdf  (open)");
    std::fs::remove_dir_all(&dir).unwrap();
}
