//! `clm hook session-start` through `clm-hooks`' call of it (`commands-in-crate`): finding the
//! CLM repository and printing its status (`session-status`).

use std::fs;
use std::path::{Path, PathBuf};

use clm_model::Violation;
use clm_view_repo::Budgets;
use tempfile::TempDir;

/// Budgets no fixture here reaches.
fn roomy(_: &Path) -> Result<Budgets, Violation> {
    Ok(Budgets {
        spec_tokens: 1_000_000,
        code_tokens: 1_000_000,
    })
}

/// The call of `clm hook session-start` on `event`, given `budgets`, with no user settings
/// through its lookup: what it wrote to each stream, and its status.
fn call(
    event: &str,
    budgets: &dyn Fn(&Path) -> Result<Budgets, Violation>,
) -> (String, String, u8) {
    let elsewhere = TempDir::new().unwrap();
    call_from(elsewhere.path(), event, budgets)
}

/// `call`, run from `cwd`.
fn call_from(
    cwd: &Path,
    event: &str,
    budgets: &dyn Fn(&Path) -> Result<Budgets, Violation>,
) -> (String, String, u8) {
    let config = TempDir::new().unwrap();
    let home = config.path().display().to_string();
    let lookup = |key: &str| (key == "XDG_CONFIG_HOME").then(|| home.clone());
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = clm_hooks::session_start_command(
        std::iter::empty::<&str>(),
        clm_hooks::clm_root(cwd),
        cwd,
        event,
        &lookup,
        &mut out,
        &mut err,
        budgets,
    );
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
        code,
    )
}

/// What the session start prints for a `SessionStart` event whose `cwd` is `cwd`; it must
/// exit 0.
fn start(cwd: &Path) -> String {
    let event = serde_json::json!({ "hook_event_name": "SessionStart", "cwd": cwd }).to_string();
    let (out, _, code) = call(&event, &roomy);
    assert_eq!(code, 0, "{out}");
    out
}

#[test]
fn session_start_shows_status_in_a_clm_repo() {
    // req:plugin.hooks/session-status@441436bd
    let dir = TempDir::new().unwrap();
    write_spec(dir.path());
    let out = start(dir.path());
    assert!(out.starts_with("CLM state"), "{out}");
    assert!(out.contains("\nCLM status\n"), "{out}");
    for section in ["\nchanges\n", "\nclaims\n"] {
        assert!(out.contains(section), "{section} missing from {out}");
    }
}

#[test]
fn session_start_marks_oversized_nodes_as_status_does() {
    // req:plugin.hooks/session-status@441436bd
    let dir = git_repo();
    write_spec(dir.path());
    let event = serde_json::json!({ "cwd": dir.path() }).to_string();
    // A component that sets no budget gets the call's: one token.
    let tiny = |_: &Path| {
        Ok(Budgets {
            spec_tokens: 1,
            code_tokens: 1_000_000,
        })
    };
    let (out, err, code) = call(&event, &tiny);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("requirements current, 1 oversized\n"), "{out}");
    assert!(out.contains("  clm  active  0/0  oversized\n"), "{out}");
    assert!(err.is_empty(), "{err}");
    // Budgets that fail: the status without the size, and why on the error stream.
    let bad = |root: &Path| {
        Err(Violation {
            path: PathBuf::from(root),
            rule: "check/node-size",
            message: "bad".into(),
        })
    };
    let (out, err, code) = call(&event, &bad);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\nCLM status\n"), "{out}");
    assert!(!out.contains("oversized"), "{out}");
    assert!(err.starts_with("size not checked: bad"), "{err}");
}

fn git_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let ok = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(ok.success());
    dir
}

fn write_spec(root: &Path) {
    fs::create_dir_all(root.join("specs")).unwrap();
    fs::write(
        root.join("specs/SPEC.md"),
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    )
    .unwrap();
}

#[test]
fn session_start_finds_the_repo_from_a_subdirectory() {
    // req:plugin.hooks/session-status@441436bd
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join(".git")).unwrap();
    write_spec(dir.path());
    let cwd = dir.path().join("crates/foo");
    fs::create_dir_all(&cwd).unwrap();
    let out = start(&cwd);
    assert!(out.contains("\nchanges\n"), "{out}");
}

#[test]
fn session_start_stops_at_the_git_root() {
    // req:plugin.hooks/session-status@441436bd
    let outer = TempDir::new().unwrap();
    write_spec(outer.path());
    let inner = outer.path().join("vendor/other");
    fs::create_dir_all(inner.join(".git")).unwrap();
    let cwd = inner.join("src");
    fs::create_dir_all(&cwd).unwrap();
    assert_eq!(start(&cwd), "");
    // A worktree's `.git` is a file, and it stops the walk too.
    fs::remove_dir(inner.join(".git")).unwrap();
    fs::write(inner.join(".git"), "gitdir: /elsewhere\n").unwrap();
    assert_eq!(start(&cwd), "");
}

#[test]
fn session_start_is_silent_elsewhere() {
    // req:plugin.hooks/session-status@441436bd
    let dir = TempDir::new().unwrap();
    assert_eq!(start(dir.path()), "");
    // A broken spec tree: status fails, so the hook says nothing.
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(dir.path().join("specs/SPEC.md"), "---\ntitle: R\n---\n").unwrap();
    // An up-to-date project allows the unattended `clm` rules, so no upgrade line shows
    // (`upgrade/migration-unattended-allow-rules`).
    let rules: Vec<String> = [
        "apply",
        "branch",
        "build",
        "check",
        "claim",
        "defect",
        "gate",
        "land",
        "pr",
        "rebase",
        "setup",
        "verify",
        "workaround",
    ]
    .iter()
    .flat_map(|s| [format!("Bash(clm {s})"), format!("Bash(clm {s} *)")])
    .collect();
    fs::create_dir_all(dir.path().join(".claude")).unwrap();
    fs::write(
        dir.path().join(".claude/settings.json"),
        format!(
            "{{\"permissions\":{{\"allow\":{}}}}}",
            serde_json::json!(rules)
        ),
    )
    .unwrap();
    assert_eq!(start(dir.path()), "");
    // An event that isn't JSON, or holds no `cwd`, leads nowhere, even run from a CLM
    // repository.
    let repo = TempDir::new().unwrap();
    write_spec(repo.path());
    assert!(start(repo.path()).starts_with("CLM state"));
    for event in ["not json", "{}"] {
        let printed = call_from(repo.path(), event, &roomy);
        assert_eq!(printed, (String::new(), String::new(), 0), "{event}");
    }
}
