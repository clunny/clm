//! Tests of the migration lock.

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::migrations;

/// A crate directory holding migrations `ids`, each with a `mod.rs` and a nested text file.
fn crate_with(ids: &[&str]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for id in ids {
        let m = dir.path().join("src/migrations").join(id);
        fs::create_dir_all(m.join("text")).unwrap();
        fs::write(m.join("mod.rs"), format!("// {id}\n")).unwrap();
        fs::write(m.join("text/old.txt"), "old\n").unwrap();
    }
    dir
}

// req:upgrade/shipped-migrations-frozen@b7c05840
#[test]
fn the_shipped_migrations_match_the_lock() {
    let ids: Vec<&str> = migrations().iter().map(|m| m.id).collect();
    crate::check_lock(Path::new(env!("CARGO_MANIFEST_DIR")), &ids).unwrap();
}

// req:upgrade/shipped-migrations-frozen@b7c05840
#[test]
fn the_directory_hash_covers_names_bytes_and_order() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("b"), "two").unwrap();
    fs::write(dir.path().join("a"), "one").unwrap();
    fs::write(dir.path().join("sub/c"), "three").unwrap();
    // The files in byte order of their paths, each as path, NUL, bytes, NUL.
    let mut text = Vec::new();
    for (name, body) in [("a", "one"), ("b", "two"), ("sub/c", "three")] {
        text.extend_from_slice(format!("{name}\0{body}\0").as_bytes());
    }
    let want: String = Sha256::digest(&text)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(crate::lock::dir_hash(dir.path()).unwrap(), want);
    fs::write(dir.path().join("a"), "ONE").unwrap();
    assert_ne!(crate::lock::dir_hash(dir.path()).unwrap(), want);
}

// req:upgrade/shipped-migrations-frozen@b7c05840
#[test]
fn an_edited_shipped_migration_fails_the_lock_naming_it_and_its_version() {
    let dir = crate_with(&["m"]);
    crate::append_lock(dir.path(), "0.2.0", &["m"]).unwrap();
    crate::check_lock(dir.path(), &["m"]).unwrap();
    fs::write(dir.path().join("src/migrations/m/text/old.txt"), "edited\n").unwrap();
    let err = crate::check_lock(dir.path(), &["m"]).unwrap_err();
    assert!(err.contains("`m`") && err.contains("0.2.0"), "{err}");
}

// req:upgrade/shipped-migrations-frozen@b7c05840
#[test]
fn a_locked_migration_the_binary_lacks_fails_the_lock() {
    let dir = crate_with(&["m"]);
    crate::append_lock(dir.path(), "0.2.0", &["m"]).unwrap();
    let err = crate::check_lock(dir.path(), &[]).unwrap_err();
    assert!(err.contains("`m`") && err.contains("0.2.0"), "{err}");
}

// req:upgrade/shipped-migrations-frozen@b7c05840
#[test]
fn locking_a_release_adds_only_the_unlisted_migrations() {
    let dir = crate_with(&["a", "b"]);
    assert_eq!(
        crate::append_lock(dir.path(), "0.2.0", &["a"]).unwrap(),
        ["a"]
    );
    let lock = dir.path().join("migrations.lock");
    let before = fs::read_to_string(&lock).unwrap();
    assert_eq!(
        crate::append_lock(dir.path(), "0.3.0", &["a", "b"]).unwrap(),
        ["b"]
    );
    let after = fs::read_to_string(&lock).unwrap();
    assert!(after.starts_with(&before));
    let added: Vec<&str> = after[before.len()..].lines().collect();
    assert_eq!(added.len(), 1);
    assert!(added[0].starts_with("b 0.3.0 "), "{added:?}");
    // Run again, it adds nothing.
    assert!(
        crate::append_lock(dir.path(), "0.4.0", &["a", "b"])
            .unwrap()
            .is_empty()
    );
    assert_eq!(fs::read_to_string(&lock).unwrap(), after);
}
