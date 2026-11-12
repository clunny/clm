//! `clm setup`: the repository's `[build] setup`, run in its root.

use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

/// A CLM repository (a `specs/SPEC.md` and a `.git`) with `clm.toml` holding `toml`, and a
/// subdirectory `src/`.
fn repo(toml: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("specs")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("specs/SPEC.md"), "# spec\n").unwrap();
    fs::write(root.join("clm.toml"), toml).unwrap();
    dir
}

fn clm_setup(cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("setup")
        .current_dir(cwd)
        .env("XDG_CONFIG_HOME", cwd)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
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
    // The caller's standard input stays open, so only a closed one ends `cat` at once.
    let mut child = Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("setup")
        .current_dir(r.path())
        .env("XDG_CONFIG_HOME", r.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let held = child.stdin.take();
    let out = child.wait_with_output().unwrap();
    drop(held);
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

#[test]
fn scenario_setup_gets_the_projects_variables() {
    // req:orchestrator.run/setup-command@fdb7c407
    let r = repo(
        "[build]\nsetup = \"echo $RELAY_SERVER_IMAGE > seen\"\n[env]\nRELAY_SERVER_IMAGE = \"relay:amd64\"\n",
    );
    fs::write(
        r.path().join("clm.local.toml"),
        "[env]\nRELAY_SERVER_IMAGE = \"relay:arm64\"\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("setup")
        .current_dir(r.path())
        .env("XDG_CONFIG_HOME", r.path())
        .env("RELAY_SERVER_IMAGE", "relay:shell")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(
        fs::read_to_string(r.path().join("seen")).unwrap().trim(),
        "relay:arm64"
    );
}
