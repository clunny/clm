//! The slot a `[land] check` runs with (land/check-slot).

mod common;

use common::Fixture;

#[test]
fn a_check_only_run_in_a_builds_place() {
    // req:land/check-slot@6585595e
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.place("a", 2, "impl/a");
    let linked = f.linked("wa", "impl/a");
    let out = clm_land::check_only(&linked, None, &f.env(&[])).unwrap();
    assert_eq!(out.lines.last().unwrap(), "checks pass");
    assert_eq!(f.lines("check-slot.log"), ["2"]);
}

#[test]
fn a_land() {
    // req:land/check-slot@6585595e
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.place("a", 2, "impl/a");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    assert_eq!(f.lines("check-slot.log"), ["2"]);
}

#[test]
fn outside_a_place() {
    // req:land/check-slot@6585595e
    let f = Fixture::new("");
    f.branch("x", &[("b.txt", "b\n")]);
    f.place("a", 3, "impl/a");
    let linked = f.linked("wx", "x");
    // The caller's own `CLM_SLOT` never reaches the check.
    // SAFETY: every other test in this binary sets `CLM_SLOT` for its own check, so none
    // reads the value this sets.
    unsafe { std::env::set_var("CLM_SLOT", "5") };
    let out = clm_land::check_only(&linked, None, &f.env(&[]));
    unsafe { std::env::remove_var("CLM_SLOT") };
    assert_eq!(out.unwrap().lines.last().unwrap(), "checks pass");
    assert_eq!(f.lines("check-slot.log"), ["0"]);
}

#[test]
fn a_detached_head_is_slot_0_whatever_a_place_names() {
    // req:land/check-slot@6585595e
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.place("a", 2, "impl/a");
    let linked = f.linked("wa", "impl/a");
    f.git(&linked, &["checkout", "-q", "--detach"]);
    clm_land::check_only(&linked, None, &f.env(&[])).unwrap();
    assert_eq!(f.lines("check-slot.log"), ["0"]);
}
