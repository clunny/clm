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

// req:clm/readme-front-page@dcb1846c
#[test]
fn readme_is_the_front_page() {
    let readme = fs::read_to_string(repo().join("README.md")).expect("README.md exists");
    for needle in [
        "/plugin marketplace add clunny/clm",
        "/plugin install clm@clm",
        "docs/guide/quickstart.md",
        "CONTRIBUTING.md",
        ".github/SECURITY.md",
        "LICENSE",
        "ARCHITECTURE.md",
        "CAPABILITIES.md",
        "arXiv:2606.27045",
    ] {
        assert!(readme.contains(needle), "README.md holds {needle}");
    }
    // Every relative link names an existing file or directory.
    for part in readme.split("](").skip(1) {
        let target = part.split(')').next().unwrap();
        if target.contains("://") || target.starts_with('#') {
            continue;
        }
        let path = target.split('#').next().unwrap();
        assert!(repo().join(path).exists(), "link {target} exists");
    }
}
