//! `clm hook`'s `--agent-harness` flag, run as the binary.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde_json::Value;
use tempfile::TempDir;

/// A git repository with a valid `specs/` and, when given, a `clm.toml`.
fn clm_repo(settings: Option<&str>) -> TempDir {
    let dir = TempDir::new().unwrap();
    let init = Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(init.status.success(), "{init:?}");
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(
        dir.path().join("specs/SPEC.md"),
        "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n",
    )
    .unwrap();
    if let Some(settings) = settings {
        fs::write(dir.path().join("clm.toml"), settings).unwrap();
    }
    dir
}

fn hook_args(args: &[&str], event: &str) -> Output {
    // The developer's own settings must not leak in either.
    let config = TempDir::new().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("hook")
        .args(args)
        .env("XDG_CONFIG_HOME", config.path())
        .env_remove("CLM_AUTOSYNC")
        .env_remove("CLM_DEFECTS")
        // The developer's own git config (signing, hooks, helpers) must not leak in.
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(event.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn hook_flag_names_the_agent_harness() {
    // req:clm/hook-agent-harness-flag@91c2ac58
    let repo = clm_repo(None);
    let push = |cwd: Option<&Path>| {
        serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": { "command": "git push origin main" },
            "cwd": cwd,
        })
        .to_string()
    };
    let out = hook_args(
        &["guard", "--agent-harness", "codex"],
        &push(Some(repo.path())),
    );
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("guard-blocks-push-to-main"));

    // No flag: Claude Code's reading, the `ask` decision as before.
    let approve = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": { "command": "clm approve spec/x" },
    })
    .to_string();
    let out = hook_args(&["guard"], &approve);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let answer: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(answer["hookSpecificOutput"]["permissionDecision"], "ask");

    // Codex can't confirm, so the same ask is a block there.
    let out = hook_args(&["guard", "--agent-harness", "codex"], &approve);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty());

    // Codex's session start carries its text as `additionalContext`.
    let start = serde_json::json!({ "cwd": repo.path() }).to_string();
    let out = hook_args(&["session-start", "--agent-harness", "codex"], &start);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let answer: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(answer["hookSpecificOutput"]["additionalContext"].is_string());

    // An unknown name: exit 1 (no block for either agent harness), naming it.
    let out = hook_args(&["guard", "--agent-harness", "nope"], &push(None));
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("nope"));
    for name in ["session-start", "autosync"] {
        let out = hook_args(&[name, "--agent-harness", "nope"], &start);
        assert_eq!(out.status.code(), Some(0), "{name}: {out:?}");
        assert!(out.stdout.is_empty(), "{name}: {out:?}");
    }
}
