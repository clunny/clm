//! The refusal a request for changes makes (land.approval/request-refusal).

mod common;

use clm_ledger::Body;
use common::{fold, line};

const AT: &str = "2026-11-17T10:00:00.000Z";
const LATER: &str = "2026-11-17T11:00:00.000Z";

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

fn approval(writer: &str) -> String {
    let body = Body::Approved {
        fingerprint: "f".into(),
        branch: "spec/x".into(),
        change: None,
    };
    line("a1", writer, LATER, body, None)
}

/// The fold of `held`: a request and a comment on `n/r`, then `more`.
fn held_fold(more: &[String]) -> clm_ledger::Fold {
    let mut lines = vec![request("Name the rule"), comment("c1", Some("n/r"))];
    lines.extend_from_slice(more);
    fold(&lines)
}

#[test]
fn held() {
    // req:land.approval/request-refusal@62c49c32
    let fold = held_fold(&[comment("c2", None)]);
    let why = clm_approval::held(&fold, "spec/x", "f").expect("held");
    let lines: Vec<&str> = why.lines().collect();
    assert!(lines.len() >= 5, "{why}");
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
    // req:land.approval/request-refusal@62c49c32
    let fold = held_fold(&[]);
    assert_eq!(clm_approval::held(&fold, "spec/x", "g"), None);
    // Nor on another branch.
    assert_eq!(clm_approval::held(&fold, "spec/y", "f"), None);
}

#[test]
fn cleared_by_the_requester() {
    // req:land.approval/request-refusal@62c49c32
    let fold = held_fold(&[approval("alice/1a2b3c4d")]);
    assert_eq!(clm_approval::held(&fold, "spec/x", "f"), None);
}

#[test]
fn another_persons_approval_doesnt_clear_it() {
    // req:land.approval/request-refusal@62c49c32
    let fold = held_fold(&[approval("dana/9f9f9f9f")]);
    let why = clm_approval::held(&fold, "spec/x", "f").expect("held");
    assert!(
        why.contains("spec/x: land/changes-requested: alice/web at"),
        "{why}"
    );
}

#[test]
fn a_text_over_several_lines() {
    // req:land.approval/request-refusal@62c49c32
    let fold = fold(&[request("a\n  b")]);
    let why = clm_approval::held(&fold, "spec/x", "f").expect("held");
    let line = why
        .lines()
        .find(|l| l.contains("land/changes-requested"))
        .unwrap();
    assert!(line.ends_with(": a b"), "{line}");
}
