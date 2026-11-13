//! Files changed on disk while the review is open (VIEW-31): the symbol and its footprint shown as they are on disk
//! now, read again by the Reload button and key, by picking a symbol again, and by the file watcher on its own;
//! the view kept, the review state worked out again; what Accept wrote reloads nothing

use super::panel::ACCEPT;
use super::tests::datasheet;
use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use std::time::{Duration, Instant};

const FIXTURES: &str = "../erc_kicad/tests/fixtures/footprints/Test.pretty";

/// A library of its own in `dir` the test may change: `Test.pretty` with Mixed_Pads and Asym_SOT, through a
/// `fp-lib-table`; `Rev.kicad_sym` with `Mixed` (reviewed: Mixed_Pads' hash now) and `Sot`, both naming the
/// datasheet `pdf`. Returns the library and Mixed_Pads' file
fn library(dir: &Path, pdf: &Path) -> (PathBuf, PathBuf) {
    let pretty = dir.join("Test.pretty");
    std::fs::create_dir_all(&pretty).unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES);
    for f in ["Mixed_Pads.kicad_mod", "Asym_SOT.kicad_mod"] {
        std::fs::copy(fixtures.join(f), pretty.join(f)).unwrap();
    }
    std::fs::write(
        dir.join("fp-lib-table"),
        "(fp_lib_table (version 7)\n  (lib (name \"Test\")(type \"KiCad\")(uri \"${KIPRJMOD}/Test.pretty\")(options \"\")(descr \"\")))\n",
    )
    .unwrap();
    let mixed = pretty.join("Mixed_Pads.kicad_mod");
    let hash = better_erc::fp::hash_file(&mixed, None).unwrap().hash;
    let symbol = |name: &str, fp: &str, review: Option<&str>| {
        let mut props = vec![
            ("Footprint", fp.to_string()),
            ("Datasheet", pdf.display().to_string()),
        ];
        props.extend(review.map(|r| ("Footprint_Review", r.to_string())));
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
    };
    let lib = dir.join("Rev.kicad_sym");
    std::fs::write(
        &lib,
        format!(
            "(kicad_symbol_lib (version 20231120) (generator \"kicad_symbol_editor\")\n{}{})\n",
            symbol("Mixed", "Test:Mixed_Pads", Some(&hash)),
            symbol("Sot", "Test:Asym_SOT", None),
        ),
    )
    .unwrap();
    (lib, mixed)
}

/// The footprint file changed as the footprint editor would: pad `n` added
fn add_pad(file: &Path, n: u32) {
    let text = std::fs::read_to_string(file).unwrap();
    let at = text.find("\t(pad \"1\"").expect("a first pad");
    let pad = format!(
        "\t(pad \"{n}\" smd rect (at {x} 4) (size 1 1) (layers \"F.Cu\" \"F.Paste\" \"F.Mask\"))\n",
        x = n
    );
    std::fs::write(file, format!("{}{pad}{}", &text[..at], &text[at..])).unwrap();
}

/// A frame, the datasheet's worker waited for, then frames until nothing more is asked. Reading files again sends
/// the worker work whose replies come back one by one, each asking a frame; under load they straddle more frames
/// than `run` steps, so the test waits for the worker to be idle first, as `open` does
fn settle(h: &mut Harness<'static, FootprintReview>) {
    h.step();
    let ctx = h.ctx.clone();
    if let Some(pdf) = h.state_mut().rt.pdf.as_mut() {
        pdf.wait_idle(&ctx);
    }
    h.run();
}

fn pads(h: &Harness<'_, FootprintReview>) -> Vec<String> {
    let s = h.state().rt.sourced.as_ref().expect("a footprint shown");
    s.footprint.pads.iter().map(|p| p.number.clone()).collect()
}

fn status(h: &Harness<'_, FootprintReview>, symbol: &str) -> SymbolStatus {
    let st = h.state();
    st.rt.symbols.as_ref().unwrap().rows[st.symbol_row(symbol).unwrap()].status
}

