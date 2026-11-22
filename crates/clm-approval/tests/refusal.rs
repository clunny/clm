//! The refusal a request for changes makes (land.approval/request-refusal).

mod common;

use std::path::Path;

use clm_ledger::Body;
use common::{Fixture, fold, keygen, line};

const AT: &str = "2026-11-17T10:00:00.000Z";

fn request(text: &str) -> String {
    let body = Body::ChangesRequested {
        branch: "spec/x".into(),
        fingerprint: "f".into(),
        text: text.into(),
    };
    line("r1", "alice/web", AT, body, None)
}

fn comment(id: &str, target: Option<&str>) -> String {
    let body = Body::Commented {
        branch: "spec/x".into(),
        fingerprint: "f".into(),
        target: target.map(str::to_string),
        text: "Say why".into(),
    };
    line(id, "bob/web", AT, body, None)
}

/// An approval of `f` by `writer` that names the request `r1` as one it clears, at the
/// request's own time, so only the name can clear it; signed with `key` when given.
fn approval_signed(writer: &str, key: Option<&Path>) -> String {
    let body = Body::approved("f".into(), "spec/x".into(), None).with_clears(vec!["r1".into()]);
    line(
        "a1",
        writer,
        AT,
        body,
        key.map(|k| (k, clm_approval::APPROVE)),
    )
}

fn approval(writer: &str) -> String {
    approval_signed(writer, None)
}

/// The fold of `held`: a request and a comment on `n/r`, then `more`.
fn held_fold(more: &[String]) -> clm_ledger::Fold {
    let mut lines = vec![request("Name the rule"), comment("c1", Some("n/r"))];
    lines.extend_from_slice(more);
    fold(&lines)
}

#[test]
fn held() {
    // req:land.approval/request-refusal@8d639b0f
    // A comment on an earlier diff isn't shown.
    let stale = Body::Commented {
        branch: "spec/x".into(),
        fingerprint: "g".into(),
        target: None,
        text: "Stale".into(),
    };
    let fold = held_fold(&[comment("c2", None), line("c3", "bob/web", AT, stale, None)]);
    let why = clm_approval::held(&fold, "spec/x", "f").expect("held");
    let lines: Vec<&str> = why.lines().collect();
    assert_eq!(lines.len(), 5, "{why}");
    assert!(!lines[0].contains("spec/x:"), "a heading first: {why}");
    assert_eq!(
        lines[1],
        format!("spec/x: land/changes-requested: alice/web at {AT}: Name the rule")
    );
    assert_eq!(
        lines[2],
        format!("spec/x: land/comment: bob/web on n/r at {AT}: Say why")
    );
    assert_eq!(
        lines[3],
        format!("spec/x: land/comment: bob/web on the change at {AT}: Say why")
    );
    assert!(lines[4].contains("clm approve spec/x"), "{why}");
    assert!(lines[4].contains("push"), "{why}");
}

#[test]
fn another_fingerprint() {
    // req:land.approval/request-refusal@8d639b0f
    let fold = held_fold(&[]);
    assert_eq!(clm_approval::held(&fold, "spec/x", "g"), None);
    // Nor on another branch.
    assert_eq!(clm_approval::held(&fold, "spec/y", "f"), None);
}

#[test]
fn cleared_by_the_requester() {
    // req:land.approval/request-refusal@8d639b0f
    let fold = held_fold(&[approval("alice/1a2b3c4d")]);
    assert_eq!(clm_approval::held(&fold, "spec/x", "f"), None);
}

#[test]
fn another_persons_approval_doesnt_clear_it() {
    // req:land.approval/request-refusal@8d639b0f
    let fold = held_fold(&[approval("dana/9f9f9f9f")]);
    let why = clm_approval::held(&fold, "spec/x", "f").expect("held");
    assert!(
        why.contains("spec/x: land/changes-requested: alice/web at"),
        "{why}"
    );
}

#[test]
fn a_text_over_several_lines() {
    // req:land.approval/request-refusal@8d639b0f
    let comment = line(
        "c1",
        "bob/web",
        AT,
        Body::Commented {
            branch: "spec/x".into(),
            fingerprint: "f".into(),
            target: None,
            text: "c\n  d".into(),
        },
        None,
    );
    let fold = fold(&[request("a\n  b"), comment]);
    let why = clm_approval::held(&fold, "spec/x", "f").expect("held");
    let find = |kind: &str| why.lines().find(|l| l.contains(kind)).unwrap().to_string();
    let request = find("land/changes-requested");
    assert!(request.ends_with(": a b"), "{request}");
    let comment = find("land/comment");
    assert!(comment.ends_with(": c d"), "{comment}");
}

#[test]
fn an_unsigned_approval_in_a_signed_repository() {
    // req:land.approval/request-refusal@8d639b0f
    let f = Fixture::new("");
    let (alice, alice_pub) = keygen(&f.path("keys"), "alice");
    let unsigned_base = f.git(&f.work(), &["rev-parse", "origin/main"]);
    f.advance_main("docs/allowed_signers", &format!("alice {alice_pub}\n"));
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    let base = f.git(&f.work(), &["rev-parse", "origin/main"]);
    let unsigned = held_fold(&[approval("alice/1a2b3c4d")]);
    let signed = held_fold(&[approval_signed("alice/1a2b3c4d", Some(&alice))]);
    let (first, second, unsigned_repo) = f.with_repo(|repo| {
        (
            repo.held(&unsigned, "spec/x", "f", Some(&base)).unwrap(),
            repo.held(&signed, "spec/x", "f", Some(&base)).unwrap(),
            repo.held(&unsigned, "spec/x", "f", Some(&unsigned_base))
                .unwrap(),
        )
    });
    let why = first.expect("an unsigned approval clears nothing where the base is signed");
    assert!(
        why.contains("spec/x: land/changes-requested: alice/web at"),
        "{why}"
    );
    assert_eq!(second, None, "the signed approval clears it");
    // At a base with no `docs/allowed_signers`, the unsigned approval clears it.
    assert_eq!(unsigned_repo, None);
}

#[test]
fn repo_held_without_a_signed_base() {
    // req:land.approval/request-refusal@8d639b0f
    let f = Fixture::new("");
    let open = held_fold(&[]);
    let cleared = held_fold(&[approval("alice/1a2b3c4d")]);
    let other = held_fold(&[approval("dana/9f9f9f9f")]);
    let multiline = fold(&[request("a\n  b")]);
    let ([open, cleared, other, multiline], elsewhere) = f.with_repo(|repo| {
        let at = |fold: &clm_ledger::Fold| repo.held(fold, "spec/x", "f", None).unwrap();
        (
            [&open, &cleared, &other, &multiline].map(at),
            repo.held(&open, "spec/x", "g", None).unwrap(),
        )
    });
    assert_eq!(elsewhere, None, "another fingerprint");
    let line = multiline.expect("held");
    assert!(line.contains("alice/web at"), "{line}");
    assert!(line.lines().nth(1).unwrap().ends_with(": a b"), "{line}");
    let why = open.expect("held");
    assert!(why.contains("land/changes-requested: alice/web"), "{why}");
    assert!(why.contains("land/comment: bob/web on n/r"), "{why}");
    assert_eq!(cleared, None);
    assert!(other.expect("held").contains("alice/web"));
}
