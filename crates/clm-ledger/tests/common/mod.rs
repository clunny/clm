#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// `git` in `dir` for test setup, panicking on failure.
pub fn sh(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("HOME", dir)
        .env_remove("XDG_CONFIG_HOME")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// A repository on branch `main` with one commit, its git `user.name` `name`.
pub fn repo(dir: &Path, name: &str) {
    sh(dir, &["init", "-q", "-b", "main"]);
    sh(dir, &["config", "user.email", "t@example.com"]);
    sh(dir, &["config", "user.name", name]);
    commit(dir, &[("README.md", "x\n")], "first");
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

/// A bare `origin` in `dir/origin.git` and a clone of it in `dir/<name>`, with git user
/// `user`, holding one commit on `main`.
pub fn clone_with_origin(dir: &Path, name: &str, user: &str) -> PathBuf {
    let origin = dir.join("origin.git");
    if !origin.exists() {
        std::fs::create_dir_all(&origin).unwrap();
        sh(&origin, &["init", "-q", "--bare", "-b", "main"]);
        let seed = dir.join("seed");
        std::fs::create_dir_all(&seed).unwrap();
        repo(&seed, "seed");
        sh(
            &seed,
            &["remote", "add", "origin", origin.to_str().unwrap()],
        );
        sh(&seed, &["push", "-q", "origin", "main"]);
    }
    sh(
        dir,
        &[
            "clone",
            "-q",
            origin.to_str().unwrap(),
            dir.join(name).to_str().unwrap(),
        ],
    );
    let clone = dir.join(name);
    sh(&clone, &["config", "user.email", "t@example.com"]);
    sh(&clone, &["config", "user.name", user]);
    clone
}

/// An environment with only `vars` set, and `HOME` pointing at `home` so no user's
/// `config.toml` is read.
pub fn env<'a>(
    home: &'a Path,
    vars: &'a [(&'a str, &'a str)],
) -> impl Fn(&str) -> Option<String> + 'a {
    move |key| match key {
        "HOME" => Some(home.to_string_lossy().into_owned()),
        _ => vars
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.to_string()),
    }
}
