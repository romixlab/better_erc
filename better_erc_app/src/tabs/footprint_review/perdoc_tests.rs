//! Each datasheet of a symbol keeps its own review: the view (page, scale, placement, ruler, model) is kept per
//! symbol and datasheet, so switching to another datasheet and back finds the first as it was left; Accept on one
//! datasheet writes that datasheet's review line and leaves the others' lines as they are

use super::panel::ACCEPT;
use super::testpdf::{TestPage, write};
use super::tests::{datasheet, footprint_table, harness, symbol_text};
use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

/// Another maker's datasheet for the same footprint, a series document naming both MPNs: its land pattern on page
/// 2 at another scale (a 2.00 mm dimension 50 points long, 0.04 mm per point), on a larger page
fn other_datasheet(dir: &Path) -> PathBuf {
    let text = |x: f32, y: f32, s: f32, t: &str| (x, y, s, t.to_string());
    let pages = [
        TestPage {
            texts: vec![
                text(50.0, 60.0, 20.0, "OTHER-PART series"),
                text(50.0, 90.0, 12.0, "Second source for TEST-PART"),
            ],
            ..Default::default()
        },
        TestPage {
            texts: vec![
                text(50.0, 60.0, 14.0, "Recommended land pattern"),
                text(115.0, 290.0, 10.0, "2.00"),
            ],
            lines: vec![[100.0, 300.0, 150.0, 300.0]],
            ..Default::default()
        },
    ];
    let file = dir.join("OTHER-PART.pdf");
    std::fs::write(&file, write([500.0, 700.0], &pages)).unwrap();
    file
}

/// Opens `pdf` as a person would and waits for it to be read
fn open(h: &mut Harness<'_, FootprintReview>, pdf: &Path) {
    let ctx = h.ctx.clone();
    h.state_mut().open_datasheet(&ctx, pdf);
    h.step();
    if let Some(p) = h.state_mut().rt.pdf.as_mut() {
        p.wait_idle(&ctx);
    }
    h.run();
}

/// Aligns the open datasheet's drawing by hand: its page, scale and the footprint's place
fn align(h: &mut Harness<'_, FootprintReview>, page: usize, mm_per_point: f64, place: &Placement) {
    let st = h.state_mut();
    st.page = page;
    st.rt.auto_page = false;
    st.scale = Some(Scale {
        mm_per_point,
        page,
        mm: 2.0,
        note: None,
    });
    st.place = place.clone();
    h.run();
}

/// Accept, with the MPNs as the question ticks them; returns them as ticked
fn accept(h: &mut Harness<'_, FootprintReview>) -> Vec<(String, bool)> {
    h.get_by_label(ACCEPT).click();
    h.run();
    let mpns = h
        .state()
        .rt
        .confirm
        .as_ref()
        .expect("Accept asks first")
        .mpns
        .iter()
        .map(|m| (m.mpn.clone(), m.ticked))
        .collect();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(
        matches!(h.state().rt.written, Some(Ok(_))),
        "{:?}",
        h.state().rt.written
    );
    mpns
}

/// The symbol's review lines in the library file, in order
fn lines(lib: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(lib).unwrap();
    text.lines()
        .filter_map(|l| {
            let rest = l.trim().strip_prefix("(property \"Footprint_Review")?;
            let value = rest.split_once("\" \"")?.1;
            Some(value.split_once('"')?.0.to_string())
        })
        .collect()
}

