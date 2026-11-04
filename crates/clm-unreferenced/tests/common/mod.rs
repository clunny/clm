//! Fixture repositories for `scan`'s TypeScript and Python tests.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

pub const TAIL: &str = "is exported but nothing outside its package uses it";

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

/// A git repository holding `files`, all added to the index.
pub fn repo(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    for (rel, content) in files {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    git(dir.path(), &["add", "-A"]);
    dir
}

/// Every line `scan` prints with these `--allow` entries.
pub fn lines(dir: &Path, allow: &[&str]) -> Vec<String> {
    let allow: Vec<String> = allow.iter().map(|s| s.to_string()).collect();
    clm_unreferenced::scan(dir, &allow, &[]).unwrap()
}

/// The full names of the package items `scan` reports, sorted.
pub fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = lines(dir, &[])
        .iter()
        .filter_map(|line| {
            let (_, rest) = line.split_once(": ")?;
            rest.strip_suffix(&format!(" {TAIL}")).map(str::to_string)
        })
        .collect();
    names.sort();
    names
}
