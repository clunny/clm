//! `clm-regenerate` runs `git` and `gh` only through `clm-git`, and `clm-sync` keeps no copy
//! of what moved here (sync.regenerate/git-through-layer). Outside `src/`, so its own text
//! isn't scanned.

use std::path::{Path, PathBuf};

/// Built from parts, so this file never holds what it looks for. The first four are the
/// scenario's; the next two catch any process run however it is written.
const GONE_HERE: [&str; 6] = [
    concat!("Command::new(", "\"git\")"),
    concat!("Command::new(", "\"gh\")"),
    concat!("\"", "origin"),
    concat!("fn ", "extract("),
    concat!("--ignore-", "skip-worktree-bits"),
    concat!("std", "::process"),
];

/// What `clm-sync` no longer defines or declares: `clm-regenerate`'s alone.
const GONE_FROM_SYNC: [&str; 8] = [
    concat!("fn ", "regenerated("),
    concat!("fn ", "retried("),
    concat!("fn ", "merge_file("),
    concat!("fn ", "other_deltas("),
    concat!("mod ", "merge"),
    concat!("mod ", "stale"),
    concat!("mod ", "stages"),
    concat!("mod ", "lock"),
];

#[test]
fn covered_by_the_scan() {
    // req:sync.regenerate/git-through-layer@23e72be1
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
        "clm-regenerate doesn't list clm-git"
    );
    let found = occurrences(&krate.join("src"), &GONE_HERE);
    assert!(found.is_empty(), "outside the layer: {found:?}");
}

#[test]
fn no_copy_left_in_clm_sync() {
    // req:sync.regenerate/git-through-layer@23e72be1
    let sync = Path::new(env!("CARGO_MANIFEST_DIR")).join("../clm-sync/src");
    let found = occurrences(&sync, &GONE_FROM_SYNC);
    assert!(found.is_empty(), "moved code still in clm-sync: {found:?}");
}

/// `file:line` of each line under `dir` that holds one of `needles`.
fn occurrences(dir: &Path, needles: &[&str]) -> Vec<String> {
    let files = rust_files(dir);
    assert!(
        !files.is_empty(),
        "no source files read in {}",
        dir.display()
    );
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for (n, line) in text.lines().enumerate() {
            if needles.iter().any(|needle| line.contains(needle)) {
                found.push(format!("{}:{}", file.display(), n + 1));
            }
        }
    }
    found
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
