//! `clm-land` runs `git` and `gh` only through `clm-git` (land/git-through-layer).

use std::path::{Path, PathBuf};

/// Built from parts, so this file never holds the calls it looks for.
const CALLS: [&str; 2] = [
    concat!("Command::new(", "\"git\")"),
    concat!("Command::new(", "\"gh\")"),
];

#[test]
fn covered_by_the_scan() {
    // req:land/git-through-layer@1446b686
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
        "clm-land doesn't list clm-git"
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
    assert!(found.is_empty(), "git or gh run outside clm-git: {found:?}");
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
