//! Its own test binary, because it puts a fake `git` first on `PATH` for the whole process.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use common::*;

#[test]
fn a_push_that_lands_but_reports_failure_still_claims() {
    // req:orchestrator/claim@026fab7a
    let which = std::process::Command::new("which")
        .arg("git")
        .output()
        .unwrap();
    let real = String::from_utf8(which.stdout).unwrap();
    let real = real.trim();
    let bin = fake_bin_dir();
    let fake = bin.join("git");
    // Runs the real push, then reports failure, as a connection dropped after the update.
    fs::write(
        &fake,
        format!("#!/bin/sh\n'{real}' \"$@\"\ns=$?\n[ \"$1\" = push ] && exit 1\nexit $s\n"),
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let r = repo();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    // SAFETY: this binary has a single test, so nothing else reads the environment.
    unsafe { std::env::set_var("PATH", path) };
    assert_eq!(clm_orchestrator::claim(&r.work, "b", 4).unwrap(), "impl/b");
    let tip = remote_tip(&r, "impl/b").unwrap();
    let main = git(&r.origin, &["rev-parse", "main"]);
    assert_eq!(git(&r.origin, &["rev-parse", &format!("{tip}^")]), main);
    assert_eq!(
        git(&r.work, &["rev-parse", "refs/remotes/origin/impl/b"]).trim(),
        tip
    );
    fs::remove_dir_all(bin).unwrap();
}

fn fake_bin_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fakegit-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}
