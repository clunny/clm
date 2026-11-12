use std::fs;
use std::path::Path;
use std::process::Command;

use clm_agent_harness::AgentHarness;
use clm_hooks::{autosync_hook, session_start_hook};
use tempfile::TempDir;

fn git(args: &[&str], cwd: &Path) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .status()
        .unwrap();
    assert!(ok.success(), "git {args:?}");
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
fn session_start_is_written_in_each_harnesss_form() {
    // req:plugin.hooks/hooks-per-agent-harness@be580a3c
    let dir = TempDir::new().unwrap();
    write_spec(dir.path());
    let event =
        serde_json::json!({ "hook_event_name": "SessionStart", "cwd": dir.path() }).to_string();

    let (out, code) = session_start_hook(AgentHarness::Codex, &event);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["hookSpecificOutput"]["hookEventName"], "SessionStart");
    let ctx = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(ctx.contains("\nCLM status\n"), "{ctx}");

    let (plain, code) = session_start_hook(AgentHarness::ClaudeCode, &event);
    assert_eq!(code, 0);
    assert!(plain.starts_with("CLM state"), "{plain}");

    assert_eq!(
        session_start_hook(AgentHarness::Codex, "not json"),
        (String::new(), 0)
    );
}

#[test]
fn codex_end_of_turn_autosyncs_silently() {
    // req:plugin.hooks/hooks-per-agent-harness@be580a3c
    let dir = TempDir::new().unwrap();
    let remote = dir.path().join("remote.git");
    git(
        &["init", "-q", "--bare", remote.to_str().unwrap()],
        dir.path(),
    );
    let work = dir.path().join("work");
    git(
        &["init", "-q", "-b", "main", work.to_str().unwrap()],
        dir.path(),
    );
    write_spec(&work);
    git(
        &["remote", "add", "origin", remote.to_str().unwrap()],
        &work,
    );
    git(&["add", "."], &work);
    git(&["commit", "-qm", "init"], &work);
    git(&["push", "-q", "origin", "main"], &work);
    git(&["switch", "-qc", "spec/x"], &work);
    fs::create_dir_all(work.join("changes/20260101-x/deltas")).unwrap();
    fs::write(work.join("changes/20260101-x/deltas/a.md"), "delta\n").unwrap();

    let event = serde_json::json!({ "hook_event_name": "Stop", "cwd": work }).to_string();
    let (out, code) = autosync_hook(AgentHarness::Codex, &event);
    assert_eq!((out.as_str(), code), ("", 0));
    let listed = Command::new("git")
        .args([
            "ls-remote",
            remote.to_str().unwrap(),
            "refs/heads/autosync/spec/x",
        ])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&listed.stdout).contains("autosync/spec/x"),
        "no snapshot pushed"
    );
}
