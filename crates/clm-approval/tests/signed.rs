//! Signed repositories: `clm approve` signs its record (signed-approve).

mod common;

use std::path::{Path, PathBuf};

use clm_ledger::{Body, Record};
use common::{Fixture, keygen, refused};

/// `repo` signs with `key` from now on.
fn use_key(f: &Fixture, repo: &Path, key: &Path) {
    f.git(repo, &["config", "gpg.format", "ssh"]);
    f.git(repo, &["config", "user.signingkey", key.to_str().unwrap()]);
}

/// A fixture whose `origin/main` holds `docs/allowed_signers` listing `christopher-lunny`, unless
/// `signers` is false; the work clone is at that `main`. Returns it with `christopher`'s key.
fn signed_repo(signers: bool) -> (Fixture, PathBuf) {
    let f = Fixture::new("");
    let keys = f.path("keys");
    let (christopher, christopher_pub) = keygen(&keys, "christopher");
    if signers {
        f.advance_main(
            "docs/allowed_signers",
            &format!("christopher-lunny {christopher_pub}\n"),
        );
        let work = f.work();
        f.git(&work, &["fetch", "-q", "origin"]);
        f.git(&work, &["merge", "-q", "--ff-only", "origin/main"]);
    }
    (f, christopher)
}

/// The line `fold` read for `record`.
fn line_of(f: &Fixture, record: &Record) -> serde_json::Map<String, serde_json::Value> {
    f.records();
    clm_ledger::fold(&f.work()).unwrap().lines[&record.id].clone()
}

fn approvals(f: &Fixture) -> Vec<Record> {
    f.records()
        .into_iter()
        .filter(|r| matches!(r.body, Body::Approved { .. }))
        .collect()
}

#[test]
fn signed_approval() {
    // req:land.approval/signed-approve@a96763ed
    let (f, christopher) = signed_repo(true);
    let f = &f;
    use_key(f, &f.work(), &christopher);
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let out = clm_approval::approve(&f.work(), Some("spec/x"), &f.isolated_env()).unwrap();
    let fingerprint = f.fingerprint("spec/x");
    assert_eq!(out.lines, [format!("approved {fingerprint} spec/x")]);
    let [record] = &approvals(f)[..] else {
        panic!("not one approval")
    };
    assert!(matches!(&record.body, Body::Approved { fingerprint: fp, .. } if *fp == fingerprint));
    let line = line_of(f, record);
    assert!(line.contains_key("sig"), "{line:?}");
    let work = f.work();
    assert!(clm_ledger::signed_by(&work, "origin/main", &line, "clm-approve").unwrap());
    // Under its own namespace only.
    assert!(!clm_ledger::signed_by(&work, "origin/main", &line, "clm-land").unwrap());
}

#[test]
fn approve_with_no_key() {
    // req:land.approval/signed-approve@a96763ed
    let (f, _) = signed_repo(true);
    let f = &f;
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let why = refused(clm_approval::approve(
        &f.work(),
        Some("spec/x"),
        &f.isolated_env(),
    ));
    assert!(why.contains("user.signingkey"), "{why}");
    assert!(f.records().is_empty(), "{:?}", f.records());
}

#[test]
fn unsigned_repository() {
    // req:land.approval/signed-approve@a96763ed
    let (f, _) = signed_repo(false);
    let f = &f;
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    clm_approval::approve(&f.work(), Some("spec/x"), &f.isolated_env()).unwrap();
    let [record] = &approvals(f)[..] else {
        panic!("not one approval")
    };
    assert!(!line_of(f, record).contains_key("sig"));
}
