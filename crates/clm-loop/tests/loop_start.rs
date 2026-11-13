//! `clm build start` (`loop-start`).

mod common;

use std::fs;

use clm_git::Failure;
use clm_loop::{Kind, RunState, start};
use common::{Fixture, line, run};

#[test]
fn a_live_loop() {
    // req:orchestrator.loop/loop-start@4e2a3d8c
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
    // req:orchestrator.loop/loop-start@4e2a3d8c
    let fx = Fixture::new();
    fx.set_lease(2);
    start(&fx.options(), "opus", true).unwrap();
    assert_eq!(fx.file().session_model, "opus");
}

#[test]
fn adopting_a_lapsed_loop() {
    // req:orchestrator.loop/loop-start@4e2a3d8c
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
