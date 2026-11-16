//! `clm setup` as `clm-run`'s call (`commands-in-crate`): the repository's `[build] setup`,
//! run in its root, whatever the current directory.

mod support;

use std::fs;
use std::path::Path;

use support::setup_repo as repo;
use support::{Output, clm_setup_with, text};

/// `clm setup` run in `cwd`, with an `XDG_CONFIG_HOME` that keeps the user's settings out.
fn clm_setup(cwd: &Path) -> Output {
    clm_setup_with(cwd, &[("XDG_CONFIG_HOME", cwd.as_os_str())])
}

#[test]
fn setup_runs_in_the_root_it_is_given() {
    // req:orchestrator.run/setup-command@fdb7c407
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("clm.toml"),
        "[build]\nsetup = \"touch ready\"\n",
    )
    .unwrap();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let printed = clm_run::setup(dir.path(), &|_| None, &mut out, &mut err).unwrap();
    assert_eq!(printed, "");
    assert!(dir.path().join("ready").exists());
}

#[test]
fn scenario_setup_runs_in_the_root() {
    // req:orchestrator.run/setup-command@fdb7c407
    let r = repo("[build]\nsetup = \"touch ready\"\n");
    let out = clm_setup(&r.path().join("src"));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(r.path().join("ready").exists());
    assert!(!r.path().join("src/ready").exists());
}

#[test]
fn scenario_setup_fails() {
    // req:orchestrator.run/setup-command@fdb7c407
    let r = repo("[build]\nsetup = \"exit 3\"\n");
    let out = clm_setup(r.path());
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(err.contains("setup failed"), "{err}");
    assert!(err.contains("`exit 3`"), "{err}");
    assert!(err.contains("status 3"), "{err}");
}

#[test]
fn a_setup_killed_by_a_signal_says_so() {
    // req:orchestrator.run/setup-command@fdb7c407
    let r = repo("[build]\nsetup = \"kill -9 $$\"\n");
    let out = clm_setup(r.path());
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(err.contains("killed by signal 9"), "{err}");
}

#[test]
fn setup_passes_output_through_and_closes_stdin() {
    // req:orchestrator.run/setup-command@fdb7c407
    let r = repo(
        "[build]\nsetup = \"echo installed; echo warned >&2; timeout 10 cat > got; echo $? > ended\"\n",
    );
    // The test process's standard input, which the call never closes, may stay open, so
    // only the command's own closed one ends `cat` at once.
    let out = clm_setup(r.path());
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("installed"));
    assert!(text(&out.stderr).contains("warned"));
    assert_eq!(fs::read_to_string(r.path().join("got")).unwrap(), "");
    assert_eq!(fs::read_to_string(r.path().join("ended")).unwrap(), "0\n");
}

#[test]
fn scenario_nothing_declared() {
    // req:orchestrator.run/setup-command@fdb7c407
    let r = repo("[land]\ncheck = \"true\"\n");
    let out = clm_setup(r.path());
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(text(&out.stdout).trim(), "no [build] setup");
}

#[test]
fn scenario_unreadable_settings() {
    // req:orchestrator.run/setup-command@fdb7c407
    let r = repo("[build\nsetup = \n");
    let out = clm_setup(r.path());
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("clm.toml"),
        "{}",
        text(&out.stderr)
    );
    assert!(text(&out.stdout).is_empty());
}
