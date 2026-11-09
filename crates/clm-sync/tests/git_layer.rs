//! `clm-sync` runs `git` and `gh` only through `clm-git` (sync/git-through-layer).

use std::path::{Path, PathBuf};

/// Built from parts, so this file never holds the calls it looks for. The scenario names the
/// first two; the last two catch any process run however it is written, such as the crate's
/// old wrapper or a `use` of `std::process`, so it can't come back.
const CALLS: [&str; 6] = [
    concat!("Command::new(", "\"git\")"),
    concat!("Command::new(", "\"gh\")"),
    concat!("clm_git", "::git("),
    concat!("clm_git", "::run("),
    concat!("Command", "::new"),
    concat!("std", "::process"),
];

#[test]
fn runs_git_and_gh_only_through_the_layer() {
    // req:sync/git-through-layer@808d5bc4
    let krate = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: toml::Table = std::fs::read_to_string(krate.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    for dependency in ["clm-git", "clm-regenerate"] {
        assert!(
            manifest["dependencies"]
                .as_table()
                .unwrap()
                .contains_key(dependency),
            "clm-sync doesn't list {dependency}"
        );
    }
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
        for import in imports_of_clm_git(&text) {
            if import
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|w| w == "git" || w == "run")
            {
                found.push(format!(
                    "{}: imports from clm_git: {import}",
                    file.display()
                ));
            }
        }
    }
    assert!(found.is_empty(), "processes run outside clm-git: {found:?}");
}

/// The text after `use clm_git::` up to its `;`, for each such import in `text`.
fn imports_of_clm_git(text: &str) -> Vec<String> {
    let start = concat!("use clm_git", "::");
    text.match_indices(start)
        .filter_map(|(at, _)| {
            let rest = &text[at + start.len()..];
            rest.find(';').map(|end| rest[..end].to_string())
        })
        .collect()
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
