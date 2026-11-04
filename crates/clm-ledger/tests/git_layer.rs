//! `clm-ledger` runs `git` and `gh` only through `clm-git` (ledger/git-through-layer).

use std::path::{Path, PathBuf};

/// Built from parts, so this file never holds the calls it looks for. The scenario names the
/// first two; the last two catch any process run at all: the crate runs none itself.
const CALLS: [&str; 4] = [
    concat!("Command::new(", "\"git\")"),
    concat!("Command::new(", "\"gh\")"),
    concat!("Command", "::new("),
    concat!("process", "::Command"),
];

#[test]
fn runs_git_and_gh_only_through_the_layer() {
    // req:ledger/git-through-layer@c3b2a0b8
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
        "clm-ledger doesn't list clm-git"
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

/// Copies of the remote's name, built from parts so this file holds none of them.
const REMOTE_NAMES: [&str; 3] = [
    concat!("\"", "origin\""),
    concat!("\"", "origin/"),
    concat!("remotes/", "origin"),
];

#[test]
fn no_copy_of_the_remotes_name() {
    // req:ledger/shared-plumbing@10a161a9
    let files = rust_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    assert!(!files.is_empty(), "no source files read");
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for (n, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if REMOTE_NAMES.iter().any(|name| line.contains(name)) {
                found.push(format!("{}:{}: {}", file.display(), n + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "the remote's name spelled out: {found:#?}"
    );
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
