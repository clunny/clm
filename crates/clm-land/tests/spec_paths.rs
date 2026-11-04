//! A spec branch holds spec paths only (land/spec-branch-holds-spec-paths).

mod common;

use common::{Fixture, refused};

#[test]
fn code_on_a_spec_branch() {
    // req:land/spec-branch-holds-spec-paths@01d6d999
    let f = Fixture::new("");
    f.write("gate-out", "code\n");
    f.branch("spec/x", &[("crates/x/src/lib.rs", "fn x() {}\n")]);
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])));
    assert!(why.contains("crates/x/src/lib.rs"), "{why}");
    assert!(why.contains("spec paths"), "{why}");
    assert!(f.lines("check.log").is_empty());
    assert_eq!(f.remote("main"), old);
    assert!(f.records().is_empty());
}

#[test]
fn a_suggestion_on_a_spec_branch() {
    // req:land/spec-branch-holds-spec-paths@01d6d999
    let f = Fixture::new("");
    f.write("gate-out", "code\n");
    f.branch("spec/x", &[("backlog/x.md", "x\n")]);
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn code_on_an_impl_branch() {
    // req:land/spec-branch-holds-spec-paths@01d6d999
    let f = Fixture::new("");
    f.branch("impl/x", &[("crates/x/src/lib.rs", "fn x() {}\n")]);
    clm_land::land(&f.work(), Some("impl/x"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}
