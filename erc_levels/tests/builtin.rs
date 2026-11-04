//! The embedded database loads and passes every check; its documents are in the library where that is here.

use std::path::PathBuf;

use erc_levels::{Corner, Database, FamilyKind, Quantity, Volts};

#[test]
fn builtin_parses_and_validates() {
    let db = Database::builtin();
    assert!(!db.family.is_empty());
    let problems = db.validate();
    assert!(
        problems.is_empty(),
        "{} problems:\n{}",
        problems.len(),
        problems
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn every_family_has_a_source_and_every_level_a_way_to_it() {
    let db = Database::builtin();
    for (id, f) in &db.family {
        assert!(
            db.docs.contains_key(&f.source.doc),
            "{id}: unknown doc {}",
            f.source.doc
        );
        if f.kind != FamilyKind::NoLevels {
            assert!(!f.level.is_empty(), "{id}: no levels");
        }
        for c in [Corner::Min, Corner::Typ, Corner::Max] {
            f.env(c).unwrap_or_else(|e| panic!("{id}: {e}"));
        }
    }
}

#[test]
fn i2c_fast_mode_at_3v3() {
    let f = Database::builtin().get("i2c-fm").unwrap();
    let mut env = f.env(Corner::Typ).unwrap();
    env.insert("VDD".into(), Volts::new(3.3));
    let vih = f
        .levels("VIH")
        .next()
        .unwrap()
        .volts(&env)
        .unwrap()
        .unwrap();
    assert!((vih[0].unwrap().get() - 2.31).abs() < 1e-9);
    let ii = f.levels("II").next().unwrap();
    assert!(
        ii.volts(&env).unwrap().is_none(),
        "a current is not a voltage"
    );
    let [_, _, max] = ii.limits(&env).unwrap();
    let Some(Quantity::Current(max)) = max else {
        panic!("{max:?}")
    };
    assert!((max.get() - 10e-6).abs() < 1e-15, "uA scaled to A");
    let vol = f.levels("VOL").next().unwrap();
    assert_eq!(vol.iout_ma.unwrap().get(), 3.0, "the test current in mA");
}

/// The extended set: each document with a library path is there, when the library is on this PC
/// (`BERC_LIBRARY_DIR`, else `~/sync`). Skipped with a note where it is not.
#[test]
fn library_documents_exist_where_the_library_is() {
    let root = std::env::var_os("BERC_LIBRARY_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join("sync")));
    let Some(root) = root.filter(|r| r.join("design").is_dir()) else {
        eprintln!(
            "note: no technical library here (BERC_LIBRARY_DIR or ~/sync/design), not checking document files"
        );
        return;
    };
    let db = Database::builtin();
    let missing: Vec<_> = db
        .docs
        .iter()
        .filter_map(|(id, d)| d.library.as_ref().map(|l| (id, l)))
        .filter(|(_, l)| !root.join(l).is_file())
        .map(|(id, l)| format!("{id}: {l}"))
        .collect();
    assert!(
        missing.is_empty(),
        "not in the library at {}:\n{}",
        root.display(),
        missing.join("\n")
    );
}
