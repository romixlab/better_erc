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
    // Accept writes the review hash into the library symbol
    assert!(!disabled(&h, ACCEPT));
    button(&h, ACCEPT).click();
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
/// PATH. Not run without them. BERC_FP_BLEND picks a blend mode.
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
    h.state_mut().select_symbol(i, true);
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
            st.colors = Some(LayerColors {
                pads: Rgb(rgb),
                ..Default::default()
            });
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
    button(&h, ACCEPT).click();
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
