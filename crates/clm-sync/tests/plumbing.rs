//! `clm-sync` shares `clm-git`'s plumbing (sync/shared-plumbing).

use std::path::{Path, PathBuf};

#[test]
fn the_error_is_the_shared_failure() {
    // req:sync/shared-plumbing@0453c1dd
    let failure: clm_git::Failure = clm_sync::Error::Refused("x".into());
    assert_eq!(failure.exit_code(), 1);
}

#[test]
fn no_copy_of_the_remote_name() {
    // req:sync/shared-plumbing@0453c1dd
    // Built from parts, so this file never holds what it looks for.
    let needle = concat!("\"", "origin");
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = rust_files(&src);
    assert!(!files.is_empty(), "no source files read");
    let found: Vec<String> = files
        .iter()
        .filter(|f| std::fs::read_to_string(f).unwrap().contains(needle))
        .map(|f| f.display().to_string())
        .collect();
    assert!(found.is_empty(), "the remote's name copied in {found:?}");
}

#[test]
fn one_mapping_in_the_dispatcher() {
    // req:sync/shared-plumbing@0453c1dd
    let main = Path::new(env!("CARGO_MANIFEST_DIR")).join("../clm/src/main.rs");
    let text = std::fs::read_to_string(main).unwrap();
    assert!(
        text.contains(concat!("fn ", "failed(")),
        "no `failed` helper"
    );
    for gone in [
        concat!("fn ", "synced("),
        concat!("clm_sync::Error", "::Refused"),
        concat!("clm_sync::Error", "::Git"),
    ] {
        assert!(!text.contains(gone), "main.rs still holds {gone}");
    }
}

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out
}
