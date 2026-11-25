//! The datasheet as the document library knows it, for the review Accept records: its content hash and its document
//! number and revision, the library's catalog (mx3_api_lab's `/lib/docs`, VIEW-26), and the way back from a
//! recorded review to its PDF, through [`super::mx3lib::roots`]. A file's catalog path is `share/path`, so it is
//! matched by the end of a local path.

use better_erc::parts::lock::LockedReview;
use mx3_api_lab::blocking::BlockingClient;
use mx3_api_lab::endpoints::library::{DocFile, DocsQuery, docs as lib_docs};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// One file of the library's index
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct IndexedDoc {
    /// `share/path`, as the catalog lists it
    pub path: PathBuf,
    /// The content hash, blake3 in hex
    pub hash: String,
    #[serde(default)]
    pub doc: Option<String>,
    #[serde(default)]
    pub rev: Option<String>,
    /// No longer in the library
    #[serde(default)]
    pub gone: bool,
}

/// The fewest hex digits of a content hash two hashes are compared on
const HASH_DIGITS_MIN: usize = 16;

impl IndexedDoc {
    /// The content hash as a review records it: blake3 in lowercase hex
    pub fn blake3(&self) -> Option<String> {
        (self.hash.len() >= HASH_DIGITS_MIN && self.hash.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| self.hash.to_ascii_lowercase())
    }

    /// Its document number and revision for the person (`DS13560 rev6`), where it is known
    pub fn doc_label(&self) -> Option<String> {
        let no = self.doc.as_deref().filter(|d| !d.is_empty())?;
        Some(match self.rev.as_deref().filter(|r| !r.is_empty()) {
            Some(r) => format!("{no} rev{r}"),
            None => no.to_string(),
        })
    }

    fn from_doc_file(f: DocFile) -> IndexedDoc {
        IndexedDoc {
            path: PathBuf::from(format!("{}/{}", f.share, f.path)),
            hash: f.hash,
            doc: f.ident.doc,
            rev: f.ident.rev,
            gone: f.gone.is_some(),
        }
    }
}

/// The catalog's files, including those gone, through mx3's `/lib/docs`
pub fn fetch_index(client: &BlockingClient) -> Result<Vec<IndexedDoc>, String> {
    let q = DocsQuery {
        if_seq: None,
        gone: true,
    };
    let answer = lib_docs(client, &q).map_err(|e| format!("mx3 /lib/docs: {e:#}"))?;
    Ok(answer
        .files
        .into_iter()
        .map(IndexedDoc::from_doc_file)
        .collect())
}

/// Two content hashes are the same PDF: one starts with the other, on at least [`HASH_DIGITS_MIN`] digits
fn same_hash(a: &str, b: &str) -> bool {
    let n = a.len().min(b.len());
    n >= HASH_DIGITS_MIN && a[..n].eq_ignore_ascii_case(&b[..n])
}

/// How well `doc` is the document `review` was made on: 0 the same PDF by its content hash, 1 the same document
/// number and revision, 2 the same number in another revision; `None` another document
pub fn rank(doc: &IndexedDoc, review: &LockedReview) -> Option<u8> {
    if let (Some(want), Some(have)) = (&review.pdf_blake3, doc.blake3())
        && same_hash(want, &have)
    {
        return Some(0);
    }
    let no = doc.doc.as_deref().filter(|d| !d.is_empty())?;
    if review.doc.as_deref() != Some(no) {
        return None;
    }
    let rev = |r: Option<&str>| {
        r.map(|r| {
            let r = r.to_ascii_lowercase();
            r.strip_prefix("rev").unwrap_or(&r).to_owned()
        })
    };
    Some(
        if rev(review.doc_rev.as_deref()) == rev(doc.rev.as_deref()) {
            1
        } else {
            2
        },
    )
}

/// The index's entry for `file`: the one whose path `file` ends with, the longest such path when several do
pub fn identify<'a>(index: &'a [IndexedDoc], file: &Path) -> Option<&'a IndexedDoc> {
    index
        .iter()
        .filter(|d| !d.gone && !d.path.as_os_str().is_empty() && file.ends_with(&d.path))
        .max_by_key(|d| d.path.components().count())
}

/// The index's files for a recorded review, best first ([`rank`])
pub fn candidates<'a>(index: &'a [IndexedDoc], review: &LockedReview) -> Vec<&'a IndexedDoc> {
    let mut out: Vec<(u8, &IndexedDoc)> = index
        .iter()
        .filter(|d| !d.gone)
        .filter_map(|d| Some((rank(d, review)?, d)))
        .collect();
    out.sort_by_key(|(r, _)| *r);
    out.into_iter().map(|(_, d)| d).collect()
}

