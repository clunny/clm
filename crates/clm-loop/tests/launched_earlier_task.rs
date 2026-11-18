//! `clm build launched` refuses a task the loop keeps for another run, or for an earlier run of
//! the name (`launched-refuses-an-earlier-task`).

mod common;

use std::fs;

use clm_git::Failure;
use clm_loop::{Kind, RunState, launched, release, returned, start, step};
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
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
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
    assert!(why.contains("late `landed` report"), "{why}");
    assert!(why.contains("changes nothing"), "{why}");
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
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
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
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
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
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
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
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
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
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Planned,
        None,
    ));
    // The file an earlier `clm` wrote has no field for the kept tasks.
    let path = fx.loop_dir().join("loop");
    let mut state: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    state.as_object_mut().unwrap().remove("kept_tasks");
    fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();

    let file = fx.file();
    assert_eq!(file.version, 1);
    assert!(file.kept_tasks.is_empty());
    launched(&fx.options(), "build:a", "t-1").unwrap();
}

#[test]
fn an_earlier_loop_files_tasks_kept_with_a_run_are_kept_under_its_name() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Planned,
        None,
    ));
    // An earlier `clm` kept the task IDs with each run.
    let path = fx.loop_dir().join("loop");
    let mut state: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    state.as_object_mut().unwrap().remove("kept_tasks");
    state["runs"][0]["earlier_tasks"] = serde_json::json!(["t-0"]);
    fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();

    let file = fx.file();
    assert_eq!(file.version, 1);
    assert_eq!(file.kept_tasks["build:a"], ["t-0"]);
    let why = refusal(launched(&fx.options(), "build:a", "t-0"));
    assert!(why.contains("finished"), "{why}");
}

#[test]
fn an_earlier_task_of_a_command_run() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
    let fx = Fixture::new();
    let mut verify = run("verify", Kind::Build, "", RunState::Planned, None);
    verify.kind = None;
    verify.command = Some("clm verify --due".to_string());
    fx.add_run(verify);
    fx.keep("verify", "t-1");

    let why = refusal(launched(&fx.options(), "verify", "t-1"));
    assert!(why.contains("finished"), "{why}");
    assert!(why.contains("late `landed` report"), "{why}");
    assert!(why.contains("changes nothing"), "{why}");
    assert!(why.contains("new command"), "{why}");
}

#[test]
fn a_task_kept_for_an_earlier_run_of_another_name() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
    let fx = Fixture::new();
    fx.add_run(run(
        "build:b",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-2"),
    ));
    fx.keep("build:b", "t-1");
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
fn a_task_reported_over_another_keeps_the_other() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    launched(&fx.options(), "build:a", "t-2").unwrap();
    let file = fx.file();
    assert_eq!(file.run("build:a").unwrap().task_id.as_deref(), Some("t-2"));
    assert_eq!(file.kept_tasks["build:a"], ["t-1"]);
}

#[test]
fn a_landed_build_dropped_by_a_new_session() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
    let fx = Fixture::new();
    let mut a = run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Returned,
        Some("t-1"),
    );
    a.landed = true;
    fx.add_run(a);
    fx.set_lease(20);
    fx.set("next", "a\n");

    let actions = start(&fx.options(), "opus", false).unwrap();
    line(&actions, "launch build:a ");

    let before = loop_bytes(&fx);
    let why = refusal(launched(&fx.options(), "build:a", "t-1"));
    assert!(why.contains("t-1"), "{why}");
    assert!(why.contains("build:a"), "{why}");
    assert!(why.contains("finished"), "{why}");
    assert_eq!(loop_bytes(&fx), before, "the loop file is unchanged");
    let a = fx.file().run("build:a").unwrap().clone();
    assert_eq!(a.state, RunState::Planned);
    assert_eq!(a.task_id, None);
}

#[test]
fn a_released_builds_task() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Stopped,
        Some("t-1"),
    ));
    release(&fx.options(), "a").unwrap();
    assert!(fx.file().run("build:a").is_none());
    fx.set("next", "a\n");
    let actions = step(&fx.options()).unwrap();
    line(&actions, "launch build:a ");

    let before = loop_bytes(&fx);
    refusal(launched(&fx.options(), "build:a", "t-1"));
    assert_eq!(loop_bytes(&fx), before, "the loop file is unchanged");
}

#[test]
fn a_refused_relaunchs_task() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    fx.set_lease(20);
    fx.set("run-exit-a", "1");

    let actions = start(&fx.options(), "opus", false).unwrap();
    assert!(!actions.iter().any(|a| a.starts_with("launch build:a")));
    let file = fx.file();
    let a = file.run("build:a").unwrap();
    assert_eq!(a.state, RunState::Returned);
    assert_eq!(a.task_id, None);
    assert_eq!(file.kept_tasks["build:a"], ["t-1"]);

    let before = loop_bytes(&fx);
    refusal(launched(&fx.options(), "build:a", "t-1"));
    assert_eq!(loop_bytes(&fx), before, "the loop file is unchanged");
}

#[test]
fn every_path_that_clears_replaces_or_removes_a_run_keeps_its_task() {
    // req:orchestrator.loop/launched-refuses-an-earlier-task@6d7a4d4c
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    fx.add_run(run(
        "build:b",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-2"),
    ));
    fx.add_run(run(
        "build:c",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-3"),
    ));
    clm_loop::update(&fx.work, |f| {
        // Cleared, replaced by a new run of its name, and removed.
        f.runs[0].task_id = None;
        f.runs[1] = run("build:b", Kind::Build, "opus", RunState::Planned, None);
        f.runs.retain(|r| r.name != "build:c");
        Ok(())
    })
    .unwrap();
    clm_loop::update(&fx.work, |f| {
        // No change removes a kept ID.
        f.kept_tasks.clear();
        Ok(())
    })
    .unwrap();
    let file = fx.file();
    assert_eq!(file.kept_tasks["build:a"], ["t-1"]);
    assert_eq!(file.kept_tasks["build:b"], ["t-2"]);
    assert_eq!(file.kept_tasks["build:c"], ["t-3"]);
}
