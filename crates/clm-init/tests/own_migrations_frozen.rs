//! This crate's own migrations are frozen once shipped, judged by `upgrade`'s `check_lock`.

use std::fs;
use std::path::Path;

use clm_init::own_migrations;
use clm_upgrade::{append_lock, check_lock};
use tempfile::TempDir;

// clm:reads crates/clm-init/migrations.lock
// req:init/own-migrations-frozen@13e2d029
#[test]
fn the_crates_own_migrations_match_its_lock() {
    let ids: Vec<&str> = own_migrations().iter().map(|m| m.id).collect();
    check_lock(Path::new(env!("CARGO_MANIFEST_DIR")), &ids).unwrap();
}

// req:init/own-migrations-frozen@13e2d029
#[test]
fn a_crate_with_no_lock_file_passes() {
    let dir = TempDir::new().unwrap();
    assert!(!dir.path().join("migrations.lock").exists());
    check_lock(dir.path(), &[]).unwrap();
}

// req:init/own-migrations-frozen@13e2d029
#[test]
fn a_shipped_migration_edited_is_reported() {
    let dir = TempDir::new().unwrap();
    let z = dir.path().join("src/migrations/z");
    fs::create_dir_all(&z).unwrap();
    fs::write(z.join("mod.rs"), "// z\n").unwrap();
    append_lock(dir.path(), "0.1.0", &["z"]).unwrap();
    check_lock(dir.path(), &["z"]).unwrap();
    fs::write(z.join("mod.rs"), "// Z\n").unwrap();
    let err = check_lock(dir.path(), &["z"]).unwrap_err();
    assert!(err.contains("`z`"), "{err}");
}