/// The index as a fixture (tests): a JSON list in [`IndexedDoc`]'s shape
#[cfg(test)]
pub fn parse_index(json: &str) -> Result<Vec<IndexedDoc>, String> {
    serde_json::from_str(json).map_err(|e| format!("not a JSON list of indexed documents: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str = r#"[
      {"doc": "DS30154", "rev": "16", "path": "datasheets/Diodes/BSS138W.pdf",
       "hash": "0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef", "gone": false},
      {"doc": "DS30154", "rev": "15", "path": "datasheets/Diodes/old/BSS138W.pdf",
       "hash": "ffff456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef", "gone": false},
      {"doc": null, "rev": null, "path": "datasheets/Nexperia/BSS138BKW.pdf",
       "hash": "aaaa456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef", "gone": false},
      {"doc": "DS1", "rev": "1", "path": "datasheets/gone.pdf", "hash": "bbbb", "gone": true}
    ]"#;

    /// A file by the end of its path, the longest match winning; its `pdf=` and `doc=`
    #[test]
    fn files_by_the_end_of_their_path() {
        let index = parse_index(INDEX).unwrap();
        let d = identify(
            &index,
            Path::new("/home/x/sync/datasheets/Diodes/BSS138W.pdf"),
        )
        .unwrap();
        assert_eq!(
            d.blake3().as_deref(),
            Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
        );
        assert_eq!(d.doc_label().as_deref(), Some("DS30154 rev16"));
        let old = identify(&index, Path::new("/s/datasheets/Diodes/old/BSS138W.pdf")).unwrap();
        assert_eq!(old.rev.as_deref(), Some("15"));
        let nx = identify(&index, Path::new("/s/datasheets/Nexperia/BSS138BKW.pdf")).unwrap();
        assert_eq!(nx.doc_label(), None);
        assert!(identify(&index, Path::new("/s/BSS138W.pdf")).is_none());
        assert!(identify(&index, Path::new("/s/datasheets/gone.pdf")).is_none());
        assert!(parse_index("nope").is_err());
    }

    /// A recorded review's PDF: by its content hash first, then by number and revision, then the number alone
    #[test]
    fn a_recorded_review_finds_its_pdf() {
        let index = parse_index(INDEX).unwrap();
        let review = |pdf: Option<&str>, doc: Option<&str>, rev: Option<&str>| LockedReview {
            pdf_blake3: pdf.map(String::from),
            doc: doc.map(String::from),
            doc_rev: rev.map(String::from),
            ..LockedReview::default()
        };
        let by_hash = candidates(
            &index,
            &review(Some("aaaa456789abcdef"), Some("DS30154"), Some("rev15")),
        );
        let paths: Vec<&Path> = by_hash.iter().map(|d| d.path.as_path()).collect();
        assert_eq!(
            paths,
            [
                Path::new("datasheets/Nexperia/BSS138BKW.pdf"),
                Path::new("datasheets/Diodes/old/BSS138W.pdf"),
                Path::new("datasheets/Diodes/BSS138W.pdf"),
            ]
        );
        let by_number = candidates(&index, &review(None, Some("DS30154"), Some("rev9")));
        assert_eq!(by_number.len(), 2);
        assert!(candidates(&index, &review(None, None, None)).is_empty());
        assert!(candidates(&index, &review(None, Some("DS1"), Some("rev1"))).is_empty());
    }

    fn fake_client(fake: &erc_fake_mx3::FakeMx3, token: bool) -> BlockingClient {
        let c = BlockingClient::new(fake.url().to_owned());
        if token {
            c.with_token(Some(erc_fake_mx3::TOKEN.to_owned()))
        } else {
            c
        }
    }

    /// The catalog comes from mx3's `/lib/docs`, gone files included; without a token the client is refused
    #[test]
    fn the_catalog_comes_from_lib_docs() {
        let fake = erc_fake_mx3::FakeMx3::start();
        let index = fetch_index(&fake_client(&fake, true)).unwrap();
        let link = index
            .iter()
            .find(|d| d.doc_label().as_deref() == Some("DSL1 rev1"))
            .unwrap();
        assert_eq!(link.path, Path::new("datasheets/Fixture/link.pdf"));
        assert!(!link.gone);
        assert!(index.iter().any(|d| d.gone), "{index:?}");
        assert!(fetch_index(&fake_client(&fake, false)).is_err());
    }
}
