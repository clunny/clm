//! Its own test binary, because it puts a fake `git` first on `PATH` for the whole process.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use common::*;

/// A `git` that, for each `fetch` naming `origin`, fails while the file `fails` holds a
/// positive count, decrementing it, and runs the real `git` otherwise.
fn install_fake(bin: &std::path::Path, fails: &std::path::Path) {
    let which = std::process::Command::new("which")
        .arg("git")
        .output()
        .unwrap();
    let real = String::from_utf8(which.stdout).unwrap();
    let real = real.trim();
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = fetch ]; then\n\
         for a in \"$@\"; do if [ \"$a\" = origin ]; then\n\
         n=$(cat '{f}')\n\
         if [ \"$n\" -gt 0 ]; then echo $((n-1)) > '{f}'; \
         echo \"error: cannot lock ref 'refs/remotes/origin/impl/a'\" >&2; exit 1; fi\n\
         fi; done\n\
         fi\n\
         exec '{real}' \"$@\"\n",
        f = fails.display()
    );
    fs::write(bin.join("git"), script).unwrap();
    fs::set_permissions(bin.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn a_fetch_that_loses_a_ref_race_goes_on_and_one_that_keeps_failing_fails() {
    // req:orchestrator/fetch-retries-race@ba70b96b
    let dir = tempfile::TempDir::new().unwrap();
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let fails = dir.path().join("fails");
    install_fake(&bin, &fails);
    let r = repo();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    // SAFETY: this binary has a single test, so nothing else reads the environment.
    unsafe { std::env::set_var("PATH", path) };

    // Keeps failing: every run fails, `origin` answers, so git's message comes back.
    fs::write(&fails, "100").unwrap();
    match clm_orchestrator::claim(&r.work, "b", 4) {
        Err(clm_orchestrator::Error::Git(m)) => assert!(m.contains("cannot lock ref"), "{m}"),
        other => panic!("{other:?}"),
    }
    assert!(remote_tip(&r, "impl/b").is_none());

    // Loses the race once: the retry goes on.
    fs::write(&fails, "1").unwrap();
    assert_eq!(clm_orchestrator::claim(&r.work, "b", 4).unwrap(), "impl/b");
    assert!(remote_tip(&r, "impl/b").is_some());
}
