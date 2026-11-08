#![allow(dead_code)] // each test file uses part of this

use std::fs;
use std::path::Path;
use std::process::Command;

use clm_check::Budgets;
use clm_health::Settings;
use tempfile::TempDir;

pub const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

pub fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Runs `git` in `root` with `env` set, failing the test when it fails.
pub fn git_env(root: &Path, args: &[&str], env: &[(&str, &str)]) -> String {
    let out = Command::new("git")
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .envs(env.iter().copied())
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

pub fn git(root: &Path, args: &[&str]) -> String {
    git_env(root, args, &[])
}

/// Commits everything at `secs_ago` seconds before now.
pub fn commit_ago(root: &Path, message: &str, secs_ago: u64) {
    let when = format!("@{} +0000", clm_git::now() - secs_ago);
    git(root, &["add", "-A"]);
    git_env(
        root,
        &["commit", "-q", "--allow-empty", "-m", message],
        &[("GIT_AUTHOR_DATE", &when), ("GIT_COMMITTER_DATE", &when)],
    );
}

/// A git repository holding the root `SPEC.md`.
pub fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    write(dir.path(), "specs/SPEC.md", ROOT);
    dir
}

/// A node's spec with `paths` and one requirement per slug.
pub fn spec(paths: &str, slugs: &[&str]) -> String {
    let mut s = format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\n---\n\n## Requirements\n"
    );
    for slug in slugs {
        s.push_str(&format!(
            "\n### Requirement: {slug}\n{slug}.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// The defaults `config` gives, with the size budgets `spec` and `code`.
pub fn settings(spec: u64, code: u64) -> Settings {
    Settings {
        reconsider: 3,
        scenarios: 6,
        exceptions: 4,
        stale_days: 30,
        lcov: None,
        land_check_target_seconds: 120,
        component_size: Budgets {
            spec_tokens: spec,
            code_tokens: code,
        },
    }
}

/// What `clm health` prints for `root` with `settings` and an expiry of 4 hours.
pub fn report_with(root: &Path, settings: &Settings) -> String {
    clm_health::report(root, settings, 4).unwrap().to_string()
}

pub fn report(root: &Path) -> String {
    report_with(root, &settings(12000, 40000))
}

/// The lines of `out` of signal `signal`, its summary first.
pub fn section<'a>(out: &'a str, signal: &str) -> Vec<&'a str> {
    let finding = format!(": health/{signal}: ");
    let summary = format!("health/{signal}: ");
    out.lines()
        .filter(|l| l.starts_with(&summary) || l.contains(&finding))
        .collect()
}
