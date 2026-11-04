//! `clm-sync` reads revisions through `clm-git`'s `extract_paths`, with no extract of its
//! own (sync/extract-through-layer). Outside `src/`, so its own text isn't scanned.

use std::path::{Path, PathBuf};

/// Built from parts, so this file never holds what it looks for.
const GONE: [&str; 2] = [
    concat!("fn ", "extract("),
    concat!("--ignore-", "skip-worktree-bits"),
];

#[test]
fn no_extract_of_its_own() {
    // req:sync/extract-through-layer@5c278f6c
    let files = rust_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    assert!(!files.is_empty(), "no source files read");
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for (n, line) in text.lines().enumerate() {
            if GONE.iter().any(|g| line.contains(g)) {
                found.push(format!("{}:{}", file.display(), n + 1));
            }
        }
    }
    assert!(found.is_empty(), "an extract of its own: {found:?}");
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
