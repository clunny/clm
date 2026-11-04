//! `catch-drift`: deterministic checks that the spec and the code agree.

use crate::{Repo, printed};

#[test]
fn test_cites_a_removed_requirement() {
    // req:capability/catch-drift@5a7a7ea7
    let repo = Repo::init();
    let clean = repo.clm(&["check"]);
    assert!(clean.status.success(), "{}", printed(&clean));

    // A test citing `a/gone`, which no spec holds.
    let marker = concat!("req", ":a/gone");
    repo.write(
        "tests/t.rs",
        &format!("#[test]\nfn t() {{\n    // {marker}\n}}\n"),
    );
    repo.commit("a test citing a removed requirement");

    let out = repo.clm(&["check"]);
    let text = printed(&out);
    assert!(!out.status.success(), "{text}");
    assert!(
        text.lines().any(|l| l.contains("tests/t.rs:3")
            && l.contains("check/dangling-markers")
            && l.contains("a/gone")),
        "{text}"
    );
}
