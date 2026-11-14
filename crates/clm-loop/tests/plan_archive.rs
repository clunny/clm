//! Archiving complete changes (`plan-archive`).

mod common;

use std::fs;

use clm_loop::{Kind, RunState, StopMode, start, step, stop};
use common::{Fixture, line, run};

#[test]
fn a_complete_change() {
    // req:orchestrator.loop/plan-archive@97501582
    let fx = Fixture::new();
    fx.set("wrap-up", "complete c\ncomplete d\n");
    let actions = start(&fx.options(), "opus", false).unwrap();
    let launch = line(&actions, "launch archive:c archive sonnet ");
    assert_eq!(
        actions.iter().filter(|a| a.starts_with("launch ")).count(),
        1,
        "the first change only: {actions:?}"
    );
    let prompt = fs::read_to_string(launch.rsplit(' ').next().unwrap()).unwrap();
    assert!(prompt.starts_with("loop run archive:c\n"), "{prompt}");
    for needle in [
        "git switch -c impl/archive/c origin/main",
        "clm rebase",
        "rebase ok",
        "clm branch push",
        "clm pr open",
        "clm land",
    ] {
        assert!(prompt.contains(needle), "{needle} in {prompt}");
    }
    // It takes no place, and every step prints it again until reported, prepared once.
    assert_eq!(step(&fx.options()).unwrap(), [launch.to_string()]);
}

#[test]
fn an_archive_branch_the_loop_doesnt_hold() {
    // req:orchestrator.loop/plan-archive@97501582
    let fx = Fixture::new();
    clm_git::git(
        &fx.work,
        &["push", "-q", "origin", "HEAD:refs/heads/impl/archive/c"],
    )
    .unwrap();
    fx.set("wrap-up", "complete c\n");
    let actions = start(&fx.options(), "opus", false).unwrap();
    assert_eq!(actions, ["wait"], "another session archives it");

    // A branch a run of the loop holds is the loop's own: its run continues, unlaunched twice.
    fx.add_run(run(
        "archive:c",
        Kind::Archive,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    assert_eq!(step(&fx.options()).unwrap(), ["wait"]);
}

#[test]
fn no_archive_while_stopping() {
    // req:orchestrator.loop/plan-archive@97501582
    let fx = Fixture::new();
    fx.set("wrap-up", "complete c\n");
    stop(&fx.options(), StopMode::Drain).unwrap();
    let actions = step(&fx.options()).unwrap();
    assert!(
        !actions.iter().any(|a| a.starts_with("launch ")),
        "{actions:?}"
    );
}
