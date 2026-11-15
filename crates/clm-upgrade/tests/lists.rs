//! The crate's public forms that take a list, called from outside its library, and the closed
//! list of its own migrations.

use std::fs;
use std::path::Path;

use clm_upgrade::{
    TooNew, apply, apply_in, check_lock, decline, decline_in, latest, latest_in, list, list_in,
    migrations, note, note_in, pending, pending_in, require_current, require_current_in,
    supersedes,
};
use tempfile::TempDir;

/// A project stamped with the last ID of the crate's own list.
fn stamped_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let last = latest_in(migrations()).unwrap();
    fs::write(
        dir.path().join("clm.toml"),
        format!("upgraded = \"{last}\"\n\n[land]\ncheck = \"true\"\n"),
    )
    .unwrap();
    dir
}

// req:upgrade/migration-list@b7ed6123
#[test]
fn every_form_that_takes_a_list_answers_as_its_form_without_one() {
    let (a, b) = (stamped_project(), stamped_project());
    let (own, pairs) = (migrations(), supersedes());
    assert_eq!(latest_in(own), latest());
    assert_eq!(latest_in(&[]), None);
    let ids = |found: Vec<&clm_upgrade::Migration>| found.iter().map(|m| m.id).collect::<Vec<_>>();
    let from_list = ids(pending_in(own, pairs, a.path()).unwrap());
    assert_eq!(from_list, ids(pending(a.path()).unwrap()));
    assert!(
        !from_list.is_empty(),
        "the fixture leaves something pending"
    );
    assert_eq!(
        require_current_in(own, pairs, a.path()),
        require_current(a.path())
    );
    assert_eq!(list_in(own, pairs, a.path()), list(a.path()));
    let first = from_list[0];
    assert_eq!(
        note_in(own, pairs, a.path(), first),
        note(a.path(), first),
        "{first}"
    );
    assert_eq!(
        decline_in(own, pairs, a.path(), "nope"),
        decline(a.path(), "nope")
    );
    assert_eq!(apply_in(own, pairs, a.path()), apply(b.path()));
    assert_eq!(
        fs::read_to_string(a.path().join("clm.toml")).unwrap(),
        fs::read_to_string(b.path().join("clm.toml")).unwrap()
    );
}

// req:upgrade/migration-list@b7ed6123
#[test]
fn a_migration_is_a_value_a_caller_copies_into_a_list_of_its_own() {
    let mut own = migrations().to_vec();
    let copy = own[0];
    own.push(copy);
    assert_eq!(own.len(), migrations().len() + 1);
    assert_eq!(latest_in(&own), Some(migrations()[0].id));
}

// req:upgrade/pending-migrations@de5ff412
#[test]
fn a_stamp_the_given_list_lacks_is_too_new_whatever_the_crate_holds() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("clm.toml"), "upgraded = \"zzz\"\n").unwrap();
    let err = pending_in(&migrations()[..1], supersedes(), dir.path()).unwrap_err();
    assert_eq!(err, TooNew("zzz".to_string()));
}

// req:upgrade/shipped-migrations-frozen@46f86d46
#[test]
fn the_lock_check_reads_any_crates_directory() {
    let ids: Vec<&str> = migrations().iter().map(|m| m.id).collect();
    check_lock(Path::new(env!("CARGO_MANIFEST_DIR")), &ids).unwrap();
    let err = check_lock(Path::new(env!("CARGO_MANIFEST_DIR")), &[]).unwrap_err();
    assert!(err.contains("is gone"), "{err}");
}

const OWN: [&str; 13] = [
    "20261105-clm-upgrade",
    "20261105-clm-upgrade-2",
    "20261106-corpus-entries-drift-from",
    "20261107-projects-set-up-before",
    "20261107-slow-tests-after-land",
    "20261107-projects-upgrading-past-the",
    "20261108-toolchain-neutral-land-builds",
    "20261109-land-still-sets-cargo",
    "20261109-a-build-s-tests",
    "20261112-agents-md",
    "20261112-a-draft-s-clm",
    "20261112-build-loop-in-clm",
    "20261114-proof-is-real",
];

/// A directory under `dir` that `OWN` doesn't name, or one it names that is missing.
fn closed_violation(dir: &Path) -> Option<String> {
    let found: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().unwrap().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    if let Some(extra) = found.iter().find(|d| !OWN.contains(&d.as_str())) {
        return Some(format!("directory `{extra}` is not one of the closed list"));
    }
    OWN.iter()
        .find(|id| !found.iter().any(|d| d == *id))
        .map(|id| format!("directory `{id}` is missing"))
}

// req:upgrade/own-list-closed@6b42898f
#[test]
fn the_list_is_exactly_the_thirteen_in_order() {
    let ids: Vec<&str> = migrations().iter().map(|m| m.id).collect();
    assert_eq!(ids, OWN);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/migrations");
    assert_eq!(closed_violation(&dir), None);
}

// req:upgrade/own-list-closed@6b42898f
#[test]
fn a_directory_added_fails_the_check_naming_it() {
    let tree = TempDir::new().unwrap();
    for id in OWN.iter().chain(&["20261120-one-more"]) {
        fs::create_dir(tree.path().join(id)).unwrap();
    }
    let err = closed_violation(tree.path()).unwrap();
    assert!(err.contains("20261120-one-more"), "{err}");
}
