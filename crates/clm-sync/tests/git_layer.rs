//! `clm-sync` runs `git` and `gh` only through `clm-git` (sync/git-through-layer).

use std::path::{Path, PathBuf};

/// Built from parts, so this file never holds the calls it looks for. The scenario names the
/// first two; the last two catch any process run however it is written, such as the crate's
/// old wrapper or a `use` of `std::process`, so it can't come back.
const CALLS: [&str; 4] = [
    concat!("Command::new(", "\"git\")"),
    concat!("Command::new(", "\"gh\")"),
    concat!("Command", "::new"),
    concat!("std", "::process"),
];

#[test]
fn runs_git_and_gh_only_through_the_layer() {
    // req:sync/git-through-layer@c3a7f356
    let krate = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: toml::Table = std::fs::read_to_string(krate.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        manifest["dependencies"]
            .as_table()
            .unwrap()
            .contains_key("clm-git"),
        "clm-sync doesn't list clm-git"
    );
    let files = rust_files(&krate.join("src"));
    assert!(!files.is_empty(), "no source files read");
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for (n, line) in text.lines().enumerate() {
            if CALLS.iter().any(|call| line.contains(call)) {
                found.push(format!("{}:{}", file.display(), n + 1));
            }
        }
    }
    assert!(found.is_empty(), "processes run outside clm-git: {found:?}");
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
