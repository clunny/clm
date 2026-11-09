//! `CLM_LAND_DIR` in the land check's environment (land/land-dir-env), and the caller's cargo
//! variables left alone (land/shared-land-target). One test in its own binary, since it sets
//! variables in the process the check inherits.

mod common;

use common::Fixture;

#[test]
fn a_land_sets_it_other_runs_remove_it_and_the_callers_cargo_variables_stay() {
    // req:land/land-dir-env@258dca71
    // SAFETY: the only test of this binary, so nothing else reads the environment meanwhile.
    unsafe { std::env::set_var("CLM_LAND_DIR", "/elsewhere") };
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let linked = f.path("linked");
    f.git(
        &f.work(),
        &["worktree", "add", "-q", linked.to_str().unwrap(), "impl/a"],
    );
    clm_land::land(&linked, None, &f.env(&[])).unwrap();
    let land_dir = f.work().join(".clm/land").display().to_string();
    assert_eq!(
        f.lines("check-land-dir.log"),
        std::slice::from_ref(&land_dir)
    );
    // `--check-only` has the caller's value removed.
    clm_land::check_only(&f.work(), None, &f.env(&[])).unwrap();
    assert_eq!(f.lines("check-land-dir.log"), [land_dir, String::new()]);

    // A caller's own target changes nothing: the check still gets `.clm/land`, and sees the
    // caller's cargo variables as the caller set them.
    // req:land/shared-land-target@2c8695d7
    let theirs = f.path("theirs");
    // SAFETY: as above.
    unsafe {
        std::env::set_var("CARGO_TARGET_DIR", &theirs);
        std::env::set_var("CARGO_INCREMENTAL", "1");
    }
    let g = Fixture::new("");
    g.branch("impl/a", &[("b.txt", "b\n")]);
    clm_land::land(&g.work(), Some("impl/a"), &g.env(&[])).unwrap();
    assert_eq!(
        g.lines("check-cargo.log"),
        [format!("{} 1", theirs.display())]
    );
    assert_eq!(
        g.lines("check-land-dir.log"),
        [g.work().join(".clm/land").display().to_string()]
    );

    // And the cap leaves the caller's directory alone.
    // req:land/land-target-cap@69c8919e
    let h = Fixture::new("");
    h.land_setting("target_cap_gb = 1");
    h.write("check-sparse-caller", "2000000000");
    h.branch("impl/a", &[("b.txt", "b\n")]);
    let out = clm_land::land(&h.work(), Some("impl/a"), &h.env(&[])).unwrap();
    assert!(out.lines.last().unwrap().starts_with("landed "), "{out:?}");
    assert!(out.warnings.iter().all(|w| !w.contains("cleared")));
    assert!(theirs.join("big").exists());
}
