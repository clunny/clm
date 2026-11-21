//! A stamp read against the list the caller gives, and a crate without a lock.

use std::fs;

use clm_upgrade::{TooNew, check_lock, migrations, pending_in, supersedes};
use tempfile::TempDir;

// req:upgrade/pending-migrations@0b011156
#[test]
fn a_stamp_naming_a_later_own_migration_is_too_new_for_a_shorter_list() {
    let dir = TempDir::new().unwrap();
    let later = migrations()[3].id;
    fs::write(
        dir.path().join("clm.toml"),
        format!("upgraded = \"{later}\"\n"),
    )
    .unwrap();
    assert!(pending_in(migrations(), supersedes(), dir.path()).is_ok());
    let err = pending_in(&migrations()[..2], supersedes(), dir.path()).unwrap_err();
    assert_eq!(err, TooNew(later.to_string()));
}

// req:upgrade/shipped-migrations-frozen@290ccdb6
#[test]
fn a_missing_lock_reads_as_empty() {
    let dir = TempDir::new().unwrap();
    assert_eq!(check_lock(dir.path(), &[]), Ok(()));
}
