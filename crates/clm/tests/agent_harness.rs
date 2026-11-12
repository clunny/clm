//! `clm agent-harness` prints what the environment's agent harness can do
//! (clm/agent-harness-command).

use std::process::Command;

/// `clm agent-harness` outside any repository, with the given variables set and the other
/// agent markers removed.
fn run(set: &[(&str, &str)]) -> (i32, String) {
    let dir = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_clm"));
    command
        .arg("agent-harness")
        .current_dir(dir.path())
        .env_remove("CLAUDECODE")
        .env_remove(clm_agent_harness::CODEX_MARKER);
    for (k, v) in set {
        command.env(k, v);
    }
    let out = command.output().unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
    )
}

#[test]
fn under_codex() {
    // req:clm/agent-harness-command@8dc1a8d6
    let (code, out) = run(&[(clm_agent_harness::CODEX_MARKER, "t1")]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "agent harness codex\nconfirm no\nguard yes\nsession-hooks yes\nsubagents yes\n\
         isolated-worktrees no\nnamed-agents no\nmodel-choice no\n"
    );
}

#[test]
fn under_claude_code() {
    // req:clm/agent-harness-command@8dc1a8d6
    let (code, out) = run(&[("CLAUDECODE", "1")]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "agent harness claude-code\nconfirm yes\nguard yes\nsession-hooks yes\nsubagents yes\n\
         isolated-worktrees yes\nnamed-agents yes\nmodel-choice yes\nbuild-model sonnet\n"
    );
}

#[test]
fn under_no_agent_and_listed_in_help() {
    // req:clm/agent-harness-command@8dc1a8d6
    let (code, out) = run(&[]);
    assert_eq!(code, 0);
    assert!(out.starts_with("agent harness none\nconfirm no\n"), "{out}");
    assert!(!out.contains("build-model"));
    let help = Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("agent-harness")
    );
}
