//! `clm-guard` runs `git` and `gh` only through `clm-git` (plugin.hooks.guard/git-through-layer),
//! and `clm-hooks` holds none of the guard.

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

fn crates() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}

#[test]
fn runs_git_and_gh_only_through_the_layer() {
    // req:plugin.hooks.guard/git-through-layer@e2616286
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

#[test]
fn nothing_of_the_guard_is_left_in_clm_hooks() {
    // req:plugin.hooks.guard/git-through-layer@e2616286
    let hooks = crates().join("clm-hooks");
    let src: Vec<String> = std::fs::read_dir(hooks.join("src"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        !src.iter().any(|name| name.starts_with("guard")),
        "clm-hooks still holds a guard module: {src:?}"
    );
    let lib = std::fs::read_to_string(hooks.join("src/lib.rs")).unwrap();
    assert!(
        !lib.contains("mod guard"),
        "clm-hooks/src/lib.rs names a guard module"
    );
    assert!(
        !hooks.join("tests/guard.rs").exists(),
        "clm-hooks still holds the guard's tests"
    );
    let main = std::fs::read_to_string(crates().join("clm/src/main.rs")).unwrap();
    assert!(
        main.contains("clm_guard::guard("),
        "`clm hook guard` doesn't call clm_guard::guard"
    );
    assert!(
        !main.contains("clm_hooks::guard("),
        "`clm hook guard` still calls clm_hooks::guard"
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
