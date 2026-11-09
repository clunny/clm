//! `CLM_LAND_DIR` in the land check's environment (land/land-dir-env). Its own test binary,
//! since it sets a variable in the process the check inherits.

mod common;

use common::Fixture;

#[test]
fn a_land_in_its_tree_sets_it_and_other_runs_remove_it() {
    // req:land/land-dir-env@41a47ff7
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
    assert_eq!(
        f.lines("check-land-dir.log"),
        [f.work().join(".clm/land").display().to_string()]
    );
    // A run that holds no lock, here `--check-only`, has the caller's value removed.
    clm_land::check_only(&f.work(), None, &f.env(&[])).unwrap();
    assert_eq!(
        f.lines("check-land-dir.log"),
        [
            f.work().join(".clm/land").display().to_string(),
            String::new()
        ]
    );
    // A land that takes no lock on `.clm/land.lock`, in the caller's own target, has it removed.
    let g = Fixture::new("");
    g.branch("impl/a", &[("b.txt", "b\n")]);
    let env = g.env(&[("CARGO_TARGET_DIR", "/elsewhere")]);
    clm_land::land(&g.work(), Some("impl/a"), &env).unwrap();
    assert_eq!(g.lines("check-land-dir.log"), [String::new()]);
}