fn harness(name: &'static str, watch: bool) -> Harness<'static, FootprintReview> {
    let settings = FootprintReviewSettings {
        font_dirs: Vec::new(),
        library_search: Vec::new(),
        library_name: Vec::new(),
        reviewer: "tester".into(),
        ..Default::default()
    };
    let mut state = FootprintReview::with(settings, Arc::new(|_: &str| {}));
    state.rt.watch_files = watch;
    let mut cx = Context::new();
    Harness::builder()
        .with_size(vec2(1200.0, 1400.0))
        .build_ui_state(
            move |ui, state: &mut FootprintReview| state.ui(ui, &mut cx, Id::new(name)),
            state,
        )
}

/// The library opened, `Rev:Mixed` under review with its datasheet, the scale set and the footprint moved off its
/// first place: the view a reload must keep
fn open(h: &mut Harness<'static, FootprintReview>, lib: &Path) {
    settle(h);
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, lib);
    settle(h);
    h.get_by_label("Rev:Mixed").click();
    settle(h);
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    settle(h);
    let page = h.state().page;
    let st = h.state_mut();
    st.scale = Some(Scale {
        mm_per_point: 0.02,
        page,
        mm: 2.0,
        note: None,
    });
    st.place.origin = [123.0, 45.0];
    st.place.rotation = 90.0;
    settle(h);
    assert_eq!(status(h, "Rev:Mixed"), SymbolStatus::Ok);
}

/// The view as a reload must leave it: the page, the scale, where the footprint is
fn view(h: &Harness<'_, FootprintReview>) -> (usize, f64, [f32; 2], f32) {
    let st = h.state();
    (
        st.page,
        st.scale.as_ref().map_or(0.0, |s| s.mm_per_point),
        st.place.origin,
        st.place.rotation,
    )
}

/// The footprint changed on disk: Reload (the button, then the key) shows the new pad and the review stale, the
/// view as it was, and says what it did; picking another symbol and back shows the file as it is now without it;
/// the datasheet read again keeps its page and scale
#[test]
fn reload_shows_the_footprint_as_it_is_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = datasheet(dir.path());
    let (lib, mixed) = library(dir.path(), &pdf);
    let mut h = harness("fp reload", false);
    open(&mut h, &lib);
    let before = view(&h);
    assert!(!pads(&h).contains(&"7".to_string()));
    let hash = h.state().rt.sourced.as_ref().unwrap().hash.clone();

    // Not watched in this test: nothing changes until asked
    add_pad(&mixed, 7);
    settle(&mut h);
    assert!(!pads(&h).contains(&"7".to_string()));
    h.get_by_label_contains("Reload · F5").click();
    settle(&mut h);
    assert!(pads(&h).contains(&"7".to_string()), "{:?}", pads(&h));
    assert_ne!(h.state().rt.sourced.as_ref().unwrap().hash, hash);
    assert_eq!(status(&h, "Rev:Mixed"), SymbolStatus::Changed);
    assert_eq!(view(&h), before, "the view stays");
    assert_eq!(h.state().symbol.as_deref(), Some("Rev:Mixed"));
    h.get_by_label_contains("Reloaded everything at ");
    // The datasheet was read again, its page and scale kept
    let ctx = h.ctx.clone();
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    settle(&mut h);
    assert_eq!(view(&h), before);

    // The key, over the canvas
    add_pad(&mixed, 8);
    let canvas = h.state().rt.canvas;
    h.hover_at(canvas.center());
    settle(&mut h);
    h.key_press(egui::Key::F5);
    settle(&mut h);
    assert!(pads(&h).contains(&"8".to_string()), "{:?}", pads(&h));
    assert_eq!(view(&h), before);

    // Another symbol, the footprint changed meanwhile, back: as it is on disk now, no Reload needed
    h.get_by_label("Rev:Sot").click();
    settle(&mut h);
    add_pad(&mixed, 9);
    h.get_by_label("Rev:Mixed").click();
    settle(&mut h);
    assert!(pads(&h).contains(&"9".to_string()), "{:?}", pads(&h));
    h.get_by_label_contains("the symbols and footprints changed on disk: reloaded at ");
}

