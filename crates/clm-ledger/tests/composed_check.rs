mod common;

use clm_ledger::{Basis, Body, fold_records};
use common::{clone_with_origin, env, sh};

fn checked(key: &str, basis: Option<Basis>) -> Body {
    Body::Checked {
        code_key: key.into(),
        command: "x".into(),
        basis,
    }
}

#[test]
fn a_basis_round_trips_and_a_whole_check_leaves_it_out() {
    // req:ledger/composed-check@af377cb9
    let dir = tempfile::tempdir().unwrap();
    let a = clone_with_origin(dir.path(), "a", "A");
    let basis = Basis {
        head: "h".into(),
        base: "b".into(),
    };
    let env = env(dir.path(), &[]);
    clm_ledger::append(&a, checked("whole", None), &env).unwrap();
    let appended = clm_ledger::append(&a, checked("m", Some(basis.clone())), &env).unwrap();
    let branch = format!("clm/ledger/{}", appended.record.writer);
    let text = sh(&a, &["show", &format!("{branch}:records.jsonl")]);
    let lines: Vec<&str> = text.lines().collect();
    assert!(!lines[0].contains("basis"), "{text}");
    assert!(
        lines[1].contains(r#""basis":{"head":"h","base":"b"}"#),
        "{text}"
    );
    let fold = clm_ledger::fold(&a).unwrap();
    assert!(matches!(
        &fold.checks["m"][0].body,
        Body::Checked { basis: Some(got), .. } if *got == basis
    ));
    assert!(matches!(
        &fold.checks["whole"][0].body,
        Body::Checked { basis: None, .. }
    ));
}

#[test]
fn a_composed_record_is_among_the_checks_of_its_key() {
    // req:ledger/composed-check@af377cb9
    let line = r#"{"id":"1","writer":"a/1","at":"2026-01-01T00:00:00.000Z","kind":"checked","code_key":"m","command":"x","basis":{"head":"h","base":"b"}}"#;
    let fold = fold_records(&[("one", line)]);
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    assert_eq!(fold.checks["m"].len(), 1);
}
