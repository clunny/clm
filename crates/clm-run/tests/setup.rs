//! `clm setup` as `clm-run`'s call (`commands-in-crate`): the repository's `[build] setup`,
//! run in its root, whatever the current directory.

mod support;

use std::fs;
use std::path::Path;

use support::setup_repo as repo;
use support::{Output, clm_setup_with, text};

/// Names the repository for `setup_passes_output_through_and_closes_stdin` run again as a
/// process of its own.
const HELD_STDIN_ROOT: &str = "CLM_RUN_TEST_HELD_STDIN_ROOT";

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
    // The call, made by this test run again as a process whose standard input the outer run
    // holds open, writes what it got to the repository the outer run made.
    if let Some(root) = std::env::var_os(HELD_STDIN_ROOT) {
        let out = clm_setup(Path::new(&root));
        let root = Path::new(&root);
        let status = out.status.code().unwrap().to_string();
        fs::write(root.join("call-status"), status).unwrap();
        fs::write(root.join("call-stdout"), &out.stdout).unwrap();
        fs::write(root.join("call-stderr"), &out.stderr).unwrap();
        return;
    }
    let r = repo(
        "[build]\nsetup = \"echo installed; echo warned >&2; timeout 10 cat > got; echo $? > ended\"\n",
    );
    // The caller's standard input stays open, so only a closed one ends `cat` at once.
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "setup_passes_output_through_and_closes_stdin",
            "--test-threads=1",
        ])
        .env(HELD_STDIN_ROOT, r.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let held = child.stdin.take();
    let run = child.wait_with_output().unwrap();
    drop(held);
    assert!(run.status.success(), "{}", text(&run.stdout));
    let read = |name: &str| fs::read(r.path().join(name)).unwrap();
    let out = support::Output {
        status: support::Status::from(text(&read("call-status")).parse::<u8>().unwrap()),
        stdout: read("call-stdout"),
        stderr: read("call-stderr"),
    };
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
fn a_process_left_in_the_background_holds_up_nothing() {
    // req:orchestrator.run/setup-command@fdb7c407
    // The background `sleep` keeps the command's standard output and error open long after
    // `sh` has ended.
    let r = repo("[build]\nsetup = \"sleep 30 & echo started\"\n");
    let began = std::time::Instant::now();
    let out = clm_setup(r.path());
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(
        text(&out.stdout).contains("started"),
        "{}",
        text(&out.stdout)
    );
    let took = began.elapsed();
    assert!(took < std::time::Duration::from_secs(10), "{took:?}");
}
