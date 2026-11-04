//! `clm-land` names the remote to git only through `clm-git`'s `REMOTE`
//! (land/shared-plumbing).

use std::path::{Path, PathBuf};

/// Built from parts, so this file never holds the spellings it looks for.
const SPELLINGS: [&str; 3] = [
    concat!("\"", "origin\""),
    concat!("\"", "origin/"),
    concat!("remotes/", "origin"),
];

#[test]
fn no_copy_of_the_remote_name() {
    // req:land/shared-plumbing@7477d2fb
    let files = files(&src());
    assert!(!files.is_empty(), "no source files read");
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for (n, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if SPELLINGS.iter().any(|s| line.contains(s)) {
                found.push(format!("{}:{}: {}", file.display(), n + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "the remote's name spelled out: {found:#?}"
    );
}

#[test]
fn messages_keep_naming_origin() {
    // req:land/shared-plumbing@7477d2fb
    let reconnect = std::fs::read_to_string(src().join("reconnect.rs")).unwrap();
    assert!(reconnect.contains("origin's `main` has moved"));
    assert!(reconnect.contains("no `origin`: add the remote, then reconnect"));
    let lib = std::fs::read_to_string(src().join("lib.rs")).unwrap();
    assert!(lib.contains("offline: origin can't be reached; push and land when it can"));
}

fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every file under `dir`, recursively.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(files(&path));
        } else {
            out.push(path);
        }
    }
    out
}
