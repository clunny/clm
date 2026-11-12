//! `clm_run::setup`: the command runs in the root it is given, whatever the current directory.

#[test]
fn setup_runs_in_the_root_it_is_given() {
    // req:orchestrator.run/setup-command@fdb7c407
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("clm.toml"),
        "[build]\nsetup = \"touch ready\"\n",
    )
    .unwrap();
    let out = clm_run::setup(dir.path()).unwrap();
    assert_eq!(out, "");
    assert!(dir.path().join("ready").exists());
}
