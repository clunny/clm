//! The order the plan launches in (`plan-order`).

mod common;

use std::collections::BTreeMap;

use clm_loop::{Group, Kind, Launch, LoopFile, RunState, Snapshot, StopMode, plan_of, step};
use common::{Fixture, run};

fn snapshot(
    free_places: u64,
    ready: &[(&str, &str)],
    groups: &[(&str, &str, &[&str])],
) -> Snapshot {
    Snapshot {
        now: clm_git::now(),
        main: "abc".to_string(),
        free_places,
        ready: ready.iter().map(|(c, _)| c.to_string()).collect(),
        priorities: ready
            .iter()
            .map(|(c, p)| (c.to_string(), p.to_string()))
            .collect::<BTreeMap<_, _>>(),
        groups: groups
            .iter()
            .map(|(key, priority, items)| Group {
                key: key.to_string(),
                priority: priority.to_string(),
                items: items.iter().map(|i| i.to_string()).collect(),
            })
            .collect(),
        build_model: "sonnet".to_string(),
        file: LoopFile {
            session_model: "opus".to_string(),
            ..LoopFile::default()
        },
        ..Snapshot::default()
    }
}

fn names(snapshot: &Snapshot) -> Vec<String> {
    plan_of(snapshot)
        .launches
        .iter()
        .map(|l| match l {
            Launch::Build { component, .. } => format!("build:{component}"),
            Launch::Draft { key, .. } => format!("draft:{key}"),
            Launch::Relaunch(name) => format!("relaunch:{name}"),
        })
        .collect()
}

#[test]
fn a_p1_build_before_p4_drafts() {
    // req:orchestrator.loop/plan-order@2f9f0cc7
    let s = snapshot(
        3,
        &[("a", "P1")],
        &[
            ("bug:x", "P4", &["x1"]),
            ("bug:y", "P4", &["y1"]),
            ("bug:z", "P4", &["z1"]),
        ],
    );
    assert_eq!(
        names(&s),
        ["build:a", "draft:bug:x", "draft:bug:y"],
        "up to the free places, the P1 build first, each list in its own order"
    );
    // The build runs on the build model, a draft on the session's.
    let plan = plan_of(&s);
    assert!(matches!(&plan.launches[0], Launch::Build { model, .. } if model == "sonnet"));
    assert!(matches!(&plan.launches[1], Launch::Draft { model, .. } if model == "opus"));
    // The entry past the free places waits for one.
    assert_eq!(plan.view.next.len(), 4);
    assert_eq!(plan.view.next[3].reason, "waits for a place");
}

#[test]
fn a_draft_before_a_build_at_equal_priority() {
    // req:orchestrator.loop/plan-order@2f9f0cc7
    let s = snapshot(2, &[("a", "P3")], &[("bug:x", "P3", &["x1"])]);
    assert_eq!(names(&s), ["draft:bug:x", "build:a"]);
}

#[test]
fn an_item_a_draft_left() {
    // req:orchestrator.loop/plan-order@2f9f0cc7
    let mut s = snapshot(3, &[], &[("bug:b", "P2", &["b"]), ("bug:c", "P2", &["c"])]);
    s.file.drafted.push("b".to_string());
    assert_eq!(names(&s), ["draft:bug:c"]);
}

#[test]
fn nothing_new_while_a_drain_or_stop_is_requested() {
    // req:orchestrator.loop/plan-order@2f9f0cc7
    for mode in [StopMode::Drain, StopMode::Now] {
        let mut s = snapshot(3, &[("a", "P1")], &[("bug:x", "P1", &["x1"])]);
        s.file.stop = Some(mode);
        assert!(names(&s).is_empty(), "{mode:?}");
    }
}

#[test]
fn a_run_in_flight_is_not_launched_again() {
    // req:orchestrator.loop/plan-order@2f9f0cc7
    let mut s = snapshot(3, &[("a", "P1")], &[("bug:x", "P1", &["x1"])]);
    s.file.runs.push(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t"),
    ));
    let mut draft = run(
        "draft:d1",
        Kind::Draft,
        "opus",
        RunState::Running,
        Some("t2"),
    );
    draft.items = vec!["x1".to_string()];
    s.file.runs.push(draft);
    assert!(names(&s).is_empty());
}

#[test]
fn a_taken_place_stops_the_step() {
    // req:orchestrator.loop/plan-order@2f9f0cc7
    let fx = Fixture::new();
    fx.set("next", "a\nb\n");
    // Another session took the last place between `clm next` and `clm run a`.
    fx.set("run-exit-a", "3");
    let actions = step(&fx.options()).unwrap();
    assert_eq!(
        actions,
        ["wait"],
        "no action for a, and nothing more launched"
    );
    assert!(
        !fx.calls().iter().any(|c| c.starts_with("run b")),
        "{:?}",
        fx.calls()
    );
}
