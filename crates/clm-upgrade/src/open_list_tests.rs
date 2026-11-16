//! Tests of `pending_in` over a list that holds a migration the crate doesn't.

use std::fs;
use std::path::Path;

use crate::shipped_tests::{NEW_ARCHITECTURE, NEW_CLAUDE, project};
use crate::{Kind, Level, Migration, migrations, pending_in, supersedes};

const PROOF_ID: &str = "20261114-proof-is-real";

fn done_when_z_exists(root: &Path) -> bool {
    root.join("z").exists()
}

fn apply_z(root: &Path) -> Result<(), String> {
    fs::write(root.join("z"), "").map_err(|e| e.to_string())
}

/// The crate's own list followed by one apply migration `z`, as `init` would offer it.
fn own_list_and_z() -> Vec<Migration> {
    let mut list = migrations().to_vec();
    list.push(Migration {
        id: "z",
        level: Level::Advised,
        done: done_when_z_exists,
        kind: Kind::Apply(apply_z, ""),
    });
    list
}

fn ids_of(pending: &[&Migration]) -> Vec<&'static str> {
    pending.iter().map(|m| m.id).collect()
}

// req:upgrade/pending-migrations@de5ff412
#[test]
fn a_migration_the_crate_does_not_hold_is_judged_like_any_other() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    let list = own_list_and_z();
    let pending = pending_in(&list, supersedes(), dir.path()).unwrap();
    assert_eq!(ids_of(&pending), ["z"]);
    let own = pending_in(migrations(), supersedes(), dir.path()).unwrap();
    assert!(own.is_empty(), "{:?}", ids_of(&own));
}

// req:upgrade/pending-migrations@de5ff412
// req:upgrade/migration-markers-in-tests@25966374
#[test]
fn a_stamp_naming_a_migration_the_crate_does_not_hold_leaves_the_four_notes_out() {
    let dir = project(NEW_ARCHITECTURE, NEW_CLAUDE);
    fs::write(dir.path().join("z"), "").unwrap();
    fs::write(
        dir.path().join("clm.toml"),
        "upgraded = \"z\"\ndefects = \"ask\"\n",
    )
    .unwrap();
    let list = own_list_and_z();
    let pending = pending_in(&list, supersedes(), dir.path()).unwrap();
    assert!(pending.is_empty(), "{:?}", ids_of(&pending));
    // The four notes are left out because of the stamp: their own `done` can't see it.
    for id in [
        "20261107-projects-upgrading-past-the",
        "20261108-toolchain-neutral-land-builds",
        "20261109-a-build-s-tests",
        PROOF_ID,
    ] {
        let own = list.iter().find(|m| m.id == id).unwrap();
        assert!(!(own.done)(dir.path()), "{id}");
    }
    // An earlier stamp leaves them pending.
    fs::write(
        dir.path().join("clm.toml"),
        "upgraded = \"20261112-a-draft-s-clm\"\ndefects = \"ask\"\n",
    )
    .unwrap();
    let pending = pending_in(&list, supersedes(), dir.path()).unwrap();
    assert!(ids_of(&pending).contains(&PROOF_ID));
}
