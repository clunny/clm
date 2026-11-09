//! `append` in a clone whose git has no `user.email` and may not guess one, as on a CI
//! runner. Its own test binary, since it masks the user's and the system's git config for
//! the whole process.

mod common;

use clm_ledger::Body;
use common::{clone_with_origin, env, sh};

#[test]
fn a_clone_with_no_email_still_appends() {
    // req:ledger/append@d894aff0
    // SAFETY: the only test in this binary, so no other thread reads the environment.
    unsafe {
        std::env::set_var("GIT_CONFIG_GLOBAL", "/dev/null");
        std::env::set_var("GIT_CONFIG_NOSYSTEM", "1");
    }
    let dir = tempfile::tempdir().unwrap();
    let clone = clone_with_origin(dir.path(), "a", "Ann");
    sh(&clone, &["config", "--unset", "user.email"]);
    sh(&clone, &["config", "user.useConfigOnly", "true"]);
    let body = Body::Approved {
        fingerprint: "f".into(),
        branch: "spec/x".into(),
        change: Some("x".into()),
    };
    let out = clm_ledger::append(&clone, body, &env(dir.path(), &[])).unwrap();
    assert!(out.pushed);
    let branch = format!("clm/ledger/{}", out.record.writer);
    let origin = dir.path().join("origin.git");
    let records = sh(&origin, &["show", &format!("{branch}:records.jsonl")]);
    assert_eq!(records.lines().count(), 1);
    assert_eq!(
        sh(
            &origin,
            &["log", "-1", "--format=%an <%ae>|%cn <%ce>", &branch]
        ),
        "ann <>|ann <>"
    );
}
