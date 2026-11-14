//! A build holds its own files (land/build-holds-own-files).

mod common;

use common::{Fixture, refused};

const REQ: &str = "\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: holds\n- ok\n";

fn root() -> String {
    "---\ntitle: Root\nstatus: active\nowners: []\npaths: [\"Cargo.lock\"]\nwiring: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n".into()
}

fn component(title: &str, paths: &str) -> String {
    format!(
        "---\ntitle: {title}\nstatus: specd\nowners: [\"@a\"]\npaths: [\"{paths}\"]\n---\n\n## Contract\nc\n\n## Design\nthe spine\n\n## Requirements\n{REQ}"
    )
}

/// A root with `wiring: []`, components `a` and `b`, pushed on `main`.
fn with_specs() -> Fixture {
    let f = Fixture::new("");
    f.commit_files(
        &[
            ("specs/SPEC.md", &root()),
            ("specs/a/SPEC.md", &component("A", "crates/a/**")),
            ("specs/b/SPEC.md", &component("B", "crates/b/**")),
            ("crates/a/src/lib.rs", "a\n"),
            ("crates/b/src/lib.rs", "b\n"),
        ],
        "specs",
    );
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    f
}

const BOTH: [(&str, &str); 2] = [
    ("crates/a/src/lib.rs", "a2\n"),
    ("crates/b/src/lib.rs", "b2\n"),
];

#[test]
fn another_components_source_refuses_the_land() {
    // req:land/build-holds-own-files@848d8f4b
    let f = with_specs();
    f.branch("impl/a", &BOTH);
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert!(
        why.starts_with("crates/b/src/lib.rs: land/build-holds-own-files: owned by `b`;"),
        "{why}"
    );
    assert!(!why.contains("crates/a/src/lib.rs"), "{why}");
    assert!(f.lines("check.log").is_empty());
    assert!(f.lines("clm-check.log").is_empty());
    assert_eq!(f.remote("main"), old);
    assert!(f.records().is_empty());
}

#[test]
fn the_builds_pre_check_prints_the_same_line() {
    // req:land/build-holds-own-files@848d8f4b
    let f = with_specs();
    f.branch("impl/a", &BOTH);
    f.git(&f.work(), &["checkout", "-q", "impl/a"]);
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert!(
        why.contains("crates/b/src/lib.rs: land/build-holds-own-files: owned by `b`;"),
        "{why}"
    );
    assert!(f.lines("check.log").is_empty());
    assert!(f.lines("clm-check.log").is_empty());
}

#[test]
fn its_own_files_land() {
    // req:land/build-holds-own-files@848d8f4b
    let f = with_specs();
    f.branch("impl/a", &BOTH[..1]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn a_branch_of_another_name_is_not_refused() {
    // req:land/build-holds-own-files@848d8f4b
    let f = with_specs();
    f.branch("tidy-b", &BOTH);
    clm_land::land(&f.work(), Some("tidy-b"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
    let g = with_specs();
    g.branch("impl/archive/x", &BOTH);
    clm_land::land(&g.work(), Some("impl/archive/x"), &g.env(&[])).unwrap();
    assert_eq!(g.landed().len(), 1);
}
