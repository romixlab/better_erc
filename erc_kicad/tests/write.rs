//! The formatting-preserving writer ([`erc_kicad::edit`], [`erc_kicad::symlib::set_property`]):
//!
//! - Synthetic symbol libraries and schematics in KiCad 8, 9 and 10's layouts: a property set or inserted, the
//!   rest of the file byte for byte.
//! - KiCad as the oracle where kicad-cli is installed: KiCad saving a file and then our edit gives the same bytes
//!   as our edit and then KiCad saving it, so a new property is where KiCad puts it and laid out as KiCad lays it
//!   out. `BERC_WRITE_KICAD=8,9,10` adds those KiCad versions through podman; each saves the files it opens, and
//!   schematics only KiCad 10 (older kicad-cli has no `sch upgrade`). KiCad's saves are cached in the target folder.
//! - The extended set, where the folders are there: every KiCad file of `~/git/kicad-library`
//!   (`BERC_KICAD_LIBRARY_DIR`) and `~/git/hw` (`BERC_HW_DIR`) read and written unchanged is the same bytes, and
//!   every symbol of every library and schematic gets the review hash as a pure insertion, then a pure value
//!   change.

use erc_kicad::edit::{Edits, Tree};
use erc_kicad::sexpr;
use erc_kicad::symlib::{EditError, OLDEST_WRITTEN, set_property};

const _: () = assert!(OLDEST_WRITTEN == 20231120);
use generate_netlists::project::{Kicad, podman_available};
use std::path::{Path, PathBuf};
use std::process::Command;

const FIELD: &str = "Footprint_Review";
const HASH: &str = "fp1:0123456789abcdef";
const HASH2: &str = "fp1:fedcba9876543210";

/// A KiCad 10 symbol library with one symbol, as the symbol editor writes it
const LIB10: &str = "(kicad_symbol_lib
\t(version 20251024)
\t(generator \"kicad_symbol_editor\")
\t(generator_version \"10.0\")
\t(symbol \"AMP\"
\t\t(exclude_from_sim no)
\t\t(in_bom yes)
\t\t(on_board yes)
\t\t(property \"Reference\" \"U\"
\t\t\t(at 0 2.54 0)
\t\t\t(show_name no)
\t\t\t(do_not_autoplace no)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t\t(justify left)
\t\t\t)
\t\t)
\t\t(property \"Value\" \"AMP\"
\t\t\t(at 0 -2.54 0)
\t\t\t(show_name no)
\t\t\t(do_not_autoplace no)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t\t(justify left)
\t\t\t)
\t\t)
\t\t(property \"Footprint\" \"Package_TO_SOT_SMD:SOT-23-5\"
\t\t\t(at 0 -5.08 0)
\t\t\t(show_name no)
\t\t\t(do_not_autoplace no)
\t\t\t(hide yes)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t)
\t\t)
\t\t(property \"Datasheet\" \"\"
\t\t\t(at 0 0 0)
\t\t\t(show_name no)
\t\t\t(do_not_autoplace no)
\t\t\t(hide yes)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t)
\t\t)
\t\t(property \"Description\" \"An amplifier\"
\t\t\t(at 0 0 0)
\t\t\t(show_name no)
\t\t\t(do_not_autoplace no)
\t\t\t(hide yes)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t)
\t\t)
\t\t(property \"ki_keywords\" \"amp\"
\t\t\t(at 0 0 0)
\t\t\t(show_name no)
\t\t\t(do_not_autoplace no)
\t\t\t(hide yes)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t)
\t\t)
\t\t(symbol \"AMP_0_1\"
\t\t\t(rectangle
\t\t\t\t(start -2.54 2.54)
\t\t\t\t(end 2.54 -2.54)
\t\t\t\t(stroke
\t\t\t\t\t(width 0.254)
\t\t\t\t\t(type default)
\t\t\t\t)
\t\t\t\t(fill
\t\t\t\t\t(type background)
\t\t\t\t)
\t\t\t)
\t\t)
\t\t(symbol \"AMP_1_1\"
\t\t\t(pin output line
\t\t\t\t(at 5.08 0 180)
\t\t\t\t(length 2.54)
\t\t\t\t(name \"OUT\"
\t\t\t\t\t(effects
\t\t\t\t\t\t(font
\t\t\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t\t\t)
\t\t\t\t\t)
\t\t\t\t)
\t\t\t\t(number \"1\"
\t\t\t\t\t(effects
\t\t\t\t\t\t(font
\t\t\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t\t\t)
\t\t\t\t\t)
\t\t\t\t)
\t\t\t)
\t\t)
\t\t(embedded_fonts no)
\t)
)
";

