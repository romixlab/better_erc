//! The validator catches what it is there to catch.

use erc_levels::Database;

const HEAD: &str = r#"
schema = 1
[docs.D]
title = "t"
publisher = "p"
url = "https://example.org"
library = "design/x.pdf"
kind = "standard"
"#;

fn problems(family: &str) -> Vec<String> {
    let db = Database::parse(&format!("{HEAD}\n{family}")).unwrap();
    db.validate().into_iter().map(|p| p.to_string()).collect()
}

fn has(ps: &[String], needle: &str) -> bool {
    ps.iter().any(|p| p.contains(needle))
}

#[test]
fn a_sound_family_passes() {
    let ps = problems(
        r#"[family.f]
name = "F"
group = "G"
kind = "single-ended"
source = { doc = "D", loc = "Table 1" }
supply = [ { name = "VDD", min = 3.0, typ = 3.3, max = 3.6, verified = true } ]
level = [
  { param = "VIL", max = "0.3*VDD", unit = "V", verified = true },
  { param = "VIH", min = "0.7*VDD", unit = "V", verified = true },
  { param = "VOL", max = 0.4, unit = "V", iout_ma = 4.0, verified = true },
  { param = "VOH", min = "VDD-0.4", unit = "V", iout_ma = -4.0, verified = true },
  { param = "II", min = -1.0, max = 1.0, unit = "uA", verified = true },
]"#,
    );
    assert!(ps.is_empty(), "{ps:?}");
}

#[test]
fn catches_crossed_thresholds_and_bad_order() {
    let ps = problems(
        r#"[family.f]
name = "F"
group = "G"
kind = "single-ended"
source = { doc = "D", loc = "x" }
supply = [ { name = "VDD", min = 3.6, max = 3.0, verified = false } ]
level = [
  { param = "VIL", max = 2.0, unit = "V", verified = false },
  { param = "VIH", min = 1.5, unit = "V", verified = false },
  { param = "VOL", min = 0.5, typ = 0.3, unit = "V", verified = false },
]"#,
    );
    assert!(has(&ps, "supply VDD: min"), "{ps:?}");
    assert!(has(&ps, "VIL max 2 V is not below VIH min 1.5 V"), "{ps:?}");
    assert!(has(&ps, "VOL: min"), "{ps:?}");
}

#[test]
fn catches_units_sources_and_unknown_names() {
    let ps = problems(
        r#"[family.f]
name = "F"
group = "G"
kind = "single-ended"
source = { doc = "NOPE" }
supply = [ { name = "VDD", typ = 3.3, verified = true } ]
see = ["ghost"]
level = [
  { param = "VIH", min = 2.0, unit = "mA", verified = false },
  { param = "VIL", max = "0.3*VCC", unit = "V", verified = false },
  { param = "VOH", min = "VDD-0.4", unit = "mV", verified = false },
  { param = "XYZ", min = 1.0, unit = "V", verified = false },
  { param = "IIH", max = 1.0, unit = "uA", verified = false },
  { param = "IIH", max = 1.0, unit = "mA", verified = false },
]"#,
    );
    for n in [
        "unknown document \"NOPE\"",
        "see: unknown family",
        "VIH: unit MilliA does not fit",
        "uses VCC, which is not a supply",
        "VOH: an expression of a supply needs unit V",
        "XYZ: unknown parameter needs a desc",
        "IIH: unit MilliA, other rows of IIH use MicroA",
        "docs.D: not used",
    ] {
        assert!(has(&ps, n), "missing {n:?} in {ps:?}");
    }
}

#[test]
fn verified_needs_a_location_and_a_fetched_document() {
    let ps = problems(
        r#"[docs.E]
title = "t"
publisher = "p"
url = "https://example.org"
kind = "appnote"

[family.f]
name = "F"
group = "G"
kind = "single-ended"
source = { doc = "D" }
level = [
  { param = "VIH", min = 2.0, unit = "V", verified = true },
  { param = "VIL", max = 0.8, unit = "V", verified = true, doc = "E", loc = "p. 1" },
]"#,
    );
    assert!(has(&ps, "VIH: verified needs a location"), "{ps:?}");
    assert!(has(&ps, "document E is not in the library"), "{ps:?}");
}

#[test]
fn rejects_a_bad_expression_and_another_schema() {
    assert!(Database::parse(&HEAD.replace("schema = 1", "schema = 2")).is_err());
    let bad = format!(
        "{HEAD}\n[family.f]\nname = \"F\"\ngroup = \"G\"\nkind = \"bus\"\nsource = {{ doc = \"D\" }}\nlevel = [ {{ param = \"VIH\", min = \"VDD*2\", unit = \"V\", verified = false }} ]"
    );
    assert!(Database::parse(&bad).is_err());
}
