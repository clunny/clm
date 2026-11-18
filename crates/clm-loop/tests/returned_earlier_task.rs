//! `clm build returned --task` ignores the late report of an earlier run of the name
//! (`returned-ignores-an-earlier-task`).

mod common;

use std::process::Command;

use clm_loop::{Kind, RunState, returned, step};
use common::{Fixture, line, run};

/// A `build:a` in `state` with task `task`, whose loop file keeps `t-1` as an earlier run's.
fn build_a(fx: &Fixture, state: RunState, task: Option<&str>) {
    fx.add_run(run("build:a", Kind::Build, "sonnet", state, task));
    fx.change_run("build:a", |r| r.earlier_tasks = vec!["t-1".to_string()]);
}

#[test]
fn a_late_context_stop_of_a_landed_build() {
    // req:orchestrator.loop/returned-ignores-an-earlier-task@15f5b840
    let fx = Fixture::new();
    build_a(&fx, RunState::Running, Some("t-2"));
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "late_report_child", "--nocapture"])
        .env("LOOP_TEST_ROOT", &fx.work)
        .env("LOOP_TEST_CLM", fx.fake.join("clm"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("build:a"), "{stderr}");
    assert!(stderr.contains("late report"), "{stderr}");
    assert!(stderr.contains("earlier run"), "{stderr}");
    assert!(stderr.contains("changed nothing"), "{stderr}");
    let actions: Vec<&str> = stdout
        .lines()
        .filter_map(|l| l.strip_prefix("action: "))
        .collect();
    assert!(
        !actions.iter().any(|l| l.contains("build:a")),
        "{actions:?}"
    );
    assert_eq!(fx.ran("run --finish a"), 0);
    let a = fx.file().run("build:a").unwrap().clone();
    assert_eq!(a.state, RunState::Running);
    assert_eq!(a.task_id.as_deref(), Some("t-2"));
    assert!(a.stops.is_empty(), "{:?}", a.stops);
}

#[test]
fn late_report_child() {
    let (Some(root), Some(clm)) = (
        std::env::var_os("LOOP_TEST_ROOT"),
        std::env::var_os("LOOP_TEST_CLM"),
    ) else {
        return;
    };
    let options = clm_loop::Options {
        clm: clm.into(),
        cwd: root.clone().into(),
        root: root.into(),
        max_builds: 3,
        agent_harness: clm_agent_harness::AgentHarness::ClaudeCode,
    };
    let actions = returned(
        &options,
        "build:a",
        Some("t-1"),
        "none",
        Some("context"),
        None,
    )
    .unwrap();
    for action in actions {
        println!("action: {action}");
    }
}

#[test]
fn a_late_work_stop_before_the_new_run_is_launched() {
    // req:orchestrator.loop/returned-ignores-an-earlier-task@15f5b840
    let fx = Fixture::new();
    fx.set("next", "a\n");
    let first = step(&fx.options()).unwrap();
    let launch = line(&first, "launch build:a ").to_string();
    fx.change_run("build:a", |r| r.earlier_tasks = vec!["t-1".to_string()]);

    let actions = returned(
        &fx.options(),
        "build:a",
        Some("t-1"),
        "none",
        Some("work"),
        None,
    )
    .unwrap();

    assert_eq!(line(&actions, "launch build:a "), launch);
    let a = fx.file().run("build:a").unwrap().clone();
    assert_eq!(a.state, RunState::Planned);
    assert!(a.stops.is_empty(), "{:?}", a.stops);
    assert_eq!(fx.ran("run --finish a"), 0);
}

#[test]
fn the_runs_own_task() {
    // req:orchestrator.loop/returned-ignores-an-earlier-task@15f5b840
    let fx = Fixture::new();
    build_a(&fx, RunState::Running, Some("t-2"));

    let actions = returned(
        &fx.options(),
        "build:a",
        Some("t-2"),
        "none",
        Some("work"),
        None,
    )
    .unwrap();

    line(&actions, "resume build:a t-2 ");
    assert_eq!(fx.file().run("build:a").unwrap().stops["work"], 1);
}

#[test]
fn a_task_the_loop_doesnt_keep() {
    // req:orchestrator.loop/returned-ignores-an-earlier-task@15f5b840
    let fx = Fixture::new();
    build_a(&fx, RunState::Running, Some("t-2"));

    let actions = returned(
        &fx.options(),
        "build:a",
        Some("t-9"),
        "none",
        Some("work"),
        None,
    )
    .unwrap();

    line(&actions, "resume build:a t-2 ");
    assert_eq!(fx.file().run("build:a").unwrap().stops["work"], 1);
}

#[test]
fn an_invalid_cause_is_still_refused() {
    // req:orchestrator.loop/returned-ignores-an-earlier-task@15f5b840
    let fx = Fixture::new();
    build_a(&fx, RunState::Running, Some("t-2"));
    assert!(
        returned(
            &fx.options(),
            "build:a",
            Some("t-1"),
            "none",
            Some("bogus"),
            None,
        )
        .is_err()
    );
}

#[test]
fn an_invalid_until_is_still_refused() {
    // req:orchestrator.loop/returned-ignores-an-earlier-task@15f5b840
    let fx = Fixture::new();
    build_a(&fx, RunState::Running, Some("t-2"));
    assert!(
        returned(
            &fx.options(),
            "build:a",
            Some("t-1"),
            "none",
            Some("limit"),
            Some("not a time"),
        )
        .is_err()
    );
}

#[test]
fn whatever_the_cause() {
    // req:orchestrator.loop/returned-ignores-an-earlier-task@15f5b840
    let fx = Fixture::new();
    build_a(&fx, RunState::Running, Some("t-2"));
    for (result, cause) in [
        ("landed 1a2b3c4", None),
        ("landed 1a2b3c4", Some("running")),
        ("none", Some("stopped")),
    ] {
        let actions = returned(&fx.options(), "build:a", Some("t-1"), result, cause, None).unwrap();
        assert!(
            !actions.iter().any(|l| l.contains("build:a")),
            "{cause:?}: {actions:?}"
        );
    }
    assert_eq!(fx.ran("run --finish a"), 0);
    let a = fx.file().run("build:a").unwrap().clone();
    assert_eq!(a.state, RunState::Running);
    assert_eq!(a.task_id.as_deref(), Some("t-2"));
    assert!(a.stops.is_empty(), "{:?}", a.stops);
}
