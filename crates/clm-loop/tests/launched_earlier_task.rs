//! `clm build launched` refuses a task the loop keeps for another run, or for an earlier run of
//! the name (`launched-refuses-an-earlier-task`).

mod common;

use std::fs;

use clm_git::Failure;
use clm_loop::{Kind, RunState, launched, returned, start, step};
use common::{Fixture, line, run};

fn refusal(result: Result<(), Failure>) -> String {
    match result {
        Err(Failure::Refused(why)) => why,
        other => panic!("not refused: {other:?}"),
    }
}

fn loop_bytes(fx: &Fixture) -> Vec<u8> {
    fs::read(fx.loop_dir().join("loop")).unwrap()
}

#[test]
fn a_landed_builds_task_reported_for_its_successor() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@10580e69
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    fx.drop_claim("a");
    fx.set("finish-a", "landed impl/a\n");
    fx.set("next", "a\n");

    let actions = step(&fx.options()).unwrap();
    line(&actions, "launch build:a ");

    let before = loop_bytes(&fx);
    let why = refusal(launched(&fx.options(), "build:a", "t-1"));
    assert!(why.contains("t-1"), "{why}");
    assert!(why.contains("build:a"), "{why}");
    assert!(why.contains("finished"), "{why}");
    assert!(why.contains("new run"), "{why}");
    assert!(why.contains("new subagent"), "{why}");
    assert_eq!(loop_bytes(&fx), before, "the loop file is unchanged");
    let a = fx.file().run("build:a").unwrap().clone();
    assert_eq!(a.state, RunState::Planned);
    assert_eq!(a.task_id, None);

    launched(&fx.options(), "build:a", "t-2").unwrap();
    let a = fx.file().run("build:a").unwrap().clone();
    assert_eq!(a.state, RunState::Running);
    assert_eq!(a.task_id.as_deref(), Some("t-2"));
}

#[test]
fn a_rebuilt_runs_old_task() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@10580e69
    let fx = Fixture::new();
    fx.session("opus");
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    // A first `context` stop finishes the run and rebuilds it, printing its `launch` again.
    let actions = returned(
        &fx.options(),
        "build:a",
        None,
        "none",
        Some("context"),
        None,
    )
    .unwrap();
    line(&actions, "launch build:a ");
    assert_eq!(fx.file().run("build:a").unwrap().round, 2);

    let before = loop_bytes(&fx);
    refusal(launched(&fx.options(), "build:a", "t-1"));
    assert_eq!(loop_bytes(&fx), before, "the loop file is unchanged");
}

#[test]
fn a_relaunched_orphans_old_task() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@10580e69
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    fx.set_lease(20);

    let actions = start(&fx.options(), "opus", false).unwrap();
    line(&actions, "launch build:a ");

    refusal(launched(&fx.options(), "build:a", "t-1"));
    let a = fx.file().run("build:a").unwrap().clone();
    assert_eq!(a.state, RunState::Planned);
    assert_eq!(a.task_id, None);
}

#[test]
fn another_runs_task() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@10580e69
    let fx = Fixture::new();
    fx.add_run(run(
        "build:b",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Planned,
        None,
    ));

    let before = loop_bytes(&fx);
    let why = refusal(launched(&fx.options(), "build:a", "t-1"));
    assert!(why.contains("build:b"), "{why}");
    assert_eq!(loop_bytes(&fx), before, "the loop file is unchanged");
}

#[test]
fn the_runs_own_task_reported_again() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@10580e69
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    launched(&fx.options(), "build:a", "t-1").unwrap();
    assert_eq!(
        fx.file().run("build:a").unwrap().task_id.as_deref(),
        Some("t-1")
    );
}

#[test]
fn an_earlier_loop_file_loads_unchanged() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@10580e69
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    // The file an earlier `clm` wrote has no field for the earlier tasks.
    let path = fx.loop_dir().join("loop");
    let mut state: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for run in state["runs"].as_array_mut().unwrap() {
        run.as_object_mut().unwrap().remove("earlier_tasks");
    }
    fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();

    let file = fx.file();
    assert_eq!(file.version, 1);
    assert!(file.run("build:a").unwrap().earlier_tasks.is_empty());
}
