//! A land whose `clm.toml` changed only in what `settings_unchanged` sets aside counts as
//! checked (`land/settings-unchanged-is-checked`).

mod common;

use clm_ledger::Body;
use common::Fixture;

/// `clm.toml` on `main` with `line` ahead of everything, so a top-level key stays top-level.
fn toml_with(f: &Fixture, line: &str) -> String {
    let text = std::fs::read_to_string(f.work().join("clm.toml")).unwrap();
    format!("{line}\n{text}")
}

/// The code key `landed` records for the commit `origin` has on `main`.
fn landed_key(f: &Fixture) -> String {
    let main = f.remote("main");
    f.landed()
        .into_iter()
        .find_map(|b| match b {
            Body::Landed {
                commit, code_key, ..
            } if commit == main => code_key,
            _ => None,
        })
        .expect("a landed record for main")
}

#[test]
fn the_upgrade_stamp_only() {
    // req:land/settings-unchanged-is-checked@d27ea07f
    let f = Fixture::new("");
    let stamped = toml_with(&f, "upgraded = \"20260101-x\"");
    f.branch("impl/a", &[("clm.toml", &stamped), ("docs/x.md", "# x\n")]);
    f.mark_checked("origin/main");
    let before = f.checked().len();
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(out.lines, [format!("landed {} code", f.remote("main"))]);
    assert!(f.lines("check.log").is_empty(), "the check command ran");
    let clm_check = f.lines("clm-check.log");
    assert_eq!(clm_check.len(), 1);
    assert!(clm_check[0].ends_with("--no-detectors"), "{clm_check:?}");
    // A `checked` record for the merged tree's own code key, which `landed` names.
    let key = clm_ledger::code_key(&f.path("origin.git"), &f.remote("main")).unwrap();
    assert_eq!(f.checked().len(), before + 1);
    assert!(f.checked().contains(&key));
    assert_eq!(landed_key(&f), key);
}

#[test]
fn a_comment_only() {
    // req:land/settings-unchanged-is-checked@d27ea07f
    let f = Fixture::new("");
    let commented = format!(
        "{}\n# [detectors]\n# name = \"x\"\n",
        std::fs::read_to_string(f.work().join("clm.toml")).unwrap()
    );
    f.branch("impl/a", &[("clm.toml", &commented)]);
    f.mark_checked("origin/main");
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(f.lines("check.log").is_empty(), "the check command ran");
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn a_reader_of_clm_toml() {
    // req:land/settings-unchanged-is-checked@d27ea07f
    let f = Fixture::new("");
    let stamped = toml_with(&f, "upgraded = \"20260101-x\"");
    f.branch("impl/a", &[("clm.toml", &stamped), ("docs/x.md", "# x\n")]);
    f.mark_checked("origin/main");
    f.write("readers-out", "file crates/e/tests/r.rs\n");
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.lines("check.log").len(), 1, "the check ran once");
    let copied = std::fs::read_to_string(f.path("selection-copy")).unwrap();
    assert_eq!(copied, "file crates/e/tests/r.rs\n");
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn a_setting_changed() {
    // req:land/settings-unchanged-is-checked@d27ea07f
    let f = Fixture::new("");
    // `[land] check` differs from the one the `checked` record names.
    let text = toml_with(&f, "upgraded = \"20260101-x\"");
    let changed = text.replace(
        &format!("check = \"{}\"", f.check_command()),
        &format!("check = \"{} \"", f.check_command()),
    );
    assert_ne!(changed, text);
    f.branch("impl/a", &[("clm.toml", &changed)]);
    f.mark_checked("origin/main");
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(
        f.lines("check.log").len(),
        1,
        "the check command didn't run"
    );
    let clm_check = f.lines("clm-check.log");
    assert_eq!(clm_check.len(), 1);
    assert!(!clm_check[0].contains("--no-detectors"), "{clm_check:?}");
}

#[test]
fn code_changed_too() {
    // req:land/settings-unchanged-is-checked@d27ea07f
    let f = Fixture::new("");
    let stamped = toml_with(&f, "upgraded = \"20260101-x\"");
    f.branch(
        "impl/a",
        &[("clm.toml", &stamped), ("crates/a/src/lib.rs", "// a\n")],
    );
    f.mark_checked("origin/main");
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(
        f.lines("check.log").len(),
        1,
        "the check command didn't run"
    );
}

#[test]
fn no_record_for_the_old_key() {
    // req:land/settings-unchanged-is-checked@d27ea07f
    let f = Fixture::new("");
    let stamped = toml_with(&f, "upgraded = \"20260101-x\"");
    f.branch("impl/a", &[("clm.toml", &stamped)]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(
        f.lines("check.log").len(),
        1,
        "the check command didn't run"
    );
}
