#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

/// `git` in `dir` for test setup, panicking on failure.
pub fn sh(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// A repository on branch `main` with one commit holding `files`.
pub fn repo(dir: &Path, files: &[(&str, &str)]) {
    sh(dir, &["init", "-q", "-b", "main"]);
    sh(dir, &["config", "user.email", "t@example.com"]);
    sh(dir, &["config", "user.name", "t"]);
    commit(dir, files, "first");
}

pub fn commit(dir: &Path, files: &[(&str, &str)], message: &str) -> String {
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    sh(dir, &["add", "-A"]);
    sh(dir, &["commit", "-q", "--allow-empty", "-m", message]);
    sh(dir, &["rev-parse", "HEAD"])
}
