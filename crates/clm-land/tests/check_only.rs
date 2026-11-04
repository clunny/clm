//! `clm land --check-only` (land/check-only): the land's checks on the working tree, landing
//! and recording nothing.

mod common;

use common::{Fixture, refused};

#[test]
fn check_command_fails() {
    // req:land/check-only@57b7e93c
    let f = Fixture::new("");
    f.write("check-fails", "");
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert!(why.contains(&f.check_command()), "{why}");
    assert!(why.contains("fails"), "{why}");
    assert_eq!(f.lines("check.log").len(), 1);
    assert!(f.lines("clm-check.log").is_empty(), "`clm check` ran");
}

#[test]
fn clm_check_fails() {
    // req:land/check-only@57b7e93c
    let f = Fixture::new("");
    f.write("clm-check-fails", "");
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert!(why.contains("`clm check` fails"), "{why}");
}

#[test]
fn no_check_command() {
    // req:land/check-only@57b7e93c
    let f = Fixture::new("");
    std::fs::write(f.work().join("clm.toml"), "").unwrap();
    let out = clm_land::check_only(&f.work(), None, &f.env(&[])).unwrap();
    assert_eq!(out.lines, ["checks pass"]);
    assert!(f.lines("check.log").is_empty(), "the check command ran");
    assert_eq!(f.lines("clm-check.log").len(), 1);
}

#[test]
fn with_a_branch() {
    // req:land/check-only@57b7e93c
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let why = refused(clm_land::check_only(&f.work(), Some("impl/a"), &f.env(&[])));
    assert!(why.contains("--check-only"), "{why}");
    assert!(f.lines("check.log").is_empty(), "the check command ran");
    assert!(f.lines("clm-check.log").is_empty(), "`clm check` ran");
}

#[test]
fn unreadable_settings() {
    // req:land/check-only@57b7e93c
    let f = Fixture::new("");
    std::fs::write(f.work().join("clm.toml"), "not = [toml").unwrap();
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert!(why.contains("clm.toml"), "{why}");
    assert!(f.lines("check.log").is_empty(), "the check command ran");
    assert!(f.lines("clm-check.log").is_empty(), "`clm check` ran");
}
