use std::fs;
use std::path::Path;
use std::process::Command;

use clm_view_repo::Budgets;
use tempfile::TempDir;

/// The call of `clm hook <name> --agent-harness <agent_harness>` on `event`, with no user
/// settings: what it wrote to standard output, and its status. It writes nothing to standard
/// error.
fn hook(name: &str, agent_harness: &str, event: &str) -> (String, u8) {
    let config = TempDir::new().unwrap();
    let home = config.path().display().to_string();
    let lookup = |key: &str| (key == "XDG_CONFIG_HOME").then(|| home.clone());
    let elsewhere = TempDir::new().unwrap();
    let cwd = elsewhere.path();
    let args = ["--agent-harness", agent_harness];
    let budgets = |_: &Path| {
        Ok(Budgets {
            spec_tokens: 1_000_000,
            code_tokens: 1_000_000,
        })
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = match name {
        "session-start" => clm_hooks::session_start_command(
            args, None, cwd, event, &lookup, &mut out, &mut err, &budgets,
        ),
        "autosync" => {
            clm_hooks::autosync_command(args, None, cwd, event, &lookup, &mut out, &mut err)
        }
        _ => panic!("no hook {name}"),
    };
    assert!(err.is_empty(), "{}", String::from_utf8_lossy(&err));
    (String::from_utf8(out).unwrap(), code)
}

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
    // req:plugin.hooks/hooks-per-agent-harness@3b9c1b3f
    let dir = TempDir::new().unwrap();
    git(&["init", "-q"], dir.path());
    write_spec(dir.path());
    let event =
        serde_json::json!({ "hook_event_name": "SessionStart", "cwd": dir.path() }).to_string();

    let (out, code) = hook("session-start", "codex", &event);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["hookSpecificOutput"]["hookEventName"], "SessionStart");
    let ctx = v["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(ctx.contains("\nCLM status\n"), "{ctx}");

    let (plain, code) = hook("session-start", "claude-code", &event);
    assert_eq!(code, 0);
    assert!(plain.starts_with("CLM state"), "{plain}");

    let (none, code) = hook("session-start", "none", &event);
    assert_eq!(code, 0);
    assert!(none.starts_with("CLM state"), "{none}");
    assert_eq!(plain, none);

    assert_eq!(
        hook("session-start", "codex", "not json"),
        (String::new(), 0)
    );
}

#[test]
fn codex_end_of_turn_autosyncs_silently() {
    // req:plugin.hooks/hooks-per-agent-harness@3b9c1b3f
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
    let (out, code) = hook("autosync", "codex", &event);
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
