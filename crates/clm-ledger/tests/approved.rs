mod common;

use clm_ledger::Body;
use common::{clone_with_origin, env};

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
