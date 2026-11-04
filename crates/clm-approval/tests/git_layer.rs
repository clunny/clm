//! `clm-approval` runs `git` and `gh` only through `clm-git`, names the remote only through
//! its `REMOTE`, and `clm-land` keeps no copy of what it calls (land.approval/git-through-layer).

use std::path::{Path, PathBuf};

/// Built from parts, so this file never holds the strings it looks for.
const FORBIDDEN: [&str; 5] = [
    concat!("Command::new(", "\"git\")"),
    concat!("Command::new(", "\"gh\")"),
    concat!("\"", "origin\""),
    concat!("\"", "origin/"),
    concat!("remotes/", "origin"),
];
const COPIES: [&str; 4] = [
    concat!("fn ", "fingerprint("),
    concat!("fn ", "held("),
    concat!("fn ", "policy("),
    concat!("fn ", "slug("),
];

#[test]
fn covered_by_the_scan() {
    // req:land.approval/git-through-layer@397f9953
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
        "clm-approval doesn't list clm-git"
    );
    let own = rust_files(&krate.join("src"));
    assert!(!own.is_empty(), "no source files read");
    let mut found = Vec::new();
    for file in own {
        let text = std::fs::read_to_string(&file).unwrap();
        for (n, line) in text.lines().enumerate() {
            let code = !line.trim_start().starts_with("//");
            if code && FORBIDDEN.iter().any(|call| line.contains(call)) {
                found.push(format!("{}:{}", file.display(), n + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "git or the remote named outside clm-git: {found:?}"
    );

    let land = rust_files(&krate.join("../clm-land/src"));
    assert!(!land.is_empty(), "no clm-land sources read");
    let copies: Vec<_> = land
        .iter()
        .filter(|f| {
            let text = std::fs::read_to_string(f).unwrap();
            COPIES.iter().any(|c| text.contains(c))
        })
        .collect();
    assert!(copies.is_empty(), "clm-land keeps a copy: {copies:?}");
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
