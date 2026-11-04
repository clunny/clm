use std::fs;
use std::path::Path;

use clm_hooks::session_start;
use tempfile::TempDir;

fn start(cwd: &Path) -> String {
    session_start(&serde_json::json!({ "hook_event_name": "SessionStart", "cwd": cwd }).to_string())
}

#[test]
fn session_start_shows_status_in_a_clm_repo() {
    // req:plugin.hooks/session-status@1e909501
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
    // req:plugin.hooks/session-status@1e909501
    let dir = TempDir::new().unwrap();
    let ok = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(ok.success());
    write_spec(dir.path());
    fs::write(
        dir.path().join("clm.toml"),
        "[component_size]\nspec_tokens = 1\n",
    )
    .unwrap();
    let out = start(dir.path());
    assert!(out.contains("requirements current, 1 oversized\n"), "{out}");
    assert!(out.contains("  clm  active  0/0  oversized\n"), "{out}");
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
    // req:plugin.hooks/session-status@1e909501
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
    // req:plugin.hooks/session-status@1e909501
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
    // req:plugin.hooks/session-status@1e909501
    let dir = TempDir::new().unwrap();
    assert_eq!(start(dir.path()), "");
    // A broken spec tree: status fails, so the hook says nothing.
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(dir.path().join("specs/SPEC.md"), "---\ntitle: R\n---\n").unwrap();
    assert_eq!(start(dir.path()), "");
    assert_eq!(session_start("not json"), "");
}
