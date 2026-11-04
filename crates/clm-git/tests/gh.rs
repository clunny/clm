//! Alone in its own test binary: it puts a fake `gh` first on this process's `PATH`.

mod exec;

#[test]
fn gh_runs_the_first_gh_on_path() {
    // req:vcs/run-git@32c20165
    let bin = tempfile::tempdir().unwrap();
    let fake = bin.path().join("gh");
    exec::write_exec(&fake, "#!/bin/sh\necho '{\"state\":\"MERGED\"}'\n");
    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap()
    );
    // SAFETY: this test binary runs this one test, so no other thread reads the environment.
    unsafe { std::env::set_var("PATH", path) };
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        clm_git::gh(root.path(), &["pr", "view", "1"]).unwrap(),
        r#"{"state":"MERGED"}"#
    );
}
