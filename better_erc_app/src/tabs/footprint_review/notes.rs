//! Review notes: what the person writes while comparing a footprint with its datasheet, appended to a Markdown file
//! (a setting, by default `footprint-review-notes.md` beside the project or library), each note with the symbol,
//! the footprint, the datasheet as the document library names it and the page, the time, and the text with the
//! measurements (`M1 = 1.234 mm`) it refers to.

use better_erc::model::fact::Time;
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};

/// One note, as it goes into the file
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    /// The library symbol under review; none for a footprint opened alone
    pub symbol: Option<String>,
    pub footprint: String,
    /// The datasheet as the library names it (`tpm d DS13560 rev6`), else its file
    pub datasheet: Option<String>,
    /// 1-based, as a viewer shows it
    pub page: Option<usize>,
    /// Unix seconds
    pub when: i64,
    pub text: String,
}

impl Note {
    /// The note as a Markdown section: a heading with the symbol (else the footprint) and the time, a list of
    /// what it is about, the text
    pub fn markdown(&self) -> String {
        let mut out = format!(
            "## {} · {}\n\n",
            self.symbol.as_deref().unwrap_or(&self.footprint),
            Time(self.when)
        );
        if let Some(s) = &self.symbol {
            out.push_str(&format!("- Symbol: `{s}`\n"));
        }
        out.push_str(&format!("- Footprint: `{}`\n", self.footprint));
        match (&self.datasheet, self.page) {
            (Some(d), Some(p)) => out.push_str(&format!("- Datasheet: {d}, page {p}\n")),
            (Some(d), None) => out.push_str(&format!("- Datasheet: {d}\n")),
            (None, _) => out.push_str("- Datasheet: none open\n"),
        }
        out.push_str(&format!("\n{}\n\n", self.text.trim()));
        out
    }
}

/// Appends `note` to `file`, made with a title first when it is new; the folder must be there
pub fn append(file: &Path, note: &Note) -> Result<(), String> {
    let new = !file.exists();
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .map_err(|e| format!("can't write {}: {e}", file.display()))?;
    let mut text = String::new();
    if new {
        text.push_str("# Footprint review notes\n\n");
    }
    text.push_str(&note.markdown());
    f.write_all(text.as_bytes())
        .map_err(|e| format!("can't write {}: {e}", file.display()))
}

/// The notes file: `setting` as it is when absolute, else beside the project or library in `dir`; none without
/// either
pub fn file(setting: &str, dir: Option<&Path>) -> Option<PathBuf> {
    let setting = Path::new(setting.trim());
    if setting.as_os_str().is_empty() {
        return None;
    }
    if setting.is_absolute() {
        return Some(setting.to_path_buf());
    }
    dir.map(|d| d.join(setting))
}

/// One document as the library's open command lists it (`tpm lib open --json`)
#[derive(Deserialize)]
struct LibraryDoc {
    path: PathBuf,
    /// The words that open it (its file name, with folders where it takes them)
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    doc_no: Option<String>,
    #[serde(default)]
    doc_rev: Option<String>,
}

/// How the library names the document `file` from `json`, the open command's list: by the vendor's document
/// number and revision where it knows them (they stay when the file is renamed or moved), else by the words that
/// open it; none when the list has no such file. As a command that opens it: `tpm d DS13560 rev6`.
pub fn library_name(json: &str, file: &Path, open: &str) -> Result<Option<String>, String> {
    let docs: Vec<LibraryDoc> =
        serde_json::from_str(json).map_err(|e| format!("not a JSON list of documents: {e}"))?;
    let same = |p: &Path| {
        p == file
            || std::fs::canonicalize(p)
                .ok()
                .zip(std::fs::canonicalize(file).ok())
                .is_some_and(|(a, b)| a == b)
    };
    let Some(d) = docs.into_iter().find(|d| same(&d.path)) else {
        return Ok(None);
    };
    let words = match (d.doc_no, d.doc_rev, d.value) {
        (Some(no), Some(rev), _) => format!("{no} rev{rev}"),
        (Some(no), None, _) => no,
        (None, _, Some(v)) if v.contains(char::is_whitespace) => format!("\"{v}\""),
        (None, _, Some(v)) => v,
        (None, _, None) => return Ok(None),
    };
    Ok(Some(format!("`{open} {words}`")))
}

