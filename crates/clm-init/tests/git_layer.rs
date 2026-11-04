//! `clm-init` runs `git` and `gh` only through `clm-git` (init/git-through-layer), and names
//! the remote only through its `REMOTE` (init/shared-plumbing).

use std::path::{Path, PathBuf};

/// Built from parts, so this file never holds the calls it looks for. The scenario names the
/// first two; the last two catch any process run, such as a wrapper taking the program as an
/// argument, so the crate's own wrapper can't come back.
const CALLS: [&str; 4] = [
    concat!("Command::new(", "\"git\")"),
    concat!("Command::new(", "\"gh\")"),
    concat!("Command", "::new("),
    concat!("process", "::Command"),
];

#[test]
fn runs_git_and_gh_only_through_the_layer() {
    // req:init/git-through-layer@a0b05934
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
        "clm-init doesn't list clm-git"
    );
    let found = lines_holding(&CALLS, false);
    assert!(found.is_empty(), "processes run outside clm-git: {found:?}");
}

#[test]
fn lists_tracked_files_through_the_layer() {
    // req:init/survey-tracked-through-git@ce420ccf
    // Comments count too: a copy of its own would show up as the command's name anywhere.
    let found = lines_holding(&[concat!("ls", "-files")], false);
    assert!(
        found.is_empty(),
        "files listing tracked files themselves: {found:?}"
    );
}

#[test]
fn names_the_remote_only_through_the_layer() {
    // req:init/shared-plumbing@d34c88aa
    // Built from parts, so this file never holds what it looks for.
    let needles = [
        concat!("\"", "origin\""),
        concat!("\"", "origin/"),
        concat!("remotes/", "origin"),
    ];
    let found = lines_holding(&needles, true);
    assert!(
        found.is_empty(),
        "remote named outside clm-git's REMOTE: {found:?}"
    );
}

/// `<file>:<line>` for each line of a file under the crate's `src` holding one of `needles`,
/// skipping each line whose first non-blank characters are `//` when `skip_comments` is set.
fn lines_holding(needles: &[&str], skip_comments: bool) -> Vec<String> {
    let files = files_under(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    assert!(!files.is_empty(), "no source files read");
    let mut found = Vec::new();
    for file in files {
        let text = String::from_utf8_lossy(&std::fs::read(&file).unwrap()).into_owned();
        for (n, line) in text.lines().enumerate() {
            if skip_comments && line.trim_start().starts_with("//") {
                continue;
            }
            if needles.iter().any(|needle| line.contains(needle)) {
                found.push(format!("{}:{}", file.display(), n + 1));
            }
        }
    }
    found
}

/// Every file under `dir`, recursively.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(files_under(&path));
        } else {
            out.push(path);
        }
    }
    out
}