/// What Accept writes into the library symbol is no change to read again: the watcher's look finds nothing, the
/// symbol stays reviewed as Accept left it
#[test]
fn accept_reloads_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = datasheet(dir.path());
    let (lib, mixed) = library(dir.path(), &pdf);
    let mut h = harness("fp reload accept", false);
    open(&mut h, &lib);
    // A stale review to accept again
    add_pad(&mixed, 7);
    h.get_by_label_contains("Reload · F5").click();
    settle(&mut h);
    assert_eq!(status(&h, "Rev:Mixed"), SymbolStatus::Changed);
    let text = std::fs::read_to_string(&lib).unwrap();
    h.get_by_label(ACCEPT).click();
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(
        matches!(h.state().rt.written, Some(Ok(_))),
        "{:?}",
        h.state().rt.written
    );
    assert_ne!(std::fs::read_to_string(&lib).unwrap(), text, "written");
    assert_eq!(h.state_mut().rt.watch.changed(), []);
    assert_eq!(status(&h, "Rev:Mixed"), SymbolStatus::Ok);
    // Picking it again (the list's row; the label is in what Accept wrote too) reads nothing again
    let i = h.state().symbol_row("Rev:Mixed").unwrap();
    h.state_mut().select_symbol(i, true);
    settle(&mut h);
    assert!(
        h.state()
            .rt
            .reloaded
            .as_deref()
            .unwrap()
            .starts_with("Reloaded everything")
    );
}

/// The watcher on: a footprint saved in the editor is read again by itself, once the file stayed quiet, the view
/// kept. Waits for the reload (a condition), bounded so a lost event fails
#[test]
fn a_change_on_disk_reloads_by_itself() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = datasheet(dir.path());
    let (lib, mixed) = library(dir.path(), &pdf);
    let mut h = harness("fp reload watched", true);
    open(&mut h, &lib);
    assert!(
        h.state().rt.watch.error.is_none(),
        "{:?}",
        h.state().rt.watch.error
    );
    assert!(
        h.state()
            .rt
            .watch
            .files(watch::Group::List)
            .contains(&mixed)
    );
    let before = view(&h);
    // Saved the way KiCad saves: a new file renamed over the old
    let tmp = dir.path().join("Mixed_Pads.kicad_mod.tmp");
    std::fs::copy(&mixed, &tmp).unwrap();
    add_pad(&tmp, 7);
    std::fs::rename(&tmp, &mixed).unwrap();
    let until = Instant::now() + Duration::from_secs(20);
    while !pads(&h).contains(&"7".to_string()) {
        assert!(Instant::now() < until, "not reloaded: {:?}", pads(&h));
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(status(&h, "Rev:Mixed"), SymbolStatus::Changed);
    assert_eq!(view(&h), before);
    settle(&mut h);
    h.get_by_label_contains("the symbols and footprints changed on disk: reloaded at ");
}

