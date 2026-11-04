//! The report ends with `wrap-up ok` or `wrap-up incomplete`.

use std::path::{Path, PathBuf};
use std::process::Command;

use clm_checkpoint::checkpoint;
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "{args:?}: {out:?}");
}

/// A bare `origin` with `main` pushed, and a clone of it.
fn clone() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let origin = dir.path().join("origin");
    let work = dir.path().join("work");
    std::fs::create_dir(&origin).unwrap();
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    git(
        dir.path(),
        &[
            "clone",
            "-q",
            origin.to_str().unwrap(),
            work.to_str().unwrap(),
        ],
    );
    std::fs::write(work.join("ARCHITECTURE.md"), "# Arch\n").unwrap();
    std::fs::create_dir(work.join("specs")).unwrap();
    std::fs::write(
        work.join("specs/SPEC.md"),
        "---\ntitle: T\nstatus: active\nowners: []\ndepends_on: []\n---\n\n\
         ## Contract\nc\n\n## Design\nd\n\n## Requirements\n",
    )
    .unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-qm", "base"]);
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    (dir, work)
}

#[test]
fn scenario_run_as_wrap_up() {
    // req:orchestrator.checkpoint/outcome-name@cb6590f1
    let (_dir, work) = clone();
    let report = checkpoint(&work).unwrap();
    assert!(report.ok, "{report:?}");
    assert_eq!(report.lines.last().map(String::as_str), Some("wrap-up ok"));
}

#[test]
fn scenario_incomplete() {
    // req:orchestrator.checkpoint/outcome-name@cb6590f1
    let (_dir, work) = clone();
    git(&work, &["checkout", "-qb", "spec/z"]);
    git(&work, &["push", "-q", "origin", "spec/z"]);
    std::fs::write(work.join("z.txt"), "z\n").unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-qm", "z"]);
    let report = checkpoint(&work).unwrap();
    assert!(!report.ok, "{report:?}");
    assert!(report.lines.iter().any(|l| l == "unpushed spec/z"));
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("wrap-up incomplete")
    );
}