/// The KiCad 8 and 9 layout: `hide` inside the effects, no `show_name`; KiCad 9 adds `embedded_fonts`
fn lib9(version: u32) -> String {
    let text = format!(
        "(kicad_symbol_lib
\t(version {version})
\t(generator \"kicad_symbol_editor\")
\t(generator_version \"9.0\")
\t(symbol \"R\"
\t\t(exclude_from_sim no)
\t\t(in_bom yes)
\t\t(on_board yes)
\t\t(property \"Reference\" \"R\"
\t\t\t(at 2.032 0 90)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t)
\t\t)
\t\t(property \"Value\" \"R\"
\t\t\t(at 0 0 90)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t)
\t\t)
\t\t(property \"Footprint\" \"Resistor_SMD:R_0402_1005Metric\"
\t\t\t(at -1.778 0 90)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t\t(hide yes)
\t\t\t)
\t\t)
\t\t(property \"Datasheet\" \"~\"
\t\t\t(at 0 0 0)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t\t(hide yes)
\t\t\t)
\t\t)
\t\t(property \"Description\" \"Resistor\"
\t\t\t(at 0 0 0)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t\t(hide yes)
\t\t\t)
\t\t)
\t\t(property \"ki_fp_filters\" \"R_*\"
\t\t\t(at 0 0 0)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t\t(hide yes)
\t\t\t)
\t\t)
\t\t(symbol \"R_0_1\"
\t\t\t(rectangle
\t\t\t\t(start -1.016 -2.54)
\t\t\t\t(end 1.016 2.54)
\t\t\t\t(stroke
\t\t\t\t\t(width 0.254)
\t\t\t\t\t(type default)
\t\t\t\t)
\t\t\t\t(fill
\t\t\t\t\t(type none)
\t\t\t\t)
\t\t\t)
\t\t)
\t\t(symbol \"R_1_1\"
\t\t\t(pin passive line
\t\t\t\t(at 0 3.81 270)
\t\t\t\t(length 1.27)
\t\t\t\t(name \"~\"
\t\t\t\t\t(effects
\t\t\t\t\t\t(font
\t\t\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t\t\t)
\t\t\t\t\t)
\t\t\t\t)
\t\t\t\t(number \"1\"
\t\t\t\t\t(effects
\t\t\t\t\t\t(font
\t\t\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t\t\t)
\t\t\t\t\t)
\t\t\t\t)
\t\t\t)
\t\t\t(pin passive line
\t\t\t\t(at 0 -3.81 90)
\t\t\t\t(length 1.27)
\t\t\t\t(name \"~\"
\t\t\t\t\t(effects
\t\t\t\t\t\t(font
\t\t\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t\t\t)
\t\t\t\t\t)
\t\t\t\t)
\t\t\t\t(number \"2\"
\t\t\t\t\t(effects
\t\t\t\t\t\t(font
\t\t\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t\t\t)
\t\t\t\t\t)
\t\t\t\t)
\t\t\t)
\t\t)
\t\t(embedded_fonts no)
\t)
)
"
    );
    if version <= 20231120 {
        text.replace("\t\t(embedded_fonts no)\n", "")
            .replace("\"9.0\"", "\"8.0\"")
    } else {
        text
    }
}

