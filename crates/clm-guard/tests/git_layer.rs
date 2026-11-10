//! `clm-guard` runs `git` and `gh` only through `clm-git` (plugin.hooks.guard/git-through-layer).

use std::path::{Path, PathBuf};

/// Built from parts, so this file never holds the calls it looks for. The scenario names the
/// first three (`recv_timeout` bounded a call on a thread of its own, which `run_within`'s
/// limit replaces); the last two catch any process run however it is written, such as a
/// wrapper of the crate's own or a `use` of `std::process`, so it can't come back.
const CALLS: [&str; 5] = [
    concat!("Command::new(", "\"git\")"),
    concat!("Command::new(", "\"gh\")"),
    concat!("recv", "_timeout"),
    concat!("Command", "::new"),
    concat!("std", "::process"),
];

#[test]
fn runs_git_and_gh_only_through_the_layer() {
    // req:plugin.hooks.guard/git-through-layer@79d6c2a1
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
        "clm-guard doesn't list clm-git"
    );
    let files = rust_files(&krate.join("src"));
    assert!(
        files.iter().any(|f| f.ends_with("shell/scan.rs")),
        "the shell module wasn't read: {files:?}"
    );
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