/// Two datasheets of one symbol with two MPNs, each at its own scale: aligned and accepted on A, then on B, then
/// back to A. A's view and line are as A left them, B's line is its own, and the symbol has a line per datasheet.
/// Work not accepted on a datasheet survives switching away and back, and is saved with the tab
#[test]
fn each_datasheet_keeps_its_own_review() {
    let dir = std::env::temp_dir().join(format!("berc-fp-perdoc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("lib/Test")).unwrap();
    let a = datasheet(&dir.join("lib/Test"));
    let b = other_datasheet(&dir.join("lib/Test"));
    footprint_table(&dir);
    let lib = dir.join("Rev.kicad_sym");
    std::fs::write(
        &lib,
        format!(
            "(kicad_symbol_lib (version 20231120) (generator \"kicad_symbol_editor\")\n{})\n",
            symbol_text(
                "Two",
                &[
                    ("Footprint", "Test:Mixed_Pads"),
                    ("Mpn1", "TEST-PART"),
                    ("Mpn2", "OTHER-PART"),
                ]
            ),
        ),
    )
    .unwrap();
    let mut h = harness("fp perdoc");
    h.state_mut().rt.settings.reviewer = "tester".into();
    h.state_mut().rt.indexer = Some(Arc::new(|| {
        docindex::parse_index(
            r#"[{"path": "Test/TEST-PART.pdf", "doc": "DS99", "rev": "3",
                 "hash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"},
                {"path": "Test/OTHER-PART.pdf", "doc": "OT-7", "rev": "1",
                 "hash": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"}]"#,
        )
    }));
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &lib);
    h.run();
    h.get_by_label("Rev:Two").click();
    h.run();
    h.run();
    let hash = h.state().rt.sourced.as_ref().unwrap().hash.clone();

    // A: aligned and accepted for TEST-PART
    open(&mut h, &a);
    let place_a = Placement {
        origin: [150.0, 230.0],
        rotation: 90.0,
        mirror: false,
    };
    align(&mut h, 1, 0.02, &place_a);
    h.state_mut().ruler = Some([[100.0, 200.0], [200.0, 200.0]]);
    assert_eq!(
        accept(&mut h),
        [
            ("TEST-PART".to_string(), true),
            ("OTHER-PART".to_string(), false)
        ]
    );
    let line_a = format!(
        "{hash} mpn=TEST-PART doc=DS99@rev3 pdf=b3:0123456789abcdef page=2 scale=0.02 at=150,230,90 by=tester on="
    );
    let after_a = lines(&lib);
    assert_eq!(after_a.len(), 1, "{after_a:?}");
    assert!(after_a[0].starts_with(&line_a), "{after_a:?}");

    // B: a fresh view, nothing of A's carried over
    open(&mut h, &b);
    {
        let st = h.state();
        assert_eq!(st.datasheet.as_deref(), Some(b.as_path()));
        assert_eq!(st.scale, None, "A's scale carried over to B");
        assert_eq!(st.ruler, None, "A's ruler carried over to B");
        assert_ne!(st.place, place_a, "A's placement carried over to B");
    }
    // Aligned on B at its own scale and accepted: B names TEST-PART too, but A's line has it, so only OTHER-PART
    // is ticked
    let place_b = Placement {
        origin: [300.0, 420.0],
        rotation: 0.0,
        mirror: false,
    };
    align(&mut h, 1, 0.04, &place_b);
    assert_eq!(
        accept(&mut h),
        [
            ("TEST-PART".to_string(), false),
            ("OTHER-PART".to_string(), true)
        ]
    );
    let after_b = lines(&lib);
    assert_eq!(after_b.len(), 2, "one line per datasheet: {after_b:?}");
    assert_eq!(after_b[0], after_a[0], "accepting on B rewrote A's line");
    assert!(
        after_b[1].starts_with(&format!(
            "{hash} mpn=OTHER-PART doc=OT-7@rev1 pdf=b3:fedcba9876543210 page=2 scale=0.04 at=300,420,0 by=tester on="
        )),
        "{after_b:?}"
    );

    // More alignment on B, not accepted
    let place_b2 = Placement {
        origin: [310.0, 425.0],
        rotation: 0.0,
        mirror: false,
    };
    h.state_mut().place = place_b2.clone();
    h.run();

    // Back to A: as A was left
    open(&mut h, &a);
    {
        let st = h.state();
        assert_eq!(st.page, 1);
        assert_eq!(
            st.scale.as_ref().map(|s| (s.mm_per_point, s.page)),
            Some((0.02, 1))
        );
        assert_eq!(st.place, place_a);
        assert_eq!(st.ruler, Some([[100.0, 200.0], [200.0, 200.0]]));
    }
    assert_eq!(lines(&lib), after_b, "switching rewrote a line");

    // Back to B: the work not accepted is still there
    open(&mut h, &b);
    {
        let st = h.state();
        assert_eq!(st.page, 1);
        assert_eq!(st.scale.as_ref().map(|s| s.mm_per_point), Some(0.04));
        assert_eq!(st.place, place_b2);
    }
    // Accepting on B again replaces B's line alone
    assert_eq!(
        accept(&mut h),
        [
            ("TEST-PART".to_string(), false),
            ("OTHER-PART".to_string(), true)
        ]
    );
    let again = lines(&lib);
    assert_eq!(again.len(), 2, "{again:?}");
    assert_eq!(again[0], after_a[0]);
    assert!(again[1].contains(" at=310,425,0 "), "{again:?}");

    // Saved with the tab: A's view comes back from the saved state too
    let saved = serde_json::to_string(h.state()).unwrap();
    let back: FootprintReview = serde_json::from_str(&saved).unwrap();
    let kept = back
        .views
        .iter()
        .find(|v| v.datasheet == a)
        .expect("A's view saved");
    assert_eq!(kept.place, place_a);
    assert_eq!(kept.symbol, "Rev:Two");
    std::fs::remove_dir_all(&dir).unwrap();
}
