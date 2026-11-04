mod common;

use clm_ledger::Body;
use common::{Fixture, refused};

#[test]
fn approve_a_spec_branch() {
    // req:land.approval/approve@0146230b
    let f = Fixture::new("");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let fp = f.fingerprint("spec/x");
    let out = clm_approval::approve(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(out.lines, [format!("approved {fp} spec/x")]);
    let records = f.records();
    match &records[..] {
        [r] => {
            assert!(r.writer.starts_with("christopher-lunny/"), "{}", r.writer);
            assert_eq!(
                r.body,
                Body::Approved {
                    fingerprint: fp,
                    branch: "spec/x".into(),
                    change: Some("x".into()),
                }
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn approve_without_a_change() {
    // req:land.approval/approve@0146230b
    let f = Fixture::new("");
    f.branch("spec/x", &[("specs/x/SPEC.md", "x\n")]);
    clm_approval::approve(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    match &f.records()[..] {
        [r] => assert!(matches!(&r.body, Body::Approved { change: None, .. })),
        other => panic!("{other:?}"),
    }
}

#[test]
fn nothing_to_approve() {
    // req:land.approval/approve@0146230b
    let f = Fixture::new("");
    let work = f.work();
    f.git(&work, &["checkout", "-q", "-b", "spec/x"]);
    f.git(&work, &["push", "-q", "origin", "spec/x"]);
    refused(clm_approval::approve(&work, None, &f.env(&[])));
    assert!(f.records().is_empty());
}

#[test]
fn main_refused() {
    // req:land.approval/approve@0146230b
    let f = Fixture::new("");
    let why = refused(clm_approval::approve(&f.work(), None, &f.env(&[])));
    assert!(why.contains("`main`"), "{why}");
    assert!(f.records().is_empty());
}
