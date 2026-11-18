//! Whether a fingerprint's approvals count, and why each doesn't
//! (land.approval/approvals-counted).

mod common;

use std::path::PathBuf;

use clm_ledger::{Body, Fold};
use common::{Fixture, fold, keygen, line};

const AT: &str = "2026-11-17T10:00:00.000Z";

/// A branch `spec/x` with one commit authored as `author`; its head and fingerprint.
fn branch(f: &Fixture, author: &str) -> (String, String) {
    let work = f.work();
    f.git(&work, &["config", "user.name", author]);
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    f.git(&work, &["config", "user.name", "Christopher Lunny"]);
    let head = f.git(&work, &["rev-parse", "origin/spec/x"]);
    (head, f.fingerprint("spec/x"))
}

/// Moves `origin/main` on to a commit whose `docs/allowed_signers` lists `christopher-lunny`
/// and `dana`, and the work clone with it. Returns the base commit and `christopher-lunny`'s
/// private key.
fn signed_base(f: &Fixture) -> (String, PathBuf) {
    let keys = f.path("keys");
    let (christopher, christopher_pub) = keygen(&keys, "christopher");
    let (_, dana_pub) = keygen(&keys, "dana");
    f.advance_main(
        "docs/allowed_signers",
        &format!("christopher-lunny {christopher_pub}\ndana {dana_pub}\n"),
    );
    let work = f.work();
    f.git(&work, &["fetch", "-q", "origin"]);
    f.git(&work, &["merge", "-q", "--ff-only", "origin/main"]);
    (f.git(&work, &["rev-parse", "origin/main"]), christopher)
}

fn approved(fingerprint: &str) -> Body {
    Body::Approved {
        fingerprint: fingerprint.into(),
        branch: "spec/x".into(),
        change: Some("x".into()),
    }
}

/// The approvals of `fingerprint` in `fold` counted in the work clone: `(accepted, refused)`.
fn count(
    f: &Fixture,
    records: &Fold,
    fingerprint: &str,
    head: &str,
    signed: Option<&str>,
    other: bool,
) -> (bool, Vec<String>) {
    let approvals = f
        .with_repo(|repo| repo.approval(records, fingerprint, head, signed, other))
        .unwrap();
    (approvals.accepted, approvals.refused)
}

#[test]
fn an_authors_own_approval() {
    // req:land.approval/approvals-counted@ebdf5eea
    let f = Fixture::new("");
    let (head, fp) = branch(&f, "Christopher Lunny");
    let records = fold(&[line(
        "a1",
        "christopher-lunny/1a2b3c4d",
        AT,
        approved(&fp),
        None,
    )]);
    let (accepted, refused) = count(&f, &records, &fp, &head, None, true);
    assert!(!accepted);
    let [why] = &refused[..] else {
        panic!("not one reason: {refused:?}")
    };
    assert!(why.contains("christopher-lunny/1a2b3c4d"), "{why}");
    assert!(why.contains(AT), "the record's time: {why}");
    assert!(why.contains("approval = \"other\""), "{why}");
    assert!(why.contains("authors"), "{why}");
    let (accepted, refused) = count(&f, &records, &fp, &head, None, false);
    assert!(accepted && refused.is_empty(), "{refused:?}");
}

#[test]
fn another_person() {
    // req:land.approval/approvals-counted@ebdf5eea
    let f = Fixture::new("");
    let (head, fp) = branch(&f, "Christopher Lunny");
    let records = fold(&[line("a1", "dana/9f9f9f9f", AT, approved(&fp), None)]);
    let (accepted, refused) = count(&f, &records, &fp, &head, None, true);
    assert!(accepted && refused.is_empty(), "{refused:?}");
}

#[test]
fn unsigned_in_a_signed_repository() {
    // req:land.approval/approvals-counted@ebdf5eea
    let f = Fixture::new("");
    let (base, _) = signed_base(&f);
    let (head, fp) = branch(&f, "Christopher Lunny");
    let records = fold(&[line("a1", "dana/9f9f9f9f", AT, approved(&fp), None)]);
    let (accepted, refused) = count(&f, &records, &fp, &head, Some(&base), false);
    assert!(!accepted);
    let [why] = &refused[..] else {
        panic!("not one reason: {refused:?}")
    };
    assert!(why.contains("not signed by its writer"), "{why}");
    assert!(why.contains("clm approve"), "{why}");
}

#[test]
fn a_forged_name() {
    // req:land.approval/approvals-counted@ebdf5eea
    let f = Fixture::new("");
    let (base, christopher) = signed_base(&f);
    let (head, fp) = branch(&f, "Zed");
    let key = Some((christopher.as_path(), "clm-approve"));
    let records = fold(&[line("a1", "dana/1a2b3c4d", AT, approved(&fp), key)]);
    let (accepted, refused) = count(&f, &records, &fp, &head, Some(&base), false);
    assert!(!accepted);
    assert!(
        refused
            .iter()
            .all(|w| w.contains("not signed by its writer")),
        "{refused:?}"
    );
    // The same record signed by its writer's own key counts: the key, not the line, failed.
    let records = fold(&[line(
        "a1",
        "christopher-lunny/1a2b3c4d",
        AT,
        approved(&fp),
        key,
    )]);
    let (accepted, _) = count(&f, &records, &fp, &head, Some(&base), false);
    assert!(accepted);
}

#[test]
fn the_landers_own_approval() {
    // req:land.approval/approvals-counted@ebdf5eea
    let f = Fixture::new("");
    let (base, christopher) = signed_base(&f);
    let (head, fp) = branch(&f, "Zed");
    let key = Some((christopher.as_path(), "clm-approve"));
    let records = fold(&[line(
        "a1",
        "christopher-lunny/1a2b3c4d",
        AT,
        approved(&fp),
        key,
    )]);
    let (accepted, refused) = count(&f, &records, &fp, &head, Some(&base), true);
    assert!(!accepted);
    let [why] = &refused[..] else {
        panic!("not one reason: {refused:?}")
    };
    assert!(why.contains("is the lander"), "{why}");
}

#[test]
fn no_approval() {
    // req:land.approval/approvals-counted@ebdf5eea
    let f = Fixture::new("");
    let (head, fp) = branch(&f, "Christopher Lunny");
    let records = fold(&[line("a1", "dana/9f9f9f9f", AT, approved("another"), None)]);
    for other in [false, true] {
        let (accepted, refused) = count(&f, &records, &fp, &head, None, other);
        assert!(!accepted && refused.is_empty(), "{refused:?}");
    }
}
