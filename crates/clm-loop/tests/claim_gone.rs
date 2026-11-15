//! A build whose claim has left `origin` is over (`claim-gone-ends-a-build`).

mod common;

use clm_loop::{Kind, RunState, StopMode, step, stop};
use common::{Fixture, line, run};

fn running(fx: &Fixture, task: &str) {
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some(task),
    ));
}

fn said(action: &str) -> String {
    std::fs::read_to_string(action.strip_prefix("say ").unwrap()).unwrap()
}

#[test]
fn a_landed_build_whose_return_never_arrived() {
    // req:orchestrator.loop/claim-gone-ends-a-build@e8bba4fc
    let fx = Fixture::new();
    running(&fx, "t-1");
    fx.drop_claim("a");
    fx.set("finish-a", "landed impl/a\n");

    let actions = step(&fx.options()).unwrap();

    let calls = fx.calls();
    let finish = calls.iter().position(|c| c == "run --finish a").unwrap();
    let wrap_up = calls.iter().position(|c| c == "wrap-up").unwrap();
    assert!(finish < wrap_up, "the finish ran before the housekeeping");
    let run = fx.file().run("build:a").unwrap().clone();
    assert_eq!(run.state, RunState::Returned);
    assert!(run.landed);
    assert!(run.stops.is_empty());
    assert!(
        !actions
            .iter()
            .any(|a| a.starts_with("stop ") || a.starts_with("launch build:a")),
        "{actions:?}"
    );
}

#[test]
fn a_drain_ends() {
    // req:orchestrator.loop/claim-gone-ends-a-build@e8bba4fc
    let fx = Fixture::new();
    running(&fx, "t-1");
    fx.drop_claim("a");
    fx.set("finish-a", "landed impl/a\n");
    stop(&fx.options(), StopMode::Drain).unwrap();

    let actions = step(&fx.options()).unwrap();

    assert_eq!(actions.last().unwrap(), "end", "{actions:?}");
    let text = said(line(&actions, "say "));
    assert!(
        text.contains("Landed since the loop started:\n  build:a"),
        "{text}"
    );
}

#[test]
fn a_merged_claim_still_on_origin() {
    // req:orchestrator.loop/claim-gone-ends-a-build@e8bba4fc
    let fx = Fixture::new();
    running(&fx, "t-1");
    // The claim's branch stays on `origin`, as the second parent of a merge on `main`.
    let git = |args: &[&str]| clm_git::git(&fx.work, args).unwrap();
    git(&["fetch", "-q", "origin"]);
    git(&["switch", "-q", "--detach", "origin/main"]);
    git(&["merge", "-q", "--no-ff", "-m", "land a", "origin/impl/a"]);
    git(&["push", "-q", "origin", "HEAD:main"]);

    step(&fx.options()).unwrap();

    assert_eq!(fx.ran("run --finish a"), 1);
    assert_eq!(fx.file().run("build:a").unwrap().state, RunState::Returned);
}

#[test]
fn a_claim_gone_with_nothing_landed() {
    // req:orchestrator.loop/claim-gone-ends-a-build@e8bba4fc
    let fx = Fixture::new();
    running(&fx, "t-1");
    fx.drop_claim("a");

    step(&fx.options()).unwrap();

    let run = fx.file().run("build:a").unwrap().clone();
    assert_eq!(run.state, RunState::Returned);
    assert!(!run.landed);
    assert!(run.stops.is_empty());
}

#[test]
fn a_live_claim() {
    // req:orchestrator.loop/claim-gone-ends-a-build@e8bba4fc
    let fx = Fixture::new();
    running(&fx, "t-1");

    step(&fx.options()).unwrap();

    assert_eq!(fx.ran("run --finish a"), 0);
    assert_eq!(fx.file().run("build:a").unwrap().state, RunState::Running);
}

#[test]
fn only_a_running_build_is_finished() {
    // req:orchestrator.loop/claim-gone-ends-a-build@e8bba4fc
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Stopped,
        Some("t-1"),
    ));
    fx.add_run(run(
        "draft:d",
        Kind::Draft,
        "opus",
        RunState::Running,
        Some("t-2"),
    ));
    fx.drop_claim("a");

    step(&fx.options()).unwrap();

    assert_eq!(
        fx.ran("run --finish a"),
        0,
        "a stopped build keeps its place"
    );
    assert_eq!(
        fx.ran("draft --finish d"),
        0,
        "a draft isn't judged by a claim"
    );
}
