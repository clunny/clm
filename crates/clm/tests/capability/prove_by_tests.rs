//! `prove-by-tests`: for every requirement, whether a passing test proves it at its current
//! version.

use std::fs;

use crate::{Repo, printed};

/// The first 8 hex digits of requirement `a/<slug>`'s hash, as `clm hashes a` prints it.
fn version(repo: &Repo, slug: &str) -> String {
    repo.hash("a", &format!("requirement:{slug}"))[..8].to_string()
}

#[test]
fn reworded_requirement() {
    // req:capability/prove-by-tests@9c0b4ad0
    let repo = Repo::init();
    let marker = format!("{}{}", concat!("req", ":a/x@"), version(&repo, "x"));
    repo.write(
        "tests/x.rs",
        &format!("#[test]\nfn x() {{\n    // {marker}\n}}\n"),
    );
    repo.commit("a test proving x");
    let current = repo.clm(&["coverage"]);
    let text = String::from_utf8_lossy(&current.stdout).into_owned();
    assert!(current.status.success(), "{}", printed(&current));
    assert!(
        text.lines()
            .any(|l| l.starts_with("a/x ") && l.split_whitespace().any(|w| w == "current")),
        "{text}"
    );

    let spec = fs::read_to_string(repo.root.join("specs/a/SPEC.md")).unwrap();
    repo.write(
        "specs/a/SPEC.md",
        &spec.replace("A SHALL x.", "A SHALL x twice."),
    );
    repo.commit("reword x");

    let out = repo.clm(&["coverage"]);
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "{}", printed(&out));
    assert!(
        text.lines().any(|l| l.starts_with("a/x ")
            && l.split_whitespace().any(|w| w == "behind")
            && l.contains("tests/x.rs")),
        "{text}"
    );
}
