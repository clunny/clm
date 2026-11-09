//! `clm approve`'s fetch (`approve-retries-fetch`): a fake `git` first on this process's
//! `PATH` fails `fetch` as the clone's `.git/fake-mode` says. One test runs both scenarios,
//! since `PATH` is the process's.

mod common;

use common::Fixture;
use std::fs;
use std::os::unix::fs::PermissionsExt;

const FAKE: &str = r#"#!/bin/sh
real=$(PATH="$FAKE_REAL_PATH" command -v git)
dir=$("$real" rev-parse --absolute-git-dir 2>/dev/null)
if [ "$1" = fetch ] && [ -n "$dir" ] && [ -f "$dir/fake-mode" ]; then
  mode=$(cat "$dir/fake-mode")
  echo fetch >> "$dir/fake-calls"
  n=$(grep -c '^fetch$' "$dir/fake-calls")
  if [ "$mode" = always ] || { [ "$mode" = once ] && [ "$n" = 1 ]; }; then
    echo "error: cannot lock ref 'refs/remotes/origin/main'" >&2
    exit 1
  fi
fi
exec "$real" "$@"
"#;

#[test]
fn approve_fetches_through_fetch_retrying() {
    // req:land.approval/approve-retries-fetch@59cf62a7
    let bin = tempfile::tempdir().unwrap();
    let fake = bin.path().join("git");
    fs::write(&fake, FAKE).unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let real = std::env::var("PATH").unwrap();
    // SAFETY: this file's only test, so no other thread reads the environment.
    unsafe {
        std::env::set_var("FAKE_REAL_PATH", &real);
        std::env::set_var("PATH", format!("{}:{real}", bin.path().display()));
    }

    // A fetch that loses a ref race once goes on.
    let f = Fixture::new("");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let fp = f.fingerprint("spec/x");
    fs::write(f.work().join(".git/fake-mode"), "once").unwrap();
    let out = clm_approval::approve(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(out.lines, [format!("approved {fp} spec/x")]);
    fs::remove_file(f.work().join(".git/fake-mode")).unwrap();
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    match &f.records()[..] {
        [r] => assert!(
            matches!(&r.body, clm_ledger::Body::Approved { fingerprint, .. } if *fingerprint == fp)
        ),
        other => panic!("{other:?}"),
    }

    // A fetch that keeps failing with origin reachable is git's message, not offline.
    let g = Fixture::new("");
    g.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    fs::write(g.work().join(".git/fake-mode"), "always").unwrap();
    let why = common::refused(clm_approval::approve(
        &g.work(),
        Some("spec/x"),
        &g.env(&[]),
    ));
    assert!(why.contains("cannot lock ref"), "{why}");
    assert!(!why.contains("offline"), "{why}");
    fs::remove_file(g.work().join(".git/fake-mode")).unwrap();
    assert!(g.records().is_empty());

    // An unreachable origin is offline, and nothing is appended.
    let h = Fixture::new("");
    h.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let gone = h.path("missing");
    h.git(
        &h.work(),
        &["remote", "set-url", "origin", gone.to_str().unwrap()],
    );
    let why = common::refused(clm_approval::approve(
        &h.work(),
        Some("spec/x"),
        &h.env(&[]),
    ));
    assert!(why.contains("offline"), "{why}");
    h.git(
        &h.work(),
        &[
            "remote",
            "set-url",
            "origin",
            h.path("origin.git").to_str().unwrap(),
        ],
    );
    assert!(h.records().is_empty());
}
