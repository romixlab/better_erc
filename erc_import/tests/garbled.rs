//! Broken input never panics: every fixture truncated at many points, with bytes, lines and brackets garbled, and
//! fed to every importer (also the wrong ones). An import either fails with an error or gives a design that dumps
//! and reads back.

use erc_import::Registry;
use erc_model::{ImportOptions, MemFiles, Source, dump};

mod common;

/// A small deterministic generator (xorshift), so a failure reproduces
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Broken copies of one file
fn variants(bytes: &[u8], rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut out = vec![Vec::new(), b"\xff\xfe\x00garbage".to_vec()];
    let n = bytes.len();
    for i in 0..40 {
        out.push(bytes[..n * i / 40].to_vec());
    }
    for _ in 0..40 {
        let mut b = bytes.to_vec();
        for _ in 0..1 + rng.below(8) {
            let i = rng.below(n);
            let what = [
                b'(',
                b')',
                b'"',
                b'\'',
                b';',
                b',',
                b'\n',
                b' ',
                0xC3,
                rng.next() as u8,
            ];
            b[i] = what[rng.below(what.len())];
        }
        out.push(b);
    }
    let lines: Vec<&[u8]> = bytes.split(|c| *c == b'\n').collect();
    for _ in 0..20 {
        let mut l = lines.clone();
        let i = rng.below(l.len());
        if rng.below(2) == 0 {
            l.remove(i);
        } else {
            let j = rng.below(l.len());
            l.swap(i, j);
        }
        out.push(l.join(&b'\n'));
    }
    out
}

fn check(reg: &Registry, files: &MemFiles, main: &str, what: &str) {
    let src = Source::new(main, files);
    for imp in reg.importers() {
        let _ = imp.sniff(&src);
        if let Ok(imported) = imp.import(&src, &ImportOptions::default()) {
            let text = dump::write(&imported.design);
            let back = dump::read(&text).unwrap_or_else(|e| {
                panic!(
                    "{what} through {}: dump does not read back: {e}",
                    imp.name()
                )
            });
            assert_eq!(dump::write(&back), text, "{what} through {}", imp.name());
        }
    }
}

#[test]
fn broken_fixtures_do_not_panic() {
    let reg = Registry::default();
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for (name, _, main, others) in common::CASES {
        let all: Vec<&str> = std::iter::once(*main)
            .chain(others.iter().copied())
            .collect();
        for broken in &all {
            let original = common::fixture(broken);
            for (k, v) in variants(&original, &mut rng).into_iter().enumerate() {
                let mut files = common::files(main, others);
                files.0.insert(broken.to_string(), v);
                check(&reg, &files, main, &format!("{name}: {broken} variant {k}"));
            }
        }
    }
}

#[test]
fn deep_nesting_is_refused() {
    let reg = Registry::default();
    let mut text = String::from("(edif x ");
    text.push_str(&"(a ".repeat(100_000));
    let files = MemFiles::default().with("deep.edf", text);
    let src = Source::new("deep.edf", &files);
    assert!(reg.import(&src, &ImportOptions::default()).is_err());
}
