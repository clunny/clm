//! Alone in its own test binary: it puts a fake `az` first on this process's `PATH`.

mod exec;

use clm_git::{Options, run};

#[test]
fn run_runs_any_program_on_path() {
    // req:vcs/raw-runs@9264ba25
    let bin = tempfile::tempdir().unwrap();
    exec::write_exec(&bin.path().join("az"), "#!/bin/sh\necho \"$@\"\n");
    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap()
    );
    // SAFETY: this test binary runs this one test, so no other thread reads the environment.
    unsafe { std::env::set_var("PATH", path) };
    let root = tempfile::tempdir().unwrap();
    let out = run(
        root.path(),
        "az",
        &["repos", "pr", "list"],
        Options::default(),
    )
    .unwrap();
    assert_eq!(out, b"repos pr list\n");
}
