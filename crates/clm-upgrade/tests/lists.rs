//! The crate's public forms that take a list, called from outside its library, and the closed
//! list of its own migrations.

use std::fs;
use std::path::Path;

use clm_upgrade::{
    apply_in, check_lock, decline_in, latest_in, list_in, migrations, note_in, pending_in,
    require_current_in, supersedes,
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
fn every_form_that_takes_a_list_answers_over_the_list_it_is_given() {
    let (a, b) = (stamped_project(), stamped_project());
    let (own, pairs) = (migrations(), supersedes());
    assert_eq!(latest_in(own), Some(own.last().unwrap().id));
    assert_eq!(latest_in(&[]), None);
    let ids = |found: Vec<&clm_upgrade::Migration>| found.iter().map(|m| m.id).collect::<Vec<_>>();
    let from_list = ids(pending_in(own, pairs, a.path()).unwrap());
    assert!(
        !from_list.is_empty(),
        "the fixture leaves something pending"
    );
    // A list of the caller's own is the one each form reads, not the crate's.
    let one = &own[..1];
    assert_eq!(latest_in(one), Some(own[0].id));
    fs::write(b.path().join("clm.toml"), "").unwrap();
    let few = ids(pending_in(one, pairs, b.path()).unwrap());
    assert!(
        few.len() <= 1 && few.iter().all(|id| *id == own[0].id),
        "{few:?}"
    );
    assert!(pending_in(&[], pairs, b.path()).unwrap().is_empty());
    assert_eq!(require_current_in(own, pairs, a.path()), Ok(()));
    let listed = list_in(own, pairs, a.path()).unwrap();
    assert_eq!(listed.lines().count(), from_list.len(), "{listed}");
    assert!(listed.starts_with(from_list[0]), "{listed}");
    let first = from_list[0];
    assert!(note_in(own, pairs, a.path(), first).is_ok(), "{first}");
    assert!(decline_in(own, pairs, a.path(), "nope").is_err());
    assert!(apply_in(own, pairs, a.path()).is_ok());
    assert!(
        fs::read_to_string(a.path().join("clm.toml"))
            .unwrap()
            .contains(&format!("upgraded = \"{}\"", latest_in(own).unwrap()))
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

// req:upgrade/own-list-closed@6b42898f
#[test]
fn the_list_is_exactly_the_thirteen_in_order() {
    let ids: Vec<&str> = migrations().iter().map(|m| m.id).collect();
    assert_eq!(ids, OWN);
}