/// Runs the library's open command (`cmd`, the file added at the end) and names `file` by its answer
/// ([`library_name`]); the command's own name for the open command shown is `tpm d` when it is tpm's
pub fn run_library_name(cmd: &[String], file: &Path) -> Result<Option<String>, String> {
    let (prog, args) = cmd.split_first().ok_or("no library name command")?;
    let out = std::process::Command::new(prog)
        .args(args)
        .arg(file)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("can't run {prog}: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    if text.trim().is_empty() {
        // tpm lib open exits 1 with nothing found
        return Ok(None);
    }
    let open = if Path::new(prog).file_name().is_some_and(|n| n == "tpm") {
        "tpm d".to_string()
    } else {
        cmd.join(" ")
    };
    library_name(&text, file, &open).map_err(|e| format!("{}: {e}", cmd.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `tpm lib open --json`'s list: the entry of the file asked for, by its number and revision, else by the
    /// words that open it
    #[test]
    fn library_names_by_number_and_revision() {
        let json = r#"[
          {"path": "/lib/ST/stm32g0b1ce datasheet.pdf", "name": "stm32g0b1ce datasheet.pdf",
           "value": "stm32g0b1ce datasheet.pdf", "doc_no": "DS13560", "doc_rev": "4", "kind": "datasheet"},
          {"path": "/lib/ST/stm32g0b1 datasheet DS13560 rev6.pdf", "value": "stm32g0b1 datasheet DS13560 rev6.pdf",
           "doc_no": "DS13560", "doc_rev": "6"},
          {"path": "/lib/TI/tps1234.pdf", "value": "TI/tps1234.pdf", "doc_no": null, "doc_rev": null}
        ]"#;
        let name = |f: &str| library_name(json, Path::new(f), "tpm d").unwrap();
        assert_eq!(
            name("/lib/ST/stm32g0b1 datasheet DS13560 rev6.pdf").as_deref(),
            Some("`tpm d DS13560 rev6`")
        );
        assert_eq!(
            name("/lib/ST/stm32g0b1ce datasheet.pdf").as_deref(),
            Some("`tpm d DS13560 rev4`")
        );
        assert_eq!(
            name("/lib/TI/tps1234.pdf").as_deref(),
            Some("`tpm d TI/tps1234.pdf`")
        );
        assert_eq!(name("/elsewhere/x.pdf"), None);
        assert!(library_name("nothing", Path::new("/x"), "tpm d").is_err());
    }

    /// A note as Markdown, and the file made with its title on the first note, the next appended under it
    #[test]
    fn notes_appended_to_their_file() {
        let dir = std::env::temp_dir().join(format!("berc-fp-notes-unit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let note = Note {
            symbol: Some("Rev:Mixed".into()),
            footprint: "Test:Mixed_Pads".into(),
            datasheet: Some("`tpm d DS13560 rev6`".into()),
            page: Some(2),
            when: Time::from_utc(2026, 11, 12, 14, 3, 0).0,
            text: "pad 3 is 0.1 mm short: M1 = 2.600 mm ".into(),
        };
        let file = file("footprint-review-notes.md", Some(&dir)).unwrap();
        assert_eq!(file, dir.join("footprint-review-notes.md"));
        append(&file, &note).unwrap();
        append(
            &file,
            &Note {
                symbol: None,
                datasheet: None,
                text: "a preview".into(),
                ..note.clone()
            },
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "# Footprint review notes\n\n\
             ## Rev:Mixed · 2026-11-12T14:03:00Z\n\n\
             - Symbol: `Rev:Mixed`\n\
             - Footprint: `Test:Mixed_Pads`\n\
             - Datasheet: `tpm d DS13560 rev6`, page 2\n\n\
             pad 3 is 0.1 mm short: M1 = 2.600 mm\n\n\
             ## Test:Mixed_Pads · 2026-11-12T14:03:00Z\n\n\
             - Footprint: `Test:Mixed_Pads`\n\
             - Datasheet: none open\n\n\
             a preview\n\n"
        );
        assert_eq!(
            super::file("/abs/notes.md", None),
            Some(PathBuf::from("/abs/notes.md"))
        );
        assert_eq!(super::file("notes.md", None), None);
        assert_eq!(super::file("", Some(&dir)), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
