mod common;

use clm_ledger::{Body, LandKind, Recheck, TimedStep};
use common::{clone_with_origin, env};

#[test]
fn the_same_record_as_a_literal() {
    // req:ledger/landed-constructor@cbe1d118
    let dir = tempfile::tempdir().unwrap();
    let a = clone_with_origin(dir.path(), "a", "A");
    let home = dir.path();
    let commit = "c".repeat(40);
    let fingerprint = "f".repeat(64);
    let code_key = "k".repeat(64);
    let changes = vec!["20260101-x".to_string(), "20260102-y".to_string()];
    let step = TimedStep {
        name: "tests".into(),
        seconds: 7,
        selected_by: vec!["specs/x/SPEC.md".into()],
    };
    let full_literal = Body::Landed {
        commit: commit.clone(),
        branch: "spec/x".into(),
        kind: LandKind::Spec,
        light: false,
        review: true,
        fingerprint: fingerprint.clone(),
        code_key: Some(code_key.clone()),
        changes: changes.clone(),
        check_seconds: Some(42),
        steps: vec![step.clone()],
        recheck: Some(Recheck::Overlap),
        recheck_files: Some(3),
    };
    let bare_literal = Body::Landed {
        commit: commit.clone(),
        branch: "spec/x".into(),
        kind: LandKind::Spec,
        light: false,
        review: true,
        fingerprint: fingerprint.clone(),
        code_key: None,
        changes: vec![],
        check_seconds: None,
        steps: vec![],
        recheck: None,
        recheck_files: None,
    };

    let full = Body::landed(
        commit.clone(),
        "spec/x".into(),
        LandKind::Spec,
        true,
        fingerprint.clone(),
    )
    .with_code_key(Some(code_key.clone()))
    .with_changes(changes.clone())
    .with_check_seconds(Some(42))
    .with_steps(vec![step.clone()])
    .with_recheck(Some(Recheck::Overlap))
    .with_recheck_files(Some(3));
    let bare = Body::landed(
        commit.clone(),
        "spec/x".into(),
        LandKind::Spec,
        true,
        fingerprint.clone(),
    );

    let from_full = clm_ledger::append(&a, full, &env(home, &[])).unwrap();
    let from_bare = clm_ledger::append(&a, bare, &env(home, &[])).unwrap();
    let fold = clm_ledger::fold(&a).unwrap();
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    let held = &fold.lands[&commit];
    assert_eq!(held.len(), 2);
    let body_of = |rid: &str| &held.iter().find(|r| r.id == rid).unwrap().body;
    assert_eq!(body_of(&from_full.record.id), &full_literal);
    assert_eq!(body_of(&from_bare.record.id), &bare_literal);

    let checked = Body::Checked {
        code_key: code_key.clone(),
        command: "make check".into(),
        basis: None,
    };
    assert_eq!(checked.clone().with_code_key(Some("other".into())), checked);
}

/// Each `with_` method sets its one field and leaves every other as it was, and leaves a
/// body of another kind unchanged.
#[test]
fn each_with_sets_only_its_field() {
    // req:ledger/landed-constructor@cbe1d118
    let base = || {
        Body::landed(
            "c".into(),
            "impl/x".into(),
            LandKind::Code,
            false,
            "f".into(),
        )
    };
    let literal = |f: &dyn Fn(&mut Body)| {
        let mut b = Body::Landed {
            commit: "c".into(),
            branch: "impl/x".into(),
            kind: LandKind::Code,
            light: false,
            review: false,
            fingerprint: "f".into(),
            code_key: None,
            changes: vec![],
            check_seconds: None,
            steps: vec![],
            recheck: None,
            recheck_files: None,
        };
        f(&mut b);
        b
    };
    let step = TimedStep {
        name: "s".into(),
        seconds: 1,
        selected_by: vec![],
    };
    macro_rules! set {
        ($field:ident = $value:expr) => {
            |b: &mut Body| {
                if let Body::Landed { $field, .. } = b {
                    *$field = $value;
                }
            }
        };
    }
    assert_eq!(
        base().with_code_key(Some("k".into())),
        literal(&set!(code_key = Some("k".into())))
    );
    assert_eq!(
        base().with_changes(vec!["x".into()]),
        literal(&set!(changes = vec!["x".into()]))
    );
    assert_eq!(
        base().with_check_seconds(Some(5)),
        literal(&set!(check_seconds = Some(5)))
    );
    assert_eq!(
        base().with_steps(vec![step.clone()]),
        literal(&set!(steps = vec![step.clone()]))
    );
    assert_eq!(
        base().with_recheck(Some(Recheck::Selected)),
        literal(&set!(recheck = Some(Recheck::Selected)))
    );
    assert_eq!(
        base().with_recheck_files(Some(9)),
        literal(&set!(recheck_files = Some(9)))
    );

    let other = Body::approved("f".into(), "spec/x".into(), None);
    assert_eq!(
        other
            .clone()
            .with_code_key(Some("k".into()))
            .with_changes(vec!["x".into()])
            .with_check_seconds(Some(5))
            .with_steps(vec![step])
            .with_recheck(Some(Recheck::Checked))
            .with_recheck_files(Some(1)),
        other
    );
}
