//! The side panel with a review under way, rendered: a symbol picked from its library, its footprint and its
//! datasheet open on the land pattern page, a scale and a ruler set. Saved as a PNG when `BERC_PANEL_SNAPSHOT`
//! names a file (`<file>.dark.png`, `<file>.light.png`), for before and after pictures of the panel

use super::tests::{datasheet, symbol_library};
use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn review_under_way(dir: &Path, theme: egui::Theme) -> Harness<'static, FootprintReview> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    let pdf = datasheet(dir);
    let lib = symbol_library(dir, &pdf);
    let settings = FootprintReviewSettings {
        font_dirs: Vec::new(),
        library_search: Vec::new(),
        library_name: Vec::new(),
        ..Default::default()
    };
    let mut cx = Context::new();
    let mut h = Harness::builder()
        .with_size(vec2(1300.0, 2000.0))
        .with_theme(theme)
        .build_ui_state(
            move |ui, state: &mut FootprintReview| state.ui(ui, &mut cx, Id::new("fp panel")),
            FootprintReview::with(settings, Arc::new(|_: &str| {})),
        );
    h.run();
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, &lib);
    h.run();
    h.get_by_label("Rev:Mixed").click();
    h.run();
    h.run();
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    let st = h.state_mut();
    st.scale = Some(Scale {
        mm_per_point: 0.02,
        page: 1,
        mm: 2.0,
        note: None,
    });
    st.ruler = Some([[100.0, 200.0], [200.0, 200.0]]);
    st.place.origin = [150.0, 230.0];
    h.run();
    h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
    h.run();
    h
}

#[test]
fn panel_with_a_review_under_way() {
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        let dir = std::env::temp_dir().join(format!(
            "berc-fp-panel-{}-{theme:?}",
            std::process::id()
        ));
        let mut h = review_under_way(&dir, theme);
        h.get_by_label("Test:Mixed_Pads");
        if let Some(p) = std::env::var_os("BERC_PANEL_SNAPSHOT") {
            let name = format!("{theme:?}").to_lowercase();
            h.render()
                .unwrap()
                .save(Path::new(&p).with_extension(format!("{name}.png")))
                .unwrap();
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
