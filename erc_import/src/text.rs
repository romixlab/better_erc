//! Text files in an unknown encoding: Altium and OrCAD write the Windows code page of the PC that exported them

use erc_model::{ImportError, Source};

/// The file as text: UTF-8 when it is (a byte order mark dropped), otherwise in the encoding the bytes look like
pub(crate) fn read(src: &Source, path: &str) -> Result<String, ImportError> {
    let bytes = src
        .files
        .read(path)
        .ok_or_else(|| ImportError::Missing(path.to_string()))?;
    Ok(decode(&bytes))
}

pub(crate) fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(bytes, true);
    detector.guess(None, true).decode(bytes).0.into_owned()
}

#[cfg_attr(not(any(feature = "altium", feature = "orcad")), allow(dead_code))]
/// The first bytes of a file, for sniffing, without the byte order mark and leading white space
pub(crate) fn head(src: &Source, path: &str, n: usize) -> Option<Vec<u8>> {
    let b = src.files.read(path)?;
    let b = b
        .strip_prefix(b"\xEF\xBB\xBF")
        .unwrap_or(&b)
        .trim_ascii_start();
    Some(b[..b.len().min(n)].to_vec())
}
