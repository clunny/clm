//! `clm build stop` and the summary a drain or a stop ends with (`build-stop`, `end-summary`).

mod common;

use std::fs;

use clm_loop::{Kind, RunState, StopMode, returned, start, step, stop};
use common::{Fixture, line, run};

fn running(fx: &Fixture, name: &str, kind: Kind, task: &str) {
    fx.add_run(run(name, kind, "sonnet", RunState::Running, Some(task)));
}

/// The text of the file a `say` action names.
fn said(action: &str) -> String {
    fs::read_to_string(action.strip_prefix("say ").unwrap()).unwrap()
}

#[test]
fn stop_now() {
    // req:orchestrator.loop/build-stop@0b42fa2a
    let fx = Fixture::new();
    clm_git::git(
        &fx.work,
        &["push", "-q", "origin", "HEAD:refs/heads/impl/a"],
    )
    .unwrap();
    running(&fx, "build:a", Kind::Build, "t-1");
    running(&fx, "draft:c", Kind::Draft, "t-2");
    // Neither stop fetches: the remote is gone, and they still record.
    clm_git::git(&fx.work, &["remote", "set-url", "origin", "/nonexistent"]).unwrap();
    stop(&fx.options(), StopMode::Now).unwrap();
    assert_eq!(fx.file().stop, Some(StopMode::Now));
    clm_git::git(&fx.work, &["remote", "set-url", "origin", &origin(&fx)]).unwrap();

    let actions = step(&fx.options()).unwrap();
    assert_eq!(
        actions,
        ["stop build:a t-1", "stop draft:c t-2"],
        "a stop prints a stop for every running run, and launches nothing"
    );
    returned(&fx.options(), "build:a", "none", Some("stopped"), None).unwrap();
    let last = returned(&fx.options(), "draft:c", "none", Some("stopped"), None).unwrap();

    assert_eq!(last.len(), 2, "{last:?}");
    assert!(last[0].starts_with("say "), "{last:?}");
    assert_eq!(last[1], "end");
    assert!(
        !fx.calls().iter().any(|c| c.contains("--finish")),
        "a stopped run finishes nothing"
    );
    assert_eq!(fx.file().run("build:a").unwrap().state, RunState::Stopped);
    clm_git::git(&fx.work, &["fetch", "-q", "origin"]).unwrap();
    clm_git::git(&fx.work, &["rev-parse", "origin/impl/a"]).expect("origin/impl/a still exists");
}

fn origin(fx: &Fixture) -> String {
    fx.tmp
        .path()
        .canonicalize()
        .unwrap()
        .join("origin.git")
        .display()
        .to_string()
}

#[test]
fn a_drain_launches_nothing_new_and_lets_runs_finish() {
    // req:orchestrator.loop/build-stop@0b42fa2a
    let fx = Fixture::new();
    fx.set("next", "b\n");
    fx.set("triage", "group bug:x [x] P1 0 x1\n");
    running(&fx, "build:a", Kind::Build, "t-1");
    stop(&fx.options(), StopMode::Drain).unwrap();

    let actions = step(&fx.options()).unwrap();
    assert_eq!(actions, ["wait"], "no launch, and a run is still going");

    fx.set("finish-a", "landed impl/a\n");
    let actions = returned(&fx.options(), "build:a", "landed 1a2b3c4", None, None).unwrap();
    assert_eq!(actions.len(), 2, "{actions:?}");
    assert!(actions[0].starts_with("say "));
    assert_eq!(actions[1], "end");
    assert!(
        !fx.calls()
            .iter()
            .any(|c| c.starts_with("run b") || c.starts_with("draft ")),
        "{:?}",
        fx.calls()
    );
}

#[test]
fn the_summary_lists_only_landed_runs_as_landed() {
    // req:orchestrator.loop/end-summary@512bea55
    let fx = Fixture::new();
    running(&fx, "build:a", Kind::Build, "t-1");
    stop(&fx.options(), StopMode::Drain).unwrap();
    let actions = returned(&fx.options(), "build:a", "stopped: check fails", None, None).unwrap();
    let text = said(line(&actions, "say "));
    assert!(
        text.contains("Landed since the loop started:\n  nothing"),
        "{text}"
    );
}

#[test]
fn summary_with_a_workaround() {
    // req:orchestrator.loop/end-summary@512bea55
    let fx = Fixture::new();
    start(&fx.options(), "opus", false).unwrap();
    // The build's branch adds a workaround flag and a reconsider item, and lands.
    fs::create_dir_all(fx.work.join("backlog")).unwrap();
    fs::write(fx.work.join("backlog/workaround-a-x-1.md"), "---\n---\n").unwrap();
    fs::write(fx.work.join("backlog/reconsider-a-y.md"), "---\n---\n").unwrap();
    fs::write(fx.work.join("backlog/other-item.md"), "---\n---\n").unwrap();
    for args in [
        vec!["add", "-A"],
        vec!["commit", "-q", "-m", "a lands"],
        vec!["push", "-q", "origin", "main"],
    ] {
        clm_git::git(&fx.work, &args).unwrap();
    }
    running(&fx, "build:a", Kind::Build, "t-1");
    fx.set("finish-a", "landed impl/a\n");
    stop(&fx.options(), StopMode::Drain).unwrap();

    let actions = returned(&fx.options(), "build:a", "landed 1a2b3c4", None, None).unwrap();

    let text = said(line(&actions, "say "));
    assert!(text.contains("build:a"), "{text}");
    assert!(text.contains("workaround-a-x-1"), "{text}");
    assert!(text.contains("reconsider-a-y"), "{text}");
    assert!(!text.contains("other-item"), "{text}");
    assert!(text.contains("/clm:health"), "{text}");
    assert!(text.contains("the session can end"), "{text}");
    assert_eq!(actions.last().unwrap(), "end");
}
