//! The path of the log a failed run kept (verify/kept-log).

mod common;

use common::Fixture;

fn every_land() -> Fixture {
    Fixture::new(Some(
        "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }",
    ))
}

/// A linked worktree of the fixture's clone, and a land one past a passing record.
fn linked(f: &Fixture) -> (std::path::PathBuf, String) {
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    let land = f.land(&[("crates/b/src/lib.rs", "x\n")], "two");
    let linked = f.path("linked");
    f.git(
        &f.work(),
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            linked.to_str().unwrap(),
        ],
    );
    (linked, land)
}

#[test]
fn a_red_runs_log() {
    // req:verify/kept-log@05b0ad5c
    let f = every_land();
    let (linked, land) = linked(&f);
    f.write("check-fails", "");
    let out = f.due();
    assert_eq!(out.code, 1);
    let named = out
        .notes
        .iter()
        .find_map(|n| n.strip_prefix("output is at "))
        .expect("the log's path");
    let found = clm_verify::kept_log(&linked, &land).expect("a kept log");
    assert!(found.is_absolute() && found.exists(), "{found:?}");
    assert_eq!(found, std::path::PathBuf::from(named));
}

#[test]
fn nothing_kept() {
    // req:verify/kept-log@05b0ad5c
    let f = every_land();
    let (linked, land) = linked(&f);
    assert_eq!(f.due().code, 0);
    assert_eq!(clm_verify::kept_log(&linked, &land), None);
    assert!(!f.work().join(".clm/verify-logs").join(&land).exists());
}

#[test]
fn a_longer_name() {
    // req:verify/kept-log@05b0ad5c
    let f = every_land();
    let (linked, land) = linked(&f);
    let dir = f.work().join(".clm/verify-logs");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{}.log", &land[..7])), "short").unwrap();
    std::fs::write(dir.join(format!("{}.log", &land[..9])), "long").unwrap();
    // A name that is no prefix of the commit, or is too short, is not matched.
    std::fs::write(dir.join(format!("{}.log", &land[..6])), "tiny").unwrap();
    std::fs::write(dir.join("0000000000.log"), "other").unwrap();
    let found = clm_verify::kept_log(&linked, &land).unwrap();
    assert_eq!(std::fs::read_to_string(found).unwrap(), "long");
}

#[test]
fn a_missing_directory() {
    // req:verify/kept-log@05b0ad5c
    let f = every_land();
    let (linked, land) = linked(&f);
    assert_eq!(clm_verify::kept_log(&linked, &land), None);
}
