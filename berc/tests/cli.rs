//! The `berc` binary itself: version line, errors, completion

use std::process::Command;

fn berc(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_berc"))
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
            .env("COMPLETE", shell)
            .output()
            .unwrap();
        assert!(out.status.success(), "{shell}");
        let script = String::from_utf8(out.stdout).unwrap();
        assert!(script.contains("berc"), "{shell}: {script}");
    }
}
