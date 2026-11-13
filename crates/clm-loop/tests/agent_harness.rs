//! The loop follows the agent harness's abilities (`loop-follows-agent-harness`).

mod common;

use std::fs;

use clm_agent_harness::AgentHarness;
use clm_loop::{returned, start, step};
use common::{Fixture, line};

fn launches(actions: &[String]) -> Vec<&String> {
    actions
        .iter()
        .filter(|a| a.starts_with("launch "))
        .collect()
}

#[test]
fn no_subagents() {
    // req:orchestrator.loop/loop-follows-agent-harness@3f186db6
    let fx = Fixture::new();
    let mut options = fx.options();
    options.agent_harness = AgentHarness::None;
    fx.set("next", "a\nb\n");
    start(&options, "opus", false).unwrap();
    let first = step(&options).unwrap();
    let all = launches(&first);
    assert_eq!(all.len(), 1, "{first:?}");
    assert!(all[0].starts_with("launch build:a "));
    let prompt = fs::read_to_string(all[0].rsplit(' ').next().unwrap()).unwrap();
    assert!(
        prompt.starts_with("loop run build:a\nlauncher for a\n"),
        "{prompt}"
    );
    assert!(prompt.contains("in the session itself"), "{prompt}");
    assert_eq!(
        fx.ran("run b --review-model opus"),
        0,
        "b isn't prepared yet"
    );

    clm_loop::launched(&options, "build:a", "t-1").unwrap();
    fx.set("finish-a", "landed impl/a\n");
    fx.set("next", "b\n");
    let after = returned(&options, "build:a", "landed 1a2b3c4", None).unwrap();
    line(&after, "launch build:b ");
}

#[test]
fn no_isolated_worktrees_and_no_model_choice() {
    // req:orchestrator.loop/loop-follows-agent-harness@3f186db6
    let fx = Fixture::new();
    let mut options = fx.options();
    options.agent_harness = AgentHarness::Codex;
    fx.set("next", "a\n");
    let actions = start(&options, "opus", false).unwrap();

    // The session's model, since the agent harness can't choose one.
    let launch = line(&actions, "launch build:a build opus ").to_string();
    let worktree = fx.work.join(".claude/worktrees/agent-build-a");
    assert!(worktree.join("README").exists(), "added from origin/main");
    let prompt = fs::read_to_string(launch.rsplit(' ').next().unwrap()).unwrap();
    assert!(prompt.contains(worktree.to_str().unwrap()), "{prompt}");

    // A step again adds nothing: the worktree stands.
    step(&options).unwrap();
}

#[test]
fn claude_code_needs_neither() {
    // req:orchestrator.loop/loop-follows-agent-harness@3f186db6
    let fx = Fixture::new();
    fx.set("next", "a\nb\n");
    let actions = start(&fx.options(), "opus", false).unwrap();
    assert_eq!(launches(&actions).len(), 2);
    assert!(!fx.work.join(".claude/worktrees").exists());
    let a = line(&actions, "launch build:a ");
    assert!(a.starts_with("launch build:a build sonnet "), "{a}");
}
