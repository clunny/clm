//! `clm build start` (`loop-start`).

mod common;

use std::fs;

use clm_git::Failure;
use clm_loop::{Kind, RunState, start};
use common::{Fixture, line, run};

#[test]
fn a_live_loop() {
    // req:orchestrator.loop/loop-start@7540e54b
    let fx = Fixture::new();
    fx.set_lease(2);
    let before = fs::read(fx.loop_dir().join("loop")).unwrap();
    let Err(Failure::Refused(why)) = start(&fx.options(), "opus", false) else {
        panic!("a live lease is refused");
    };
    assert!(why.contains("2m ago"), "{why}");
    assert!(why.contains("clm view queue"), "{why}");
    assert_eq!(fs::read(fx.loop_dir().join("loop")).unwrap(), before);
}

#[test]
fn taking_over_a_live_loop() {
    // req:orchestrator.loop/loop-start@7540e54b
    let fx = Fixture::new();
    fx.set_lease(2);
    start(&fx.options(), "opus", true).unwrap();
    assert_eq!(fx.file().session_model, "opus");
}

#[test]
fn adopting_a_lapsed_loop() {
    // req:orchestrator.loop/loop-start@7540e54b
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    fx.add_run(run(
        "draft:d",
        Kind::Draft,
        "opus",
        RunState::Running,
        Some("t-2"),
    ));
    let mut command = run("verify", Kind::Fold, "sonnet", RunState::Running, None);
    command.kind = None;
    command.command = Some("clm verify --due".to_string());
    fx.add_run(command);
    fx.add_run(run(
        "fold:x",
        Kind::Fold,
        "sonnet",
        RunState::Running,
        Some("t-3"),
    ));
    fx.set_lease(20);

    let actions = start(&fx.options(), "opus", false).unwrap();

    // A build resumes its branch through `clm run`; a draft continues its spec branch.
    let build = line(&actions, "launch build:a build sonnet ");
    let draft = line(&actions, "launch draft:d draft opus ");
    let prompt = |launch: &str| fs::read_to_string(launch.rsplit(' ').next().unwrap()).unwrap();
    assert_eq!(prompt(build), "loop run build:a\nlauncher for a\n");
    assert_eq!(prompt(draft), "loop run draft:d\nresume launcher for d\n");
    assert!(fx.calls().iter().any(|c| c == "draft --resume d"));
    // A command run is run again; a fold is dropped for the plan to compute again.
    assert_eq!(line(&actions, "run verify "), "run verify clm verify --due");
    let file = fx.file();
    assert!(file.run("fold:x").is_none());
    // A relaunched orphan counts no stop, and is planned again, owed a `launched`.
    for name in ["build:a", "draft:d", "verify"] {
        let run = file.run(name).unwrap();
        assert!(run.stops.is_empty());
        assert!(!run.orphan);
        assert_eq!(run.state, RunState::Planned);
    }
}

fn said(action: &str) -> String {
    fs::read_to_string(action.strip_prefix("say ").unwrap()).unwrap()
}

fn orphan(fx: &Fixture, name: &str, kind: Kind) {
    fx.add_run(run(name, kind, "sonnet", RunState::Running, Some("t-1")));
    fx.set_lease(20);
}

#[test]
fn a_refused_relaunch() {
    // req:orchestrator.loop/loop-start@7540e54b
    let fx = Fixture::new();
    orphan(&fx, "build:a", Kind::Build);
    fx.set("run-exit-a", "1");
    fx.set(
        "run-reason-a",
        "a has uncommitted changes in its worktree\n",
    );

    let actions = start(&fx.options(), "opus", false).unwrap();

    assert_eq!(fx.ran("run --finish a"), 1);
    assert!(!actions.iter().any(|a| a.starts_with("launch build:a")));
    let text = said(line(&actions, "say "));
    assert!(text.contains("build:a wasn't relaunched"), "{text}");
    assert!(
        text.contains("a has uncommitted changes in its worktree"),
        "{text}"
    );
    let run = fx.file().run("build:a").unwrap().clone();
    assert_eq!(run.state, RunState::Returned);
    assert!(!run.landed);
    assert!(run.stops.is_empty());

    clm_loop::stop(&fx.options(), clm_loop::StopMode::Drain).unwrap();
    let last = clm_loop::step(&fx.options()).unwrap();
    assert_eq!(last.len(), 2, "{last:?}");
    assert!(last[0].starts_with("say "));
    assert_eq!(last[1], "end");
}

#[test]
fn a_refused_relaunch_of_a_build_that_landed_says_nothing() {
    // req:orchestrator.loop/loop-start@7540e54b
    let fx = Fixture::new();
    orphan(&fx, "build:a", Kind::Build);
    fx.set("run-exit-a", "1");
    fx.set("run-reason-a", "a is landed\n");
    fx.set("finish-a", "landed impl/a\n");

    let actions = start(&fx.options(), "opus", false).unwrap();

    assert!(
        !actions.iter().any(|a| a.starts_with("say ")),
        "{actions:?}"
    );
    assert!(fx.file().run("build:a").unwrap().landed);
}

#[test]
fn a_draft_with_nothing_to_resume() {
    // req:orchestrator.loop/loop-start@7540e54b
    let fx = Fixture::new();
    orphan(&fx, "draft:d", Kind::Draft);
    fx.set("resume-exit-d", "1");
    fx.set("resume-reason-d", "the draft left nothing to resume\n");

    let actions = start(&fx.options(), "opus", false).unwrap();

    assert_eq!(fx.ran("draft --finish d"), 1);
    assert!(!actions.iter().any(|a| a.starts_with("launch draft:d")));
    let text = said(line(&actions, "say "));
    assert!(text.contains("draft:d wasn't relaunched"), "{text}");
    assert!(text.contains("the draft left nothing to resume"), "{text}");
    let run = fx.file().run("draft:d").unwrap().clone();
    assert_eq!(run.state, RunState::Returned);
    assert!(run.stops.is_empty());
}

#[test]
fn a_relaunch_for_want_of_a_place_stays_an_orphan() {
    // req:orchestrator.loop/loop-start@7540e54b
    let fx = Fixture::new();
    orphan(&fx, "build:a", Kind::Build);
    fx.set("run-exit-a", "3");

    start(&fx.options(), "opus", false).unwrap();

    assert_eq!(fx.ran("run --finish a"), 0);
    assert!(fx.file().run("build:a").unwrap().orphan);
}