/// A schematic whose `lib_symbols` holds `lib`'s first symbol as `Test:<name>`, one instance of it placed
fn schematic(lib: &str, version: u32) -> String {
    let start = lib.find("\t(symbol \"").unwrap();
    let end = lib.rfind("\n)").unwrap();
    let name_start = start + "\t(symbol \"".len();
    let name_end = name_start + lib[name_start..].find('"').unwrap();
    let name = &lib[name_start..name_end];
    // One level deeper, the top symbol renamed to its library id; its units keep their names
    let body: String = lib[start..end]
        .lines()
        .map(|l| format!("\t{l}\n"))
        .collect::<String>()
        .replacen(
            &format!("(symbol \"{name}\""),
            &format!("(symbol \"Test:{name}\""),
            1,
        );
    format!(
        "(kicad_sch
\t(version {version})
\t(generator \"eeschema\")
\t(generator_version \"10.0\")
\t(uuid \"6a3fa0b4-27c9-4c35-8a59-4d4c5a2b0001\")
\t(paper \"A4\")
\t(lib_symbols
{body}\t)
\t(sheet_instances
\t\t(path \"/\"
\t\t\t(page \"1\")
\t\t)
\t)
\t(embedded_fonts no)
)
"
    )
}

/// `new` is `old` with one stretch replaced: the common prefix and suffix, and what `old` and `new` have between
fn one_hunk<'a>(old: &'a str, new: &'a str) -> (&'a str, &'a str) {
    let pre = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    let suf = old[pre..]
        .bytes()
        .rev()
        .zip(new[pre..].bytes().rev())
        .take_while(|(a, b)| a == b)
        .count();
    (&old[pre..old.len() - suf], &new[pre..new.len() - suf])
}

/// The properties of symbol `name` itself (not inherited), in file order
fn props(text: &str, name: &str) -> Vec<(String, String)> {
    let parsed = sexpr::parse(text).unwrap();
    let root = &parsed[0];
    let symbols = match root.head() {
        Some("kicad_sch") => root.child("lib_symbols").unwrap(),
        _ => root,
    };
    let s = symbols
        .children("symbol")
        .find(|s| s.arg(0) == Some(name))
        .unwrap();
    s.children("property")
        .map(|p| {
            (
                p.arg(0).unwrap().to_string(),
                p.arg(1).unwrap_or("").to_string(),
            )
        })
        .collect()
}

/// What a new property must do to a file: be inserted (nothing else changes), read back with the value, the other
/// properties as they were; set again, nothing; another value, only the value's token
fn check_insert(text: &str, symbol: &str) -> String {
    let before = props(text, symbol);
    assert!(before.iter().all(|(k, _)| k != FIELD));
    let e = set_property(text, symbol, FIELD, HASH).unwrap();
    assert!(e.changed);
    assert_eq!(e.old, None);
    let (removed, added) = one_hunk(text, &e.text);
    assert_eq!(removed, "", "{symbol}: a pure insertion");
    assert!(added.contains(HASH), "{added}");
    let mut after = props(&e.text, symbol);
    let i = after.iter().position(|(k, _)| k == FIELD).unwrap();
    assert_eq!(after.remove(i).1, HASH);
    assert_eq!(after, before);
    // Right after the symbol's last own property. KiCad 10 writes its ki_ properties after the own ones, but a
    // hand-edited file may hold one earlier (`ki_keywords` before `Mpn1`): that one stays where it is
    let last_own = before.iter().rposition(|(k, _)| !k.starts_with("ki_"));
    assert_eq!(i, last_own.map_or(0, |n| n + 1), "{symbol}: {before:?}");
    let again = set_property(&e.text, symbol, FIELD, HASH).unwrap();
    assert!(!again.changed);
    assert_eq!(again.text, e.text);
    let other = set_property(&e.text, symbol, FIELD, HASH2).unwrap();
    assert_eq!(other.old.as_deref(), Some(HASH));
    assert_eq!(
        one_hunk(&e.text, &other.text),
        (&HASH[4..], &HASH2[4..]),
        "only the value changes"
    );
    e.text
}

/// `new` is `old` with `block` inserted right after the first `)` of `anchor` (a property's end before the next)
fn assert_inserted(old: &str, new: &str, anchor: &str, block: &str) {
    let at = old.find(anchor).unwrap() + anchor.find('\n').unwrap();
    assert_eq!(new, format!("{}{block}{}", &old[..at], &old[at..]));
}

#[test]
fn kicad10_library_insert() {
    let out = check_insert(LIB10, "AMP");
    // KiCad 10's layout: show_name, do_not_autoplace and hide on the property, after Description
    assert_inserted(
        LIB10,
        &out,
        "\t\t)\n\t\t(property \"ki_keywords\"",
        "\n\t\t(property \"Footprint_Review\" \"fp1:0123456789abcdef\"\n\t\t\t(at 0 0 0)\n\t\t\t(show_name no)\n\t\t\t\
         (do_not_autoplace no)\n\t\t\t(hide yes)\n\t\t\t(effects\n\t\t\t\t(font\n\t\t\t\t\t(size 1.27 1.27)\n\t\t\t\t)\n\t\t\t)\n\t\t)",
    );
}

#[test]
fn kicad8_and_9_library_insert() {
    for version in [20231120, 20241209] {
        let text = lib9(version);
        let out = check_insert(&text, "R");
        assert_inserted(
            &text,
            &out,
            "\t\t)\n\t\t(property \"ki_fp_filters\"",
            "\n\t\t(property \"Footprint_Review\" \"fp1:0123456789abcdef\"\n\t\t\t(at 0 0 0)\n\t\t\t(effects\n\t\t\t\t\
             (font\n\t\t\t\t\t(size 1.27 1.27)\n\t\t\t\t)\n\t\t\t\t(hide yes)\n\t\t\t)\n\t\t)",
        );
    }
}

#[test]
fn schematic_lib_symbols_insert() {
    for (lib, version) in [(LIB10.to_string(), 20260306), (lib9(20250114), 20250114)] {
        let sch = schematic(&lib, version);
        let name = if lib.contains("\"AMP\"") {
            "Test:AMP"
        } else {
            "Test:R"
        };
        let out = check_insert(&sch, name);
        // One level deeper than in the library
        assert!(
            out.contains("\t\t\t)\n\t\t\t(property \"Footprint_Review\""),
            "{out}"
        );
        // The library's own name is not the schematic's: only the library id is found
        let bare = name.trim_start_matches("Test:");
        assert_eq!(
            set_property(&sch, bare, FIELD, HASH).unwrap_err(),
            EditError::NoSymbol(bare.into())
        );
    }
}

/// A `.kicad_symdir` file is one symbol in a library of its own; a derived symbol gets its own property
#[test]
fn symdir_file_and_derived_symbol() {
    let derived = "(kicad_symbol_lib
\t(version 20251024)
\t(generator \"kicad_symbol_editor\")
\t(generator_version \"10.0\")
\t(symbol \"AMP_X\"
\t\t(extends \"AMP\")
\t\t(property \"Reference\" \"U\"
\t\t\t(at 0 2.54 0)
\t\t\t(show_name no)
\t\t\t(do_not_autoplace no)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t)
\t\t)
\t\t(property \"Value\" \"AMP_X\"
\t\t\t(at 0 -2.54 0)
\t\t\t(show_name no)
\t\t\t(do_not_autoplace no)
\t\t\t(effects
\t\t\t\t(font
\t\t\t\t\t(size 1.27 1.27)
\t\t\t\t)
\t\t\t)
\t\t)
\t\t(embedded_fonts no)
\t)
)
";
    let out = check_insert(derived, "AMP_X");
    let fields = erc_kicad::symlib::resolve_fields("AMP_X", |n| match n {
        "AMP_X" => Some(out.clone()),
        "AMP" => Some(LIB10.to_string()),
        _ => None,
    })
    .unwrap();
    assert!(fields.contains(&(FIELD.to_string(), HASH.to_string())));
    assert!(fields.contains(&(
        "Footprint".to_string(),
        "Package_TO_SOT_SMD:SOT-23-5".to_string()
    )));
}

#[test]
fn unusual_symbols() {
    // Only ki_ properties: before the first of them
    let ki_only = "(kicad_symbol_lib\n\t(version 20241209)\n\t(symbol \"A\"\n\t\t(in_bom yes)\n\t\t(property \"ki_keywords\" \"k\"\n\t\t\t(at 0 0 0)\n\t\t\t(effects\n\t\t\t\t(font\n\t\t\t\t\t(size 1.27 1.27)\n\t\t\t\t)\n\t\t\t\t(hide yes)\n\t\t\t)\n\t\t)\n\t\t(symbol \"A_1_1\")\n\t)\n)\n";
    check_insert(ki_only, "A");
    // No property at all: after the settings, before the units, in the version's layout
    let bare = "(kicad_symbol_lib\n\t(version 20251024)\n\t(symbol \"B\"\n\t\t(in_bom yes)\n\t\t(symbol \"B_1_1\")\n\t)\n)\n";
    let out = check_insert(bare, "B");
    assert!(out.contains("\t\t(in_bom yes)\n\t\t(property \"Footprint_Review\" \"fp1:0123456789abcdef\"\n\t\t\t(at 0 0 0)\n\t\t\t(show_name no)"), "{out}");
    let bare_old = bare.replace("20251024", "20241209");
    let out = check_insert(&bare_old, "B");
    assert!(out.contains("(at 0 0 0)\n\t\t\t(effects"), "{out}");
    // CRLF line breaks stay CRLF
    let crlf = lib9(20241209).replace('\n', "\r\n");
    let out = check_insert(&crlf, "R");
    assert!(!out.replace("\r\n", "").contains('\n'));
    // An existing property with an escaped value: only that token changes
    let esc = lib9(20241209).replace("\"Resistor\"", "\"a \\\"quoted\\\" one\"");
    let e = set_property(&esc, "R", "Description", "plain").unwrap();
    assert_eq!(e.old.as_deref(), Some("a \"quoted\" one"));
    assert_eq!(one_hunk(&esc, &e.text), ("a \\\"quoted\\\" one", "plain"));
    let e = set_property(&e.text, "R", "Description", "back\\slash \"q\"").unwrap();
    assert_eq!(props(&e.text, "R")[4].1, "back\\slash \"q\"");
}

#[test]
fn refusals() {
    assert_eq!(
        set_property(&lib9(20230121), "R", FIELD, HASH).unwrap_err(),
        EditError::TooOld(20230121)
    );
    assert_eq!(
        set_property(LIB10, "Nope", FIELD, HASH).unwrap_err(),
        EditError::NoSymbol("Nope".into())
    );
    assert_eq!(
        set_property("(kicad_pcb (version 20241229))", "R", FIELD, HASH).unwrap_err(),
        EditError::NotSymbolFile
    );
    assert!(matches!(
        set_property("(kicad_symbol_lib (version", "R", FIELD, HASH),
        Err(EditError::Parse(_))
    ));
}

// ---------------------------------------------------------------------------------------------------------------
// KiCad as the oracle

/// The installed kicad-cli's version, asked once
fn installed() -> Option<String> {
    static V: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    V.get_or_init(generate_netlists::project::installed_version)
        .clone()
}

/// The KiCad versions to save with: the installed kicad-cli, and `BERC_WRITE_KICAD`'s through podman
fn oracle_kicads() -> Vec<Kicad> {
    let mut out = Vec::new();
    if installed().is_some() {
        out.push(Kicad::Installed);
    }
    if let Ok(list) = std::env::var("BERC_WRITE_KICAD") {
        let podman = podman_available();
        for v in list.split(',').filter_map(|v| v.trim().parse::<u32>().ok()) {
            let k = Kicad::for_major(v);
            if k != Kicad::Installed {
                assert!(podman, "BERC_WRITE_KICAD={list} needs podman");
                out.push(k);
            }
        }
    }
    out
}

/// `text` saved by KiCad: `sym upgrade --force` for a library, `sch upgrade --force` for a schematic; cached in the
/// target folder by the KiCad and the text, as kicad-cli takes seconds a file
fn kicad_save(kicad: Kicad, text: &str, ext: &str, scratch: &Path) -> String {
    use std::hash::{Hash, Hasher};
    let tag = match kicad {
        Kicad::Installed => installed().unwrap_or_default(),
        Kicad::Container(v) => format!("container-{v}"),
    };
    let key: Vec<u64> = (0..2u64)
        .map(|seed| {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (seed, &tag, ext, text).hash(&mut h);
            h.finish()
        })
        .collect();
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("write-oracle-cache")
        .join(format!("{:016x}{:016x}.{ext}", key[0], key[1]));
    if let Ok(saved) = std::fs::read_to_string(&cache) {
        return saved;
    }
    let saved = kicad_save_now(kicad, text, ext, scratch);
    let _ = std::fs::create_dir_all(cache.parent().unwrap());
    let _ = std::fs::write(&cache, &saved);
    saved
}

fn kicad_save_now(kicad: Kicad, text: &str, ext: &str, scratch: &Path) -> String {
    let _ = std::fs::remove_dir_all(scratch);
    std::fs::create_dir_all(scratch).unwrap();
    let file = format!("in.{ext}");
    std::fs::write(scratch.join(&file), text).unwrap();
    let args: Vec<String> = match ext {
        "kicad_sym" => ["sym", "upgrade", "--force", "-o", "out.kicad_sym", &file]
            .map(String::from)
            .to_vec(),
        _ => ["sch", "upgrade", "--force", &file]
            .map(String::from)
            .to_vec(),
    };
    let out = match kicad {
        Kicad::Installed => Command::new("kicad-cli")
            .args(&args)
            .current_dir(scratch)
            .output()
            .unwrap(),
        // As this user, so the container can write the scratch folder
        Kicad::Container(v) => Command::new("podman")
            .args([
                "run",
                "--rm",
                "--userns=keep-id",
                "--user",
                &host_user(),
                "-e",
                "HOME=/tmp",
                "-v",
            ])
            .arg(format!("{}:/w:Z", scratch.display()))
            .args([
                "-w",
                "/w",
                &format!("docker.io/kicad/kicad:{v}.0"),
                "kicad-cli",
            ])
            .args(&args)
            .output()
            .unwrap(),
    };
    assert!(
        out.status.success(),
        "{kicad:?} kicad-cli {args:?}: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let saved = if ext == "kicad_sym" {
        "out.kicad_sym"
    } else {
        file.as_str()
    };
    std::fs::read_to_string(scratch.join(saved)).unwrap()
}

/// The KiCad major version of `kicad`
fn major(kicad: Kicad) -> u32 {
    match kicad {
        Kicad::Installed => installed()
            .and_then(|v| v.split('.').next()?.parse().ok())
            .unwrap_or(0),
        Kicad::Container(v) => v,
    }
}

/// Whether KiCad `major` opens a file of `version`: none opens a newer KiCad's file
fn opens(major: u32, version: u32) -> bool {
    match major {
        8 => version <= 20231120,
        9 => version <= 20250114,
        _ => true,
    }
}

/// The file version of a library or schematic
fn version_of(text: &str) -> u32 {
    let t = Tree::parse(text).unwrap();
    let root = &t.roots()[0];
    t.child(root, "version")
        .and_then(|v| t.arg(v, 0))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// `uid:gid` of this process, for podman's `--user`
fn host_user() -> String {
    let id = |flag: &str| {
        let out = Command::new("id").arg(flag).output().unwrap();
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };
    format!("{}:{}", id("-u"), id("-g"))
}

/// KiCad then our edit is our edit then KiCad: the new property is where and how KiCad writes it. Skipped (false)
/// where this KiCad can't open the file, and for a schematic before KiCad 10 (its kicad-cli has no `sch upgrade`).
fn check_oracle(kicad: Kicad, text: &str, ext: &str, symbol: &str, scratch: &Path) -> bool {
    let m = major(kicad);
    if !opens(m, version_of(text)) || ext == "kicad_sch" && m < 10 {
        return false;
    }
    let edited = set_property(text, symbol, FIELD, HASH).unwrap().text;
    let saved = kicad_save(kicad, text, ext, scratch);
    let saved_edited = kicad_save(kicad, &edited, ext, scratch);
    let ours = set_property(&saved, symbol, FIELD, HASH).unwrap().text;
    if ours != saved_edited {
        let (a, b) = one_hunk(&ours, &saved_edited);
        panic!("{kicad:?} {symbol}: ours\n{a}\nKiCad's\n{b}");
    }
    true
}

#[test]
fn kicad_is_the_oracle() {
    let kicads = oracle_kicads();
    if kicads.is_empty() {
        eprintln!("kicad-cli is not installed: the writer is not checked against KiCad");
        return;
    }
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("write-oracle");
    for k in kicads {
        let cases = [
            (LIB10.to_string(), "kicad_sym", "AMP"),
            (lib9(20241209), "kicad_sym", "R"),
            (lib9(20231120), "kicad_sym", "R"),
            (schematic(LIB10, 20260306), "kicad_sch", "Test:AMP"),
            (schematic(&lib9(20250114), 20250114), "kicad_sch", "Test:R"),
            (schematic(&lib9(20231120), 20231120), "kicad_sch", "Test:R"),
        ];
        let mut n = 0;
        for (text, ext, symbol) in &cases {
            n += usize::from(check_oracle(k, text, ext, symbol, &scratch));
        }
        assert!(n >= 1, "{k:?}: no file this KiCad opens");
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The extended set

fn dir_from(var: &str, home_rel: &str) -> Option<PathBuf> {
    let d = std::env::var_os(var)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(home_rel)))?;
    d.is_dir().then_some(d)
}

/// Every KiCad s-expression file below `dir`, sorted; `.git` and other hidden folders skipped
fn kicad_files(dir: &Path) -> Vec<PathBuf> {
    const EXT: [&str; 6] = [
        "kicad_sym",
        "kicad_sch",
        "kicad_pcb",
        "kicad_mod",
        "kicad_wks",
        "kicad_dru",
    ];
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if EXT.iter().any(|x| name.ends_with(&format!(".{x}")))
                || name == "sym-lib-table"
                || name == "fp-lib-table"
            {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// The symbols an edit can name in a library or schematic: top-level ones, or the `lib_symbols` copies
fn symbol_names(text: &str) -> Vec<String> {
    let Ok(tree) = Tree::parse(text) else {
        return Vec::new();
    };
    let list = tree.root("kicad_symbol_lib").or_else(|| {
        tree.root("kicad_sch")
            .and_then(|s| tree.child(s, "lib_symbols"))
    });
    list.map(|l| {
        tree.children(l, "symbol")
            .filter_map(|s| tree.arg(s, 0).map(|n| n.into_owned()))
            .collect()
    })
    .unwrap_or_default()
}

#[test]
fn extended_set_round_trips() {
    let dirs: Vec<PathBuf> = [
        dir_from("BERC_KICAD_LIBRARY_DIR", "git/kicad-library"),
        dir_from("BERC_HW_DIR", "git/hw"),
    ]
    .into_iter()
    .flatten()
    .collect();
    if dirs.is_empty() {
        eprintln!("no ~/git/kicad-library or ~/git/hw: the writer's extended set is not run");
        return;
    }
    let (mut files, mut symbols, mut too_old) = (0, 0, 0);
    for dir in &dirs {
        for path in kicad_files(dir) {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            files += 1;
            // Read and written without a change: the same bytes
            let tree = Tree::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(
                Edits::new().apply(tree.text()).unwrap(),
                text,
                "{}",
                path.display()
            );
            // Every symbol with BERC_HW_FULL (`just test-full`), else the first few of each file
            let names = symbol_names(&text);
            let take = if full() { names.len() } else { 3 };
            for name in names.into_iter().take(take) {
                let has = props(&text, &name).iter().any(|(k, _)| k == FIELD);
                match set_property(&text, &name, FIELD, HASH) {
                    Err(EditError::TooOld(_)) => {
                        too_old += 1;
                        break;
                    }
                    Err(e) => panic!("{} {name}: {e}", path.display()),
                    Ok(_) if has => {}
                    Ok(_) => {
                        std::panic::catch_unwind(|| check_insert(&text, &name))
                            .unwrap_or_else(|_| panic!("{} {name}", path.display()));
                    }
                }
                symbols += 1;
            }
        }
    }
    eprintln!(
        "writer: {files} files read and written unchanged, {symbols} symbols given a property, {too_old} files older than KiCad 8 refused"
    );
    assert!(files > 0);
}

/// With `BERC_HW_FULL` (`just test-full`): every symbol of the extended set, the oracle on more files
fn full() -> bool {
    std::env::var_os("BERC_HW_FULL").is_some()
}

/// KiCad as the oracle on a sample of the extended set's libraries and schematics (8 files spread over them, every
/// 10th with `BERC_HW_FULL`), the first symbol of each without a review hash
#[test]
fn extended_set_kicad_oracle() {
    let kicads = oracle_kicads();
    let Some(dir) = dir_from("BERC_KICAD_LIBRARY_DIR", "git/kicad-library") else {
        eprintln!("no ~/git/kicad-library: the oracle's extended set is not run");
        return;
    };
    if kicads.is_empty() {
        eprintln!("kicad-cli is not installed: the oracle's extended set is not run");
        return;
    }
    let mut files: Vec<PathBuf> = kicad_files(&dir)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "kicad_sym"))
        .collect();
    if let Some(hw) = dir_from("BERC_HW_DIR", "git/hw") {
        files.extend(kicad_files(&hw).into_iter().filter(|p| {
            p.extension()
                .is_some_and(|e| e == "kicad_sch" || e == "kicad_sym")
        }));
    }
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("write-oracle-ext");
    let mut n = 0;
    let step = if full() {
        10
    } else {
        files.len().div_ceil(8).max(1)
    };
    for path in files.iter().step_by(step) {
        let text = std::fs::read_to_string(path).unwrap();
        let ext = path.extension().unwrap().to_string_lossy().into_owned();
        // A derived symbol (`extends`) in a symdir file has its parent in a sibling file: alone KiCad cannot load it
        if text.contains("(extends ") {
            continue;
        }
        let Some(name) = symbol_names(&text)
            .into_iter()
            .find(|s| !props(&text, s).iter().any(|(k, _)| k == FIELD))
        else {
            continue;
        };
        if matches!(
            set_property(&text, &name, FIELD, HASH),
            Err(EditError::TooOld(_))
        ) {
            continue;
        }
        for k in &kicads {
            let _ = check_oracle(*k, &text, &ext, &name, &scratch);
        }
        n += 1;
    }
    eprintln!("writer oracle: {n} files");
}
