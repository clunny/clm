//! The workspace holds no `clm-watch` crate and `clm` has no `pr-body` subcommand
//! (clm/no-watch-crate).

use std::path::Path;
use std::process::Command;

/// The removed crate's name, built from parts so this file never names it whole.
const WATCH: &str = concat!("clm-", "watch");

#[test]
fn the_watch_crate_and_pr_body_are_gone() {
    // req:clm/no-watch-crate@c5595556
    let krate = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo = krate.join("../..");
    for manifest in [repo.join("Cargo.toml"), krate.join("Cargo.toml")] {
        let text = std::fs::read_to_string(&manifest).unwrap();
        assert!(
            !text.contains(WATCH),
            "{} names {WATCH}",
            manifest.display()
        );
    }
    let crates: Vec<String> = std::fs::read_dir(repo.join("crates"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!crates.is_empty(), "crates/ listed nothing");
    assert!(!crates.iter().any(|c| c == WATCH), "crates/{WATCH} exists");
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["pr-body", "x.md"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "clm pr-body isn't unknown");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("unrecognized subcommand"), "{err}");
}
