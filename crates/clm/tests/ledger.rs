//! `clm check`'s call of `ledger verify`, which runs another component's command on what the
//! ledger made, so it stays the root's (`commands-live-in-their-crates`,
//! `backlog/clm-check-s-run-of-the-ledger-audit-is-proven-only-through-the-binary`).

#[path = "../../clm-ledger/tests/common/mod.rs"]
mod common;

use std::path::Path;
use std::process::Command;

use clm_ledger::{Body, LandKind};
use common::{commit, env, repo, sh};

/// `clm <args> --root <root>` with `HOME` at `home`: exit code, stdout, stderr.
fn clm(root: &Path, home: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .arg("--root")
        .arg(root)
        .env("HOME", home)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("CLM_WRITER")
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn checked(key: &str) -> Body {
    Body::Checked {
        code_key: key.into(),
        command: "check".into(),
        basis: None,
    }
}

/// A code land of `commit`, its code key `key`.
fn landed(commit: &str, key: &str) -> Body {
    Body::Landed {
        commit: commit.into(),
        branch: "impl/x".into(),
        kind: LandKind::Code,
        review: false,
        fingerprint: "f".into(),
        code_key: Some(key.into()),
        changes: vec![],
        check_seconds: None,
        steps: vec![],
        recheck: None,
        recheck_files: None,
    }
}

/// A repository whose `main` holds a merge commit per message: the merge commits.
fn history(dir: &Path, merges: &[&str]) -> Vec<String> {
    repo(dir, "T");
    std::fs::write(dir.join("clm.toml"), "host = \"none\"\n").unwrap();
    commit(dir, &[], "files");
    let mut out = Vec::new();
    for (i, message) in merges.iter().enumerate() {
        let branch = format!("b{i}");
        sh(dir, &["switch", "-q", "-c", &branch]);
        commit(dir, &[(&format!("src/{i}.rs"), "x\n")], "work");
        sh(dir, &["switch", "-q", "main"]);
        sh(dir, &["merge", "-q", "--no-ff", "-m", message, &branch]);
        out.push(sh(dir, &["rev-parse", "HEAD"]));
    }
    out
}

/// `landed` and `checked` records for `commit`.
fn record_land(dir: &Path, commit: &str) {
    let key = clm_ledger::code_key(dir, commit).unwrap();
    let env = env(dir, &[]);
    clm_ledger::append(dir, checked(&key), &env).unwrap();
    // So the land's `at` is later.
    std::thread::sleep(std::time::Duration::from_millis(5));
    clm_ledger::append(dir, landed(commit, &key), &env).unwrap();
}

#[test]
fn check_runs_verify_on_main() {
    // req:clm/check-across-components@45a113b3
    let dir = tempfile::tempdir().unwrap();
    let merges = history(dir.path(), &["Merge b0", "Merge b1"]);
    record_land(dir.path(), &merges[0]);
    let (_, out, _) = clm(dir.path(), dir.path(), &["check", "--no-detectors"]);
    assert!(out.contains("ledger/unrecorded-land"), "{out}");
    // Off `main`, it doesn't.
    sh(dir.path(), &["switch", "-q", "-c", "other"]);
    let (_, out, _) = clm(dir.path(), dir.path(), &["check", "--no-detectors"]);
    assert!(!out.contains("ledger/"), "{out}");
}
