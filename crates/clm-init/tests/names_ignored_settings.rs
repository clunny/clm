//! `clm init` says when git ignores `.claude/settings.json`, so its allow rules would stay on
//! this machine.

use std::path::Path;
use std::process::Command;
use std::sync::Once;

use clm_init::{Options, init};
use tempfile::TempDir;

static ISOLATE: Once = Once::new();

/// Points git at no global or system configuration, so a developer's own excludes can't change
/// the outcome. Every test calls it first, before any thread reads the environment.
fn isolate_git() {
    ISOLATE.call_once(|| {
        let empty = std::env::temp_dir().join("clm-init-names-ignored-settings-empty");
        std::fs::create_dir_all(&empty).unwrap();
        // SAFETY: runs once, before any test of this file starts a git process, and every
        // test waits on the `Once`.
        unsafe {
            std::env::set_var("GIT_CONFIG_GLOBAL", "/dev/null");
            std::env::set_var("GIT_CONFIG_NOSYSTEM", "1");
            std::env::set_var("XDG_CONFIG_HOME", &empty);
        }
    });
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn repo() -> TempDir {
    isolate_git();
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    dir
}

fn run(dir: &Path) -> String {
    init(dir, &Options::default()).unwrap()
}

/// The next steps, one string per step.
fn steps(out: &str) -> Vec<&str> {
    out.split_once("Next steps (yours):\n")
        .map(|(_, rest)| rest.lines().collect())
        .unwrap_or_default()
}

fn push_step(steps: &[&str]) -> usize {
    steps
        .iter()
        .position(|s| s.contains("push them to `main`"))
        .expect("a push step")
}

fn naming_settings(steps: &[&str]) -> Vec<usize> {
    steps
        .iter()
        .enumerate()
        .filter(|(_, s)| s.contains(".claude/settings.json"))
        .map(|(i, _)| i)
        .collect()
}

#[test]
fn directory_ignored() {
    // req:init/init-names-ignored-settings@a3978231
    let dir = repo();
    std::fs::write(dir.path().join(".gitignore"), ".claude/\n").unwrap();
    let out = run(dir.path());
    let settings = std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
    assert!(settings.contains("Bash(clm land *)"), "{settings}");
    let ignore = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    assert!(ignore.lines().any(|l| l == ".claude/"), "{ignore}");
    let steps = steps(&out);
    let named = naming_settings(&steps);
    assert_eq!(named.len(), 1, "{out}");
    let step = steps[named[0]];
    assert!(step.contains("`.claude/*`") && step.contains("`!.claude/settings.json`"));
    assert!(step.contains("git add -f"), "{step}");
    assert!(named[0] < push_step(&steps), "{out}");
}

#[test]
fn not_ignored() {
    // req:init/init-names-ignored-settings@a3978231
    let dir = repo();
    let out = run(dir.path());
    assert!(naming_settings(&steps(&out)).is_empty(), "{out}");
}

#[test]
fn re_included() {
    // req:init/init-names-ignored-settings@a3978231
    let dir = repo();
    std::fs::write(
        dir.path().join(".gitignore"),
        ".claude/*\n!.claude/settings.json\n",
    )
    .unwrap();
    let out = run(dir.path());
    assert!(naming_settings(&steps(&out)).is_empty(), "{out}");
    let status = git(
        dir.path(),
        &["status", "--porcelain", "--untracked-files=all"],
    );
    assert!(
        status.lines().any(|l| l == "?? .claude/settings.json"),
        "{status}"
    );
}

#[test]
fn tracked_though_ignored() {
    // req:init/init-names-ignored-settings@a3978231
    let dir = repo();
    run(dir.path());
    let gitignore = dir.path().join(".gitignore");
    let mut text = std::fs::read_to_string(&gitignore).unwrap();
    text.push_str(".claude/\n");
    std::fs::write(&gitignore, text).unwrap();
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["add", "-f", ".claude/settings.json"]);
    git(dir.path(), &["commit", "-q", "-m", "init"]);
    let out = run(dir.path());
    assert!(
        git(dir.path(), &["status", "--porcelain"])
            .trim()
            .is_empty(),
        "{out}"
    );
    assert!(naming_settings(&steps(&out)).is_empty(), "{out}");
}

#[test]
fn ignored_on_a_rerun() {
    // req:init/init-names-ignored-settings@a3978231
    let dir = repo();
    run(dir.path());
    let gitignore = dir.path().join(".gitignore");
    let mut text = std::fs::read_to_string(&gitignore).unwrap();
    text.push_str(".claude/\n");
    std::fs::write(&gitignore, text).unwrap();
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "init"]);
    assert!(
        git(dir.path(), &["ls-files", ".claude"]).trim().is_empty(),
        "settings must stay untracked"
    );
    let out = run(dir.path());
    let steps = steps(&out);
    let named = naming_settings(&steps);
    assert!(!named.is_empty(), "{out}");
    assert!(steps[named[0]].contains("!.claude/settings.json"), "{out}");
    assert!(named[0] < push_step(&steps), "{out}");
}
