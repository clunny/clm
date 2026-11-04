//! A reviewer's request for changes holds a branch (land/changes-requested-holds).

mod common;

use common::{Fixture, refused};
use serde_json::{Value, json};

fn approved(id: &str, writer: &str, at: &str, fingerprint: &str) -> Value {
    json!({"id": id, "writer": writer, "at": at, "kind": "approved",
           "fingerprint": fingerprint, "branch": "spec/x", "change": "x"})
}

fn requested(writer: &str, at: &str, fingerprint: &str, text: &str) -> Value {
    json!({"id": "r1", "writer": writer, "at": at, "kind": "changes_requested",
           "branch": "spec/x", "fingerprint": fingerprint, "text": text})
}

fn commented(writer: &str, fingerprint: &str, target: Option<&str>) -> Value {
    json!({"id": "c1", "writer": writer, "at": "2026-10-28T11:05:00.000Z", "kind": "commented",
           "branch": "spec/x", "fingerprint": fingerprint, "target": target,
           "text": "  Say which\n   rule  "})
}

const REQUESTED_AT: &str = "2026-10-28T11:00:00.000Z";

/// A `spec/x` that needs review, at fingerprint `f`, approved by `bob/web`, with `alice/web`'s request for
/// changes and `carol/web`'s comment on `n/r`, both at `f`. Returns `f`.
fn held(settings: &str) -> (Fixture, String) {
    let f = Fixture::new(settings);
    f.write("gate-out", "spec review\n");
    f.write("needs-approval", "");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let fp = f.fingerprint("spec/x");
    f.web_ledger(
        "bob/web",
        &[approved("a1", "bob/web", "2026-10-28T10:00:00.000Z", &fp)],
    );
    f.web_ledger(
        "alice/web",
        &[requested(
            "alice/web",
            REQUESTED_AT,
            &fp,
            "Name\n  the rule",
        )],
    );
    f.web_ledger("carol/web", &[commented("carol/web", &fp, Some("n/r"))]);
    (f, fp)
}

#[test]
fn held_branch_refused() {
    // req:land.approval/changes-requested-holds@27e6c16b
    let (f, _) = held("");
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])));
    let lines: Vec<&str> = why.lines().collect();
    let request =
        format!("spec/x: land/changes-requested: alice/web at {REQUESTED_AT}: Name the rule");
    let comment =
        "spec/x: land/comment: carol/web on n/r at 2026-10-28T11:05:00.000Z: Say which rule";
    let at = |line: &str| lines.iter().position(|l| *l == line);
    let (Some(r), Some(c)) = (at(&request), at(comment)) else {
        panic!("{why}")
    };
    assert!(r < c, "{why}");
    assert!(why.contains("clm approve spec/x"), "{why}");
    // Nothing ran: not the gate, not the check.
    assert!(f.lines("gate.log").is_empty(), "{why}");
    assert!(f.lines("check.log").is_empty());
    assert_eq!(f.remote("main"), old);
    assert!(f.landed().is_empty());
}

#[test]
fn comment_on_the_change_as_a_whole() {
    // req:land.approval/changes-requested-holds@27e6c16b
    let (f, fp) = held("");
    f.web_ledger("carol/web", &[commented("carol/web", &fp, None)]);
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])));
    assert!(
        why.contains("land/comment: carol/web on the change at"),
        "{why}"
    );
}

#[test]
fn cleared_by_a_push() {
    // req:land.approval/changes-requested-holds@27e6c16b
    let (f, old_fp) = held("");
    let work = f.work();
    f.git(&work, &["checkout", "-q", "spec/x"]);
    f.commit_files(&[("changes/x/change.md", "x, named\n")], "name the rule");
    f.git(&work, &["push", "-q", "origin", "spec/x"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let fp = f.fingerprint("spec/x");
    assert_ne!(fp, old_fp);
    f.web_ledger(
        "bob/web",
        &[approved("a2", "bob/web", "2026-10-28T12:00:00.000Z", &fp)],
    );
    clm_land::land(&work, Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(f.parents(&f.path("origin.git"), "main").len(), 2);
}

#[test]
fn cleared_by_the_requester() {
    // req:land.approval/changes-requested-holds@27e6c16b
    let (f, fp) = held("");
    f.web_ledger(
        "alice/1a2b3c4d",
        &[approved(
            "a2",
            "alice/1a2b3c4d",
            "2026-10-28T11:30:00.000Z",
            &fp,
        )],
    );
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(f.parents(&f.path("origin.git"), "main").len(), 2);
}

#[test]
fn an_earlier_approval_by_the_requester_doesnt_clear_it() {
    // req:land.approval/changes-requested-holds@27e6c16b
    let (f, fp) = held("");
    // Times compare as strings, as the ledger orders them: an earlier one, an equal one,
    // and one whose instant is later (11:30 UTC) but whose text sorts first don't clear it.
    for at in [
        "2026-10-28T10:30:00.000Z",
        REQUESTED_AT,
        "2026-10-28T10:30:00-01:00",
    ] {
        f.web_ledger(
            "alice/1a2b3c4d",
            &[approved("a2", "alice/1a2b3c4d", at, &fp)],
        );
        let why = refused(clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])));
        assert!(why.contains("land/changes-requested"), "{why}");
    }
}

#[test]
fn another_persons_approval_doesnt_clear_it() {
    // req:land.approval/changes-requested-holds@27e6c16b
    let (f, fp) = held("approval = \"any\"");
    f.web_ledger(
        "dana/9f9f9f9f",
        &[approved(
            "a2",
            "dana/9f9f9f9f",
            "2026-10-28T12:00:00.000Z",
            &fp,
        )],
    );
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])));
    assert!(why.contains("land/changes-requested"), "{why}");
}

#[test]
fn a_request_on_another_branch_doesnt_hold() {
    // req:land.approval/changes-requested-holds@27e6c16b
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let fp = f.fingerprint("impl/a");
    let mut request = requested("alice/web", REQUESTED_AT, &fp, "no");
    request["branch"] = "impl/b".into();
    f.web_ledger("alice/web", &[request]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
}
