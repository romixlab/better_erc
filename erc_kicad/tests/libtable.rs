//! Library tables: a project's `fp-lib-table` and `sym-lib-table` before the global ones in a KiCad configuration
//! directory, `${VAR}` from `kicad_common.json` and `KIPRJMOD`, nested and disabled entries, and `Lib:Name` to a
//! parsed `.kicad_mod` of `fixtures/footprints/Test.pretty`.

use erc_kicad::libtable::{FootprintLibraries, LibError, LibKind, LibTables, SymbolLibraries};
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("erc-libtable-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn pretty() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/footprints/Test.pretty")
}

fn lib(name: &str, kind: &str, uri: &str, extra: &str) -> String {
    format!(
        "  (lib (name \"{name}\")(type \"{kind}\")(uri \"{uri}\")(options \"\")(descr \"\"){extra})\n"
    )
}

fn write_table(path: &Path, kind: LibKind, libs: &[String]) {
    let head = match kind {
        LibKind::Symbol => "sym_lib_table",
        LibKind::Footprint => "fp_lib_table",
    };
    std::fs::write(path, format!("({head} (version 7)\n{})\n", libs.concat())).unwrap();
}

/// A project directory and a configuration directory: the project's table lists `Prj` (through `KIPRJMOD`) and
/// shadows the global `Shadowed`; the global one lists `Test` through a `kicad_common.json` variable, a disabled
/// `Off`, a nested table with `Nested`, and `Unset` through a variable nobody sets
fn setup(name: &str) -> (PathBuf, PathBuf) {
    let root = scratch(name);
    let (prj, cfg) = (root.join("prj"), root.join("cfg"));
    std::fs::create_dir_all(prj.join("lib")).unwrap();
    std::fs::create_dir_all(&cfg).unwrap();
    std::fs::write(
        prj.join("lib/Own.kicad_mod"),
        std::fs::read_to_string(pretty().join("Asym_SOT.kicad_mod")).unwrap(),
    )
    .unwrap();
    std::fs::write(prj.join("lib/Broken.kicad_mod"), "(module x)").unwrap();
    let fixtures = pretty().parent().unwrap().to_string_lossy().into_owned();
    std::fs::write(
        cfg.join("kicad_common.json"),
        serde_json::json!({ "environment": { "vars": { "BERC_TEST_FP": fixtures } } }).to_string(),
    )
    .unwrap();
    write_table(
        &prj.join("fp-lib-table"),
        LibKind::Footprint,
        &[
            lib("Prj", "KiCad", "${KIPRJMOD}/lib", ""),
            lib("Shadowed", "KiCad", "${KIPRJMOD}/lib", ""),
        ],
    );
    write_table(
        &cfg.join("nested-fp-lib-table"),
        LibKind::Footprint,
        &[lib("Nested", "KiCad", "${BERC_TEST_FP}/Test.pretty", "")],
    );
    write_table(
        &cfg.join("fp-lib-table"),
        LibKind::Footprint,
        &[
            lib("Test", "KiCad", "${BERC_TEST_FP}/Test.pretty", ""),
            lib("Shadowed", "KiCad", "${BERC_TEST_FP}/Test.pretty", ""),
            lib("Off", "KiCad", "${BERC_TEST_FP}/Test.pretty", "(disabled)"),
            lib(
                "More",
                "Table",
                &cfg.join("nested-fp-lib-table").to_string_lossy(),
                "",
            ),
            lib(
                "Unset",
                "KiCad",
                "${BERC_TEST_NOBODY_SETS_THIS}/x.pretty",
                "",
            ),
            lib("Old", "Legacy", "${KIPRJMOD}/old.mod", ""),
        ],
    );
    (prj, cfg)
}

#[test]
fn footprints_through_project_and_global_tables() {
    let (prj, cfg) = setup("fp");
    let mut libs = FootprintLibraries::new(&prj, Some(&cfg));
    let names: Vec<&str> = libs
        .tables()
        .libraries()
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(names, ["Prj", "Shadowed", "Test", "Nested", "Unset", "Old"]);

    let f = libs.footprint("Test:Asym_SOT").unwrap();
    assert_eq!(f.path, pretty().join("Asym_SOT.kicad_mod"));
    assert_eq!(f.footprint.footprint.lib_id, "Asym_SOT");
    assert!(!f.footprint.pads.is_empty());
    assert!(f.notes.is_empty(), "{:#?}", f.notes);
    assert_eq!(
        libs.footprint("Nested:Mixed_Pads")
            .unwrap()
            .footprint
            .footprint
            .lib_id,
        "Mixed_Pads"
    );
    // The project's table goes first
    assert_eq!(
        libs.footprint("Shadowed:Own").unwrap().path,
        prj.join("lib/Own.kicad_mod")
    );
    assert_eq!(libs.names("Test").unwrap(), ["Asym_SOT", "Mixed_Pads"]);
    assert_eq!(libs.names("Prj").unwrap(), ["Broken", "Own"]);
    assert_eq!(libs.path("Prj:Own").unwrap(), prj.join("lib/Own.kicad_mod"));

    let err = |libs: &mut FootprintLibraries, id: &str| libs.footprint(id).unwrap_err();
    assert_eq!(
        err(&mut libs, "NoLib"),
        LibError::NoNickname("NoLib".into())
    );
    assert_eq!(
        err(&mut libs, "Gone:X").to_string(),
        "library Gone is in no fp-lib-table"
    );
    assert!(matches!(
        err(&mut libs, "Off:Asym_SOT"),
        LibError::Disabled { .. }
    ));
    assert!(matches!(
        err(&mut libs, "Unset:X"),
        LibError::Variable { .. }
    ));
    assert!(matches!(
        err(&mut libs, "Old:X"),
        LibError::Unsupported { .. }
    ));
    assert!(matches!(
        err(&mut libs, "Test:Missing"),
        LibError::NotFound { .. }
    ));
    match err(&mut libs, "Prj:Broken") {
        LibError::Invalid { path, message } => {
            assert_eq!(path, prj.join("lib/Broken.kicad_mod"));
            assert!(message.contains("KiCad 5"), "{message}");
        }
        e => panic!("{e:?}"),
    }
}

#[test]
fn symbols_and_footprints_have_their_own_tables() {
    let (prj, cfg) = setup("sym");
    std::fs::write(
        prj.join("Own.kicad_sym"),
        "(kicad_symbol_lib (version 20241209)\n  (symbol \"R\" (property \"Footprint\" \"Prj:Own\")))\n",
    )
    .unwrap();
    write_table(
        &prj.join("sym-lib-table"),
        LibKind::Symbol,
        &[lib("Own", "KiCad", "${KIPRJMOD}/Own.kicad_sym", "")],
    );
    let mut syms = SymbolLibraries::new(&prj, Some(&cfg));
    let fields = syms.fields("Own:R").unwrap();
    assert_eq!(fields, [("Footprint".to_string(), "Prj:Own".to_string())]);
    assert_eq!(
        syms.fields("Test:R").unwrap_err().to_string(),
        "library Test is in no sym-lib-table"
    );
    let fps = LibTables::new(LibKind::Footprint, &prj, None);
    assert!(fps.entry("Own").is_err());
    assert!(fps.entry("Prj").is_ok());
    assert!(
        fps.entry("Test").is_err(),
        "no global table without a config directory"
    );
}
