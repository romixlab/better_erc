//! Every fixture through the registry: the importer it picks, a committed canonical dump of the design and of the
//! import notes (`tests/golden`, `BLESS=1` rewrites them), and `dump → read → dump` stable.

use common::{CASES, files};
use erc_import::Registry;
use erc_model::{Design, ImportOptions, Source, dump};
use std::path::Path;

mod common;

#[test]
fn golden_dumps() {
    let reg = Registry::default();
    let mut failed = Vec::new();
    for (name, importer, main, others) in CASES {
        let files = files(main, others);
        let src = Source::new(*main, &files);
        let (picked, _) = reg
            .pick(&src)
            .unwrap_or_else(|| panic!("{name}: no importer"));
        assert_eq!(picked.name(), *importer, "{name}");
        let imported = reg
            .import(&src, &ImportOptions::default())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let text = dump::write(&imported.design);
        let notes: String = imported.notes.iter().map(|n| format!("{n}\n")).collect();
        let golden = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden")
            .join(format!("{name}.dump"));
        let golden_notes = golden.with_extension("notes");
        if std::env::var_os("BLESS").is_some() {
            std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
            std::fs::write(&golden, &text).unwrap();
            if notes.is_empty() {
                let _ = std::fs::remove_file(&golden_notes);
            } else {
                std::fs::write(&golden_notes, &notes).unwrap();
            }
        }
        if std::fs::read_to_string(&golden).unwrap_or_default() != text {
            failed.push(golden.display().to_string());
        }
        if std::fs::read_to_string(&golden_notes).unwrap_or_default() != notes {
            failed.push(golden_notes.display().to_string());
        }
        let back: Design = dump::read(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(dump::write(&back), text, "{name}: dump, read, dump");
        assert!(
            back == imported.design,
            "{name}: read gives back the design"
        );
    }
    assert!(
        failed.is_empty(),
        "dumps differ (BLESS=1 to update): {failed:#?}"
    );
}