/// A project in `dir` around placed.kicad_pcb whose Mixed_Pads footprints U7-U12 are placed for `Rev:Mixed` as
/// `Test:Mixed_Pads`, while the library symbol's own `Footprint` is `Test:Asym_SOT`: an override. The library is
/// [`library`]'s; returns its Mixed_Pads file
fn override_project(dir: &Path) -> PathBuf {
    let pdf = datasheet(dir);
    let (lib, mixed) = library(dir, &pdf);
    let fixtures =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../erc_kicad/tests/fixtures/footprints");
    let board = std::fs::read_to_string(fixtures.join("placed.kicad_pcb"))
        .unwrap()
        .replace(
            "(footprint \"Mixed_Pads\"",
            "(footprint \"Test:Mixed_Pads\"",
        );
    std::fs::write(dir.join("rev.kicad_pcb"), board).unwrap();
    // The library symbol names its own footprint, not the one the board places
    let text = std::fs::read_to_string(&lib).unwrap();
    let own = text.replacen(
        "(property \"Footprint\" \"Test:Mixed_Pads\"",
        "(property \"Footprint\" \"Test:Asym_SOT\"",
        1,
    );
    std::fs::write(&lib, own).unwrap();
    let root = "b0000000-0000-4000-8000-000000000000";
    let lib_symbol = "(symbol \"Rev:Mixed\" (in_bom yes) (on_board yes)\n    (property \"Reference\" \"U\" (at 0 0 0) (effects (font (size 1.27 1.27))))\n    (property \"Footprint\" \"Test:Asym_SOT\" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))\n    (symbol \"Mixed_1_1\" (pin passive line (at 0 0 0) (length 1.27) (name \"A\" (effects (font (size 1.27 1.27)))) (number \"1\" (effects (font (size 1.27 1.27)))))))\n";
    let placed: String = (7..=12)
        .map(|n| {
            format!(
                "  (symbol (lib_id \"Rev:Mixed\") (at {x} 50 0) (unit 1) (in_bom yes) (on_board yes) (dnp no) (uuid \"b0000000-0000-4000-8000-0000000001{n:02}\")\n    (property \"Reference\" \"U{n}\" (at {x} 45 0) (effects (font (size 1.27 1.27))))\n    (property \"Footprint\" \"Test:Mixed_Pads\" (at {x} 50 0) (effects (font (size 1.27 1.27)) (hide yes)))\n    (pin \"1\" (uuid \"b0000000-0000-4000-8000-0000000002{n:02}\"))\n    (instances (project \"rev\" (path \"/{root}\" (reference \"U{n}\") (unit 1)))))\n",
                x = 20 * n
            )
        })
        .collect();
    std::fs::write(
        dir.join("rev.kicad_sch"),
        format!(
            "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n  (uuid \"{root}\")\n  (paper \"A4\")\n  (lib_symbols {lib_symbol})\n{placed}  (sheet_instances (path \"/\" (page \"1\"))))\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("rev.kicad_pro"), "{}\n").unwrap();
    std::fs::write(
        dir.join("sym-lib-table"),
        "(sym_lib_table (version 7)\n  (lib (name \"Rev\")(type \"KiCad\")(uri \"${KIPRJMOD}/Rev.kicad_sym\")(options \"\")(descr \"\")))\n",
    )
    .unwrap();
    mixed
}

/// The board places a footprint other than the symbol's own, and that footprint was edited in its library after
/// the board was last updated: the review is of the library file (the review hash is the library footprint's),
/// with a note that the board's copy differs; the library edited again is read again
#[test]
fn an_override_is_reviewed_from_its_library() {
    let dir = tempfile::tempdir().unwrap();
    let mixed = override_project(dir.path());
    add_pad(&mixed, 7);
    let mut h = harness("fp reload override", false);
    settle(&mut h);
    let ctx = h.ctx.clone();
    h.state_mut()
        .open_path(&ctx, &dir.path().join("rev.kicad_pro"));
    settle(&mut h);
    let i = h.state().symbol_row("Rev:Mixed").unwrap();
    h.state_mut().select_symbol(i, true);
    settle(&mut h);
    let st = h.state();
    let s = st.rt.sourced.as_ref().expect("a footprint shown");
    assert_eq!(s.source, FootprintSource::File(mixed.clone()));
    assert!(pads(&h).contains(&"7".to_string()), "{:?}", pads(&h));
    let note = h.state().rt.board_note.clone().unwrap_or_default();
    assert!(
        note.contains(
            "placed on U7 as Test:Mixed_Pads, not the symbol's own Footprint (Test:Asym_SOT)"
        ),
        "{note}"
    );
    assert!(
        note.contains("the board copy U7 differs from the library")
            && note.contains("update the board"),
        "{note}"
    );
    // Edited in the library again: read again, as any footprint shown
    add_pad(&mixed, 8);
    h.get_by_label_contains("Reload · F5").click();
    settle(&mut h);
    assert!(pads(&h).contains(&"8".to_string()), "{:?}", pads(&h));
    assert!(
        h.state()
            .rt
            .watch
            .files(watch::Group::List)
            .contains(&mixed)
    );
}
