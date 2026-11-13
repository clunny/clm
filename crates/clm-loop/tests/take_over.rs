//! Taking over a live loop whose subagents may still run (`take-over-holds-moving-orphans`).

mod common;

use std::ffi::OsStr;

use clm_loop::{Kind, RunState, start, step, stored_plan};
use common::{Fixture, run};

/// Pushes a commit made `minutes_ago` minutes ago to `origin`'s `branch`.
fn push_branch(fx: &Fixture, branch: &str, minutes_ago: u64) {
    let date = format!("{} +0000", clm_git::now() - minutes_ago * 60);
    let env: [(&str, &OsStr); 2] = [
        ("GIT_AUTHOR_DATE", OsStr::new(&date)),
        ("GIT_COMMITTER_DATE", OsStr::new(&date)),
    ];
    let commit = clm_git::run(
        &fx.work,
        "git",
        &["commit-tree", "HEAD^{tree}", "-p", "HEAD", "-m", "work"],
        clm_git::Options {
            stdin: None,
            env: &env,
        },
    )
    .unwrap();
    let commit = String::from_utf8(commit).unwrap();
    clm_git::git(
        &fx.work,
        &[
            "push",
            "-q",
            "-f",
            "origin",
            &format!("{}:refs/heads/{branch}", commit.trim()),
        ],
    )
    .unwrap();
}

fn running_a(fx: &Fixture) {
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
}

fn launches(actions: &[String]) -> Vec<&String> {
    actions
        .iter()
        .filter(|l| l.starts_with("launch "))
        .collect()
}

#[test]
fn a_branch_still_moving() {
    // req:orchestrator.loop/take-over-holds-moving-orphans@5a0f0917
    let fx = Fixture::new();
    running_a(&fx);
    push_branch(&fx, "impl/a", 3);
    fx.set_lease(2);
    let first = start(&fx.options(), "opus", true).unwrap();
    assert!(launches(&first).is_empty(), "{first:?}");

    // Five minutes later the branch still hasn't been still for 15.
    fx.change_run("build:a", |r| r.hold.as_mut().unwrap().since -= 5 * 60);
    let second = step(&fx.options()).unwrap();
    assert!(launches(&second).is_empty(), "{second:?}");
    let view = stored_plan(&fx.work).unwrap();
    let running = view.running.iter().find(|r| r.name == "build:a").unwrap();
    assert!(running.taken_over, "listed as held by a session taken over");
}

#[test]
fn a_branch_that_stayed_still_is_relaunched() {
    // req:orchestrator.loop/take-over-holds-moving-orphans@5a0f0917
    let fx = Fixture::new();
    running_a(&fx);
    push_branch(&fx, "impl/a", 3);
    fx.set_lease(2);
    start(&fx.options(), "opus", true).unwrap();
    // Seen at the same commit for 16 minutes.
    fx.change_run("build:a", |r| r.hold.as_mut().unwrap().since -= 16 * 60);
    let actions = step(&fx.options()).unwrap();
    assert_eq!(launches(&actions).len(), 1, "{actions:?}");
    assert!(fx.file().run("build:a").unwrap().hold.is_none());
}

#[test]
fn a_push_restarts_the_wait() {
    // req:orchestrator.loop/take-over-holds-moving-orphans@5a0f0917
    let fx = Fixture::new();
    running_a(&fx);
    push_branch(&fx, "impl/a", 3);
    fx.set_lease(2);
    start(&fx.options(), "opus", true).unwrap();
    fx.change_run("build:a", |r| r.hold.as_mut().unwrap().since -= 16 * 60);
    // The old session's subagent pushes again before the next step.
    push_branch(&fx, "impl/a", 0);
    let actions = step(&fx.options()).unwrap();
    assert!(launches(&actions).is_empty(), "{actions:?}");
    assert!(fx.file().run("build:a").unwrap().hold.is_some());
}

#[test]
fn a_branch_quiet_before_the_start_is_not_held() {
    // req:orchestrator.loop/take-over-holds-moving-orphans@5a0f0917
    let fx = Fixture::new();
    running_a(&fx);
    push_branch(&fx, "impl/a", 40);
    fx.set_lease(2);
    let actions = start(&fx.options(), "opus", true).unwrap();
    assert_eq!(launches(&actions).len(), 1, "{actions:?}");
}
