//! The mx3 part source against a fake mx3: facts by MPN, reviews of a hash, a review accepted, the URL and token
//! found as the mx3 command line finds them

use erc_fake_mx3::{
    CAPACITOR, FakeMx3, MAKER_A, MAKER_A_SYNONYM, MAKER_B, NO_MAKER, RESISTOR, SHARED, SHARED_A_ID,
    SHARED_B_ID, SHARED_STUB_ID, TOKEN,
};
use erc_parts::board::{BoardMpns, HashedFootprint};
use erc_parts::keys;
use erc_parts::lock::{Cover, Coverage, FactValue, PartKey, PartMatch, PartsLock};
use erc_parts::mx3::{Connection, Environment, Mx3Source};
use erc_parts::refresh::refresh_mpns;
use erc_parts::{NewReview, Source};
use std::path::{Path, PathBuf};

const HASH: &str = "fp1:0123456789abcdef";

fn source(fake: &FakeMx3) -> Mx3Source {
    Mx3Source::connect(Connection::new(fake.url(), Some(TOKEN.to_owned()))).unwrap()
}

/// R1 the resistor, C1 the capacitor, U1 an MPN mx3 has no part of, all on one footprint
fn board() -> BoardMpns {
    let mut b = BoardMpns::default();
    for (mpn, r) in [(RESISTOR, "R1"), (CAPACITOR, "C1"), ("NOPE-1", "U1")] {
        b.mpns.insert(PartKey::mpn(mpn), vec![r.to_owned()]);
    }
    b.footprints.insert(
        HASH.to_owned(),
        HashedFootprint {
            lib_id: "Fix:R_0402".to_owned(),
            mpns: [RESISTOR, CAPACITOR, "NOPE-1"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        },
    );
    b
}

#[test]
fn facts_by_mpn() {
    let fake = FakeMx3::start();
    let mpns = vec![
        PartKey::mpn(RESISTOR),
        PartKey::mpn(CAPACITOR),
        PartKey::mpn("NOPE-1"),
    ];
    let a = source(&fake).parts(&mpns, &keys::names()).unwrap();
    assert_eq!(a.unknown_keys, ["inductance", "saturation_current"]);

    let r = &a.parts[0];
    assert_eq!((r.mpn.as_str(), r.matched), (RESISTOR, PartMatch::Found));
    assert_eq!(r.part.as_deref(), Some("12"));
    let d = r.decode.as_ref().unwrap();
    assert_eq!(d.scheme.as_deref(), Some("yageo.rc"));
    assert_eq!(d.segments.get("case_eia").map(String::as_str), Some("0402"));
    let keys: Vec<&str> = r.facts.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(keys, ["power", "resistance", "tolerance"]);
    let ohm = r.fact("resistance").unwrap();
    assert!(
        matches!(&ohm.value, FactValue::Number { value, unit } if (*value - 10e3).abs() < 1e-6 && unit == "Ω"),
        "{ohm:?}"
    );
    assert_eq!(
        (ohm.source.as_str(), ohm.rank, ohm.at.as_str()),
        ("digikey", 50, "2026-11-20T10:00:00+00:00")
    );
    assert!(
        matches!(&r.fact("power").unwrap().value, FactValue::Number { value, unit } if (*value - 0.0625).abs() < 1e-12 && unit == "W")
    );
    assert_eq!(
        r.fact("tolerance").unwrap().value,
        FactValue::Tolerance {
            minus: 1.0,
            plus: 1.0,
            unit: "%".to_owned()
        }
    );
    // the retracted dielectric is no value
    assert_eq!(
        r.missing,
        [
            "capacitance",
            "case_eia",
            "dielectric",
            "rated_voltage",
            "technology"
        ]
    );

    let c = &a.parts[1];
    assert!(
        matches!(&c.fact("capacitance").unwrap().value, FactValue::Number { value, unit } if (*value - 1e-7).abs() < 1e-18 && unit == "F")
    );
    assert!(
        matches!(&c.fact("tolerance").unwrap().value, FactValue::Tolerance { minus, plus, unit } if (*minus - 0.25e-12).abs() < 1e-20 && (*plus - 0.25e-12).abs() < 1e-20 && unit == "F"),
        "{:?}",
        c.fact("tolerance")
    );
    let x7r = c.fact("dielectric").unwrap();
    assert_eq!(
        x7r.value,
        FactValue::Text {
            text: "X7R".to_owned()
        }
    );
    assert!(x7r.conflict, "two sources disagree");
    assert_eq!(x7r.raw.as_deref(), Some("X7R (class 2)"));

    let u = &a.parts[2];
    assert_eq!((u.matched, u.part.as_ref()), (PartMatch::NotFound, None));
    assert_eq!(u.decode.as_ref().unwrap().outcome, "no_scheme");
    assert!(u.facts.is_empty());
    assert_eq!(u.missing.len(), keys::FACT_KEYS.len() - 2);

    let req = &fake.requests()[0];
    assert_eq!(req.path, "/params/by_mpn");
    assert_eq!(
        req.authorization.as_deref(),
        Some(format!("Bearer {TOKEN}").as_str())
    );
    let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
    assert_eq!(body["mpns"][0]["mpn"], RESISTOR);
    assert_eq!(
        body["keys"].as_array().unwrap().len(),
        keys::FACT_KEYS.len()
    );
}

#[test]
fn refresh_accept_refresh() {
    let fake = FakeMx3::start();
    let s = source(&fake);
    let lock = refresh_mpns(&board(), &s).unwrap();
    assert_eq!(lock.source, "mx3");
    assert_eq!(lock.parts.len(), 3);
    let status: Vec<(&str, Coverage)> = lock
        .footprints
        .iter()
        .map(|p| (p.mpn.as_str(), p.status))
        .collect();
    assert_eq!(
        status,
        [
            (CAPACITOR, Coverage::Uncovered),
            ("NOPE-1", Coverage::UnknownMpn),
            (RESISTOR, Coverage::Uncovered)
        ]
    );
    assert!(lock.footprints.iter().all(|p| p.footprint == "Fix:R_0402"));
    assert!(lock.reviews.is_empty());

    let family = Cover::family_of(lock.part(RESISTOR).unwrap()).unwrap();
    let stored = s
        .accept(&NewReview {
            hash: HASH.to_owned(),
            footprint: "Fix:R_0402".to_owned(),
            doc: Some("RC_Series".to_owned()),
            doc_rev: Some("rev12".to_owned()),
            page: Some(4),
            reviewer: "tester".to_owned(),
            reviewed_on: "2026-11-25".to_owned(),
            note: "pads per the land pattern table".to_owned(),
            covers: vec![
                family.clone(),
                Cover::Mpn {
                    mpn: CAPACITOR.to_owned(),
                },
            ],
            ..NewReview::default()
        })
        .unwrap();
    assert_eq!((stored.id, stored.hash.as_str()), (7, HASH));
    assert_eq!(
        stored.covers,
        [
            family.clone(),
            Cover::Mpn {
                mpn: CAPACITOR.to_owned()
            }
        ]
    );
    let sent = &fake.reviews()[0];
    assert_eq!(sent.source.doc.as_deref(), Some("RC_Series"));
    assert_eq!(sent.source.page, Some(4));
    assert_eq!(sent.reviewed_on.to_string(), "2026-11-25");

    let again = refresh_mpns(&board(), &s).unwrap();
    let covered: Vec<(&str, &[i64])> = again
        .footprints
        .iter()
        .map(|p| (p.mpn.as_str(), p.covered_by.as_slice()))
        .collect();
    assert_eq!(
        covered,
        [
            (CAPACITOR, &[7][..]),
            ("NOPE-1", &[][..]),
            (RESISTOR, &[7][..])
        ]
    );
    assert_eq!(again.reviews.len(), 1);
    assert_eq!(again.reviews[0].covers[0], family);
    // the lock reads back as written, and a third refresh writes the same text
    let text = again.to_toml();
    assert_eq!(PartsLock::parse("parts.lock.toml", &text).unwrap(), again);
    assert_eq!(refresh_mpns(&board(), &s).unwrap().to_toml(), text);
}

#[test]
fn an_mpn_of_two_makers_and_a_stub() {
    let fake = FakeMx3::start();
    let asked = [
        PartKey::mpn(SHARED),
        PartKey::new(SHARED, Some(MAKER_A_SYNONYM)),
        PartKey::new(SHARED, Some(MAKER_B)),
        PartKey::new(SHARED, Some("Nobody Corp")),
        PartKey::new(RESISTOR, Some(MAKER_B)),
    ];
    let a = source(&fake).parts(&asked, &keys::names()).unwrap();
    let by = |i: usize| &a.parts[i];

    // No maker: several parts, each maker named, nothing resolved
    let all = [
        (SHARED_A_ID, MAKER_A),
        (SHARED_B_ID, MAKER_B),
        (SHARED_STUB_ID, NO_MAKER),
    ];
    let makers = |i: usize| -> Vec<(String, String)> {
        by(i)
            .makers
            .iter()
            .map(|m| (m.part.clone(), m.maker.clone()))
            .collect()
    };
    let all: Vec<(String, String)> = all
        .iter()
        .map(|(id, m)| (id.to_string(), (*m).to_owned()))
        .collect();
    assert_eq!(by(0).matched, PartMatch::NotUnique);
    assert_eq!(by(0).part, None);
    assert!(by(0).facts.is_empty());
    assert_eq!(makers(0), all);

    // A synonym chooses its company's part, with that part's facts and id; no choices kept
    assert_eq!(by(1).maker.as_deref(), Some(MAKER_A_SYNONYM));
    assert_eq!(by(1).matched, PartMatch::Found);
    assert_eq!(by(1).part, Some(SHARED_A_ID.to_string()));
    assert!(by(1).makers.is_empty());
    let watts = |i: usize| match &by(i).fact("power").unwrap().value {
        FactValue::Number { value, .. } => *value,
        other => panic!("{other:?}"),
    };
    assert!((watts(1) - 0.0625).abs() < 1e-9);
    assert_eq!(by(2).part, Some(SHARED_B_ID.to_string()));
    assert!((watts(2) - 0.1).abs() < 1e-9);

    // A maker mx3 does not know resolves nothing
    assert_eq!(by(3).matched, PartMatch::UnknownMaker);
    assert!(by(3).facts.is_empty());
    assert_eq!(makers(3), all);

    // A known maker that has no part of the MPN: the maker that has it to choose from
    assert_eq!(by(4).matched, PartMatch::NotFound);
    assert_eq!(by(4).maker_names(), ["Yageo"]);

    let req = fake.requests().pop().unwrap();
    let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
    assert_eq!(body["mpns"][0]["manufacturer"], serde_json::Value::Null);
    assert_eq!(body["mpns"][1]["manufacturer"], MAKER_A_SYNONYM);
}

#[test]
fn a_review_is_checked_before_it_is_sent() {
    let fake = FakeMx3::start();
    let e = source(&fake)
        .accept(&NewReview {
            hash: "fp1:short".to_owned(),
            reviewer: "tester".to_owned(),
            reviewed_on: "2026-11-25".to_owned(),
            covers: vec![Cover::Mpn {
                mpn: RESISTOR.to_owned(),
            }],
            ..NewReview::default()
        })
        .unwrap_err();
    assert!(e.to_string().contains("no review hash"), "{e}");
    assert!(fake.requests().is_empty());
}

#[test]
fn a_wrong_token_is_refused_and_never_shown() {
    let fake = FakeMx3::start();
    let secret = "mx3_wrong-secret-token";
    let s = Mx3Source::connect(Connection::new(fake.url(), Some(secret.to_owned()))).unwrap();
    let e = s
        .parts(&[PartKey::mpn(RESISTOR)], &keys::names())
        .unwrap_err();
    let shown = format!("{e} {e:?} {s:?}");
    assert!(shown.contains("401"), "{shown}");
    assert!(shown.contains(fake.url()), "{shown}");
    assert!(!shown.contains(secret), "{shown}");
}

#[test]
fn no_token_no_source() {
    let e = Mx3Source::connect(Connection::new("http://127.0.0.1:9", None)).unwrap_err();
    assert!(e.to_string().contains("MX3_TOKEN"), "{e}");
}

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("erc-parts-mx3-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn url_and_token_as_the_mx3_cli_finds_them() {
    let home = scratch("home");
    let tpm = home.join("git/tpm");
    std::fs::create_dir_all(&tpm).unwrap();
    std::fs::write(
        tpm.join("config.toml"),
        "[models]\nx = 1\n\n[share]\nmx3_url = \"http://from-config:3000/\" # prod\n",
    )
    .unwrap();
    std::fs::create_dir_all(home.join(".config/mx3")).unwrap();
    std::fs::write(home.join(".config/mx3/token"), "  mx3_from-file\n").unwrap();
    let env = Environment {
        home: Some(home.display().to_string()),
        ..Environment::default()
    };

    let c = Connection::resolve(&env);
    assert_eq!(c.url, "http://from-config:3000");
    assert_eq!(c.url_from, tpm.join("config.toml").display().to_string());
    assert!(c.has_token());
    assert_eq!(c.token(), Some("mx3_from-file"));
    assert!(!format!("{c:?}").contains("mx3_from-file"), "{c:?}");

    let c = Connection::resolve(&Environment {
        mx3_url: Some("http://from-env:3000".to_owned()),
        mx3_token: Some("mx3_from-env".to_owned()),
        ..env.clone()
    });
    assert_eq!(
        (c.url.as_str(), c.url_from.as_str()),
        ("http://from-env:3000", "MX3_URL")
    );
    assert!(!format!("{c:?}").contains("mx3_from-env"));

    // TPM_ROOT names another tpm checkout; one without the key falls back to the default
    let other = scratch("other-tpm");
    std::fs::write(other.join("config.toml"), "[share]\nother = 1\n").unwrap();
    let c = Connection::resolve(&Environment {
        tpm_root: Some(other.display().to_string()),
        ..env.clone()
    });
    assert_eq!(
        (c.url.as_str(), c.url_from.as_str()),
        ("http://localhost:3000", "default")
    );

    let bare = scratch("bare-home");
    let c = Connection::resolve(&Environment {
        home: Some(bare.display().to_string()),
        mx3_token: Some("   ".to_owned()),
        ..Environment::default()
    });
    assert!(
        !c.has_token(),
        "an empty MX3_TOKEN and no file give no token"
    );
    assert_eq!(c.token(), None);
}
