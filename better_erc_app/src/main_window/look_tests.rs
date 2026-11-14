//! The whole app rendered with each tab in front, dark and light, a test design loaded: every tab draws.
//! Saved as PNGs when `BERC_LOOK_SNAPSHOT` names a file stem (`<stem>.<tab>.<theme>.png`), for before and after
//! pictures of a change to the app's look

use super::*;
use crate::context::Loaded;
use crate::tabs::TabKind;
use egui::vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use std::path::Path;

/// A test design with I2C buses and power rails, so the tabs have something to show
fn design() -> Context {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test_schematics/sources/i2c-10_non_equal_pull_ups.kicad_sch");
    let mut cx = Context::new();
    let loaded = better_erc::Project::open(&path)
        .and_then(Loaded::new)
        .unwrap();
    cx.blocking_write().load(loaded);
    cx
}

/// The app with one tab: a footprint review is not initialised, its settings would come from this PC's files
fn app_with(kind: TabKind, cx: &Context) -> BetterErcApp {
    let tab = match kind {
        TabKind::PcbDataImport => Tab::PcbDataImport(Default::default()),
        TabKind::Nets => Tab::Nets(Default::default()),
        TabKind::I2C => Tab::I2C(Default::default()),
        TabKind::Style => Tab::Style(Default::default()),
        TabKind::Inputs => Tab::Inputs(Default::default()),
        TabKind::FootprintReview => Tab::FootprintReview(Default::default()),
    };
    let mut tiles = egui_tiles::Tiles::default();
    let mut tab = tab;
    if kind != TabKind::FootprintReview {
        tab.init(cx);
    }
    let pane = tiles.insert_pane(tab);
    let root = tiles.insert_tab_tile(vec![pane]);
    let mut state = State {
        tabs: egui_tiles::Tree::new("look_test_tree", root, tiles),
        tabs_behavior: Default::default(),
        debug_window_shown: false,
    };
    state.tabs_behavior.feed_cx(cx.clone());
    BetterErcApp {
        cx: cx.clone(),
        state,
        look: None,
        colors_note: None,
    }
}

#[test]
fn every_tab_draws() {
    let cx = design();
    for kind in TabKind::iter() {
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let mut app = app_with(kind, &cx);
            let mut h = Harness::builder()
                .with_size(vec2(1280.0, 800.0))
                .with_theme(theme)
                .build_ui(move |ui| app.show(ui));
            setup_look(&h.ctx);
            h.run();
            h.run();
            if let Some(p) = std::env::var_os("BERC_LOOK_SNAPSHOT") {
                let name = format!(
                    "{}.{}.png",
                    kind.as_ref(),
                    format!("{theme:?}").to_lowercase()
                );
                h.render()
                    .unwrap()
                    .save(Path::new(&p).with_extension(name))
                    .unwrap();
            }
        }
    }
}

/// The colours follow the desktop theme (a synthetic Tokyo Night, as omarchy writes it), and the Colours menu says
/// so and switches to the brand's. Saved as `<stem>.<tab>.tokyo-night.png` with `BERC_LOOK_SNAPSHOT`
#[test]
fn follows_the_desktop_theme() {
    let dir = std::env::temp_dir().join(format!("berc-look-theme-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("colors.toml");
    std::fs::write(
        &file,
        "mode = \"dark\"\naccent = \"#7aa2f7\"\nselection = \"#292e42\"\nbackground = \"#1a1b26\"\n\
         lighter_background = \"#24283b\"\nforeground = \"#a9b1d6\"\ndark_foreground = \"#565f89\"\n",
    )
    .unwrap();
    let cx = design();
    for kind in [TabKind::FootprintReview, TabKind::Nets] {
        let app = app_with(kind, &cx);
        let mut h = Harness::builder()
            .with_size(vec2(1280.0, 800.0))
            .with_theme(egui::Theme::Dark)
            .build_ui_state(|ui, app: &mut BetterErcApp| app.show(ui), app);
        // The harness has a context of its own: the look goes there
        let look = follow_look_with(&h.ctx, ve_theme::ColorSource::Desktop, Some(file.clone()));
        h.state_mut().look = Some(look);
        h.run();
        assert_eq!(
            h.ctx.style_of(egui::Theme::Dark).visuals.panel_fill,
            ve_theme::parse_hex("#1a1b26").unwrap()
        );
        if let Some(p) = std::env::var_os("BERC_LOOK_SNAPSHOT") {
            let name = format!("{}.tokyo-night.png", kind.as_ref());
            h.render()
                .unwrap()
                .save(Path::new(&p).with_extension(name))
                .unwrap();
        }
        if kind == TabKind::Nets {
            h.get_by_label("Colours").click();
            h.run();
            h.get_by_label_contains("Following the desktop theme");
        }
    }
    std::fs::remove_dir_all(&dir).ok();
}
