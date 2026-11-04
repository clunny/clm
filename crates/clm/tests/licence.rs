//! The repository's licence: `LICENSE` and the workspace manifest's `license`.

use std::fs;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// req:clm/mit-licence@60693bca
#[test]
fn repository_is_mit_licensed() {
    let licence = fs::read_to_string(repo().join("LICENSE")).expect("LICENSE exists");
    assert!(
        licence.starts_with("MIT License"),
        "LICENSE starts with MIT License"
    );
    assert!(licence.contains("Copyright (c) 2026 Christopher Lunny"));
    assert!(licence.contains("THE SOFTWARE IS PROVIDED \"AS IS\""));

    let manifest: toml::Table = fs::read_to_string(repo().join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        manifest["workspace"]["package"]["license"].as_str(),
        Some("MIT")
    );
}
