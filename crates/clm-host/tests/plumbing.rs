//! `clm-host` takes the remote's name from `clm-git`'s `REMOTE` (host/shared-plumbing).

use std::path::{Path, PathBuf};

/// Built from parts, so this file never holds what it looks for.
const NEEDLES: [&str; 3] = [
    concat!("\"", "origin\""),
    concat!("\"", "origin/"),
    concat!("remotes/", "origin"),
];

/// The lines of `text` that spell the remote's name, skipping comment lines.
fn copies(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .filter(|line| NEEDLES.iter().any(|needle| line.contains(needle)))
        .collect()
}

#[test]
fn no_copy_of_the_remote_name() {
    // req:host/shared-plumbing@a65259c7
    let files = all_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    assert!(!files.is_empty(), "no source files read");
    let found: Vec<String> = files
        .iter()
        .flat_map(|file| {
            let text = String::from_utf8_lossy(&std::fs::read(file).unwrap()).into_owned();
            copies(&text)
                .into_iter()
                .map(|line| format!("{}: {}", file.display(), line.trim()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(found.is_empty(), "the remote's name copied: {found:?}");
}

#[test]
fn the_scan_skips_only_comment_lines() {
    // req:host/shared-plumbing@a65259c7
    for needle in NEEDLES {
        assert_eq!(copies(&format!("    git(root, &[{needle}])")).len(), 1);
        assert_eq!(copies(&format!("x(); // {needle}")).len(), 1);
        assert!(copies(&format!("    /// {needle}")).is_empty());
        assert!(copies(&format!("//! {needle}")).is_empty());
    }
}

/// Every file under `dir`, recursively.
fn all_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(all_files(&path));
        } else {
            out.push(path);
        }
    }
    out
}
