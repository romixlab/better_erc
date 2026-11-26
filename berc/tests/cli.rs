//! The `berc` binary itself: version line, errors, completion

use std::process::Command;

fn berc(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_berc"))
        .env("BERC_USAGE", "off")
        .args(args)
        .env_remove("NO_COLOR")
        .env_remove("COMPLETE")
        .output()
        .unwrap()
}

#[test]
fn version_names_the_commit() {
    let out = berc(&["--version"]);
    assert!(out.status.success());
    let version = String::from_utf8(out.stdout).unwrap();
    // "berc 0.3.2 (9bdcaf0-dirty, commit 21 Oct 2026 20:31 UTC)", or "unknown" without git
    let expected_start = format!("berc {} (", env!("CARGO_PKG_VERSION"));
    assert!(version.starts_with(&expected_start), "{version}");
    assert!(version.contains(", commit "), "{version}");
}

#[test]
fn unreadable_file_is_a_plain_error_when_piped() {
    let out = berc(&["textconv", "/nonexistent/file.kicad_sch"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.starts_with("berc: error: can't read /nonexistent/file.kicad_sch"),
        "{stderr}"
    );
    assert!(
        !stderr.contains('\x1b'),
        "colour codes on a pipe: {stderr:?}"
    );
    assert!(out.stdout.is_empty());
}

#[test]
fn completion_scripts_for_bash_zsh_and_fish() {
    for shell in ["bash", "zsh", "fish"] {
        let out = Command::new(env!("CARGO_BIN_EXE_berc"))
            .env("BERC_USAGE", "off")
            .env("COMPLETE", shell)
            .output()
            .unwrap();
        assert!(out.status.success(), "{shell}");
        let script = String::from_utf8(out.stdout).unwrap();
        assert!(script.contains("berc"), "{shell}: {script}");
    }
}

/// The JSON is the library's types; the text is a table
#[test]
fn probe_access_json_on_a_board() {
    let board = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/small.kicad_pcb"
    );
    let out = berc(&["probe-access", board, "--format", "json"]);
    assert!(out.status.success(), "{out:?}");
    let p: better_erc::probe::ProbeAccess = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(p.schema, better_erc::probe::SCHEMA_VERSION);
    assert_eq!(p.board, "small.kicad_pcb");
    assert_eq!(p.nets.iter().map(|n| n.vias.len()).sum::<usize>(), 1);
    let text = berc(&["probe-access", board]);
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(text.contains("/SDA  -    -       1"), "{text}");
}

#[test]
fn probe_access_without_a_board_fails() {
    let out = berc(&["probe-access", "/nonexistent/dir"]);
    assert_eq!(out.status.code(), Some(2));
}
