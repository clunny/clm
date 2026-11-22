mod common;

use clm_ledger::Body;
use common::{clone_with_origin, env, sh};

#[test]
fn the_same_record_as_a_literal() {
    // req:ledger/approved-constructor@7603f36e
    let dir = tempfile::tempdir().unwrap();
    let a = clone_with_origin(dir.path(), "a", "A");
    let fingerprint = "f".repeat(64);
    let literal = Body::Approved {
        fingerprint: fingerprint.clone(),
        branch: "spec/x".into(),
        change: Some("20260101-x".into()),
        clears: None,
    };
    let built = Body::approved(
        fingerprint.clone(),
        "spec/x".into(),
        Some("20260101-x".into()),
    );
    assert_eq!(built, literal);
    let home = dir.path();
    let from_built = clm_ledger::append(&a, built, &env(home, &[])).unwrap();
    let from_literal = clm_ledger::append(&a, literal.clone(), &env(home, &[])).unwrap();
    let fold = clm_ledger::fold(&a).unwrap();
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    let held = &fold.approvals[&fingerprint];
    assert_eq!(held.len(), 2);
    let body_of = |rid: &str| &held.iter().find(|r| r.id == rid).unwrap().body;
    assert_eq!(body_of(&from_built.record.id), &literal);
    assert_eq!(
        body_of(&from_built.record.id),
        body_of(&from_literal.record.id)
    );
}

#[test]
fn clears_set_through_the_constructor() {
    // req:ledger/request-cleared@8069ffc9
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let a = clone_with_origin(home, "a", "A");
    let plain = Body::approved("f".into(), "spec/x".into(), Some("20260101-x".into()));
    let naming = plain.clone().with_clears(vec!["r1".into()]);
    let plain = clm_ledger::append(&a, plain, &env(home, &[])).unwrap();
    let naming = clm_ledger::append(&a, naming, &env(home, &[])).unwrap();
    let branch = format!("clm/ledger/{}", plain.record.writer);
    let text = sh(&a, &["show", &format!("{branch}:records.jsonl")]);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert!(!lines[0].contains("clears"), "{}", lines[0]);
    let fold = clm_ledger::fold(&a).unwrap();
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    let held = fold
        .records
        .iter()
        .find(|r| r.id == naming.record.id)
        .unwrap();
    assert!(
        matches!(&held.body, Body::Approved { clears: Some(c), .. } if *c == ["r1"]),
        "{held:?}"
    );
    // Any other body comes back as it was.
    let checked = Body::Checked {
        code_key: "k".into(),
        command: "true".into(),
        basis: None,
    };
    assert_eq!(checked.clone().with_clears(vec!["r1".into()]), checked);
}
