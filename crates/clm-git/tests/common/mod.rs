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

/// Puts a fake `git` running `script` first on this process's `PATH`, once per test binary,
/// with the real `PATH` in `FAKE_REAL_PATH` so the script can find the real `git`. `write`
/// is `exec::write_exec`, which this module can't name, since not every binary has `exec`.
pub fn install_fake_git(script: &str, write: fn(&Path, &str)) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let bin = Box::leak(Box::new(tempfile::tempdir().unwrap()));
        write(&bin.path().join("git"), script);
        let real = std::env::var("PATH").unwrap();
        let path = format!("{}:{real}", bin.path().display());
        // SAFETY: every test of the binary calls this first, and `Once` blocks the others
        // until the variables are set, before any of them starts a command.
        unsafe {
            std::env::set_var("FAKE_REAL_PATH", real);
            std::env::set_var("PATH", path);
        }
    });
}

/// How many lines of the file at `calls` read `what`.
pub fn count_calls(calls: &Path, what: &str) -> usize {
    std::fs::read_to_string(calls)
        .unwrap_or_default()
        .lines()
        .filter(|l| *l == what)
        .count()
}
