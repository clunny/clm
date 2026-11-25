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
            Launch::Archive { id, .. } => format!("archive:{id}"),
            Launch::Rebase { batch, .. } => format!("rebase:{batch}"),
            Launch::Fold { ids, .. } => format!("fold:{}", ids.join(",")),
            Launch::Land { batch } => format!("land:{batch}"),
            Launch::Verify { .. } => "verify".to_string(),
        })
        .collect()
}

#[test]
fn a_p1_build_before_p4_drafts() {
    // req:orchestrator.loop/plan-order@d3316d65
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
    // req:orchestrator.loop/plan-order@d3316d65
    let s = snapshot(2, &[("a", "P3")], &[("bug:x", "P3", &["x1"])]);
    assert_eq!(names(&s), ["draft:bug:x", "build:a"]);
}

#[test]
fn an_item_a_draft_left() {
    // req:orchestrator.loop/plan-order@d3316d65
    let mut s = snapshot(3, &[], &[("bug:b", "P2", &["b"]), ("bug:c", "P2", &["c"])]);
    s.file.drafted.push("b".to_string());
    assert_eq!(names(&s), ["draft:bug:c"]);
}

#[test]
fn nothing_new_while_a_drain_or_stop_is_requested() {
    // req:orchestrator.loop/plan-order@d3316d65
    for mode in [StopMode::Drain, StopMode::Now] {
        let mut s = snapshot(3, &[("a", "P1")], &[("bug:x", "P1", &["x1"])]);
        s.file.stop = Some(mode);
        assert!(names(&s).is_empty(), "{mode:?}");
    }
}

#[test]
fn a_run_in_flight_is_not_launched_again() {
    // req:orchestrator.loop/plan-order@d3316d65
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
    // req:orchestrator.loop/plan-order@d3316d65
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

/// A fixture with `components` ready at `P2`, on a clone of `max_builds` places.
fn ready_fixture(max_builds: u64, components: &[&str]) -> (Fixture, clm_loop::Options) {
    let fx = Fixture::new();
    fx.set("next", &format!("{}\n", components.join("\n")));
    let nodes: Vec<String> = components
        .iter()
        .map(|c| format!(r#"{{"node":"{c}","priority":"P2"}}"#))
        .collect();
    fx.set(
        "flight.json",
        &format!(r#"{{"nodes":[{}]}}"#, nodes.join(",")),
    );
    let mut options = fx.options();
    options.max_builds = max_builds;
    (fx, options)
}

fn launches(actions: &[String]) -> Vec<&String> {
    actions
        .iter()
        .filter(|l| l.starts_with("launch "))
        .collect()
}

/// Each entry of the stored plan's next section, with its reason.
fn next_reasons(fx: &Fixture) -> Vec<(String, String)> {
    clm_loop::stored_plan(&fx.work)
        .unwrap()
        .next
        .into_iter()
        .map(|n| (n.name, n.reason))
        .collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

/// The plan's next section for a snapshot, each entry with its reason.
fn reasons(s: &Snapshot) -> Vec<(String, String)> {
    plan_of(s)
        .view
        .next
        .into_iter()
        .map(|n| (n.name, n.reason))
        .collect()
}

#[test]
fn room_for_one_build() {
    // req:orchestrator.loop/plan-order@d3316d65
    let (fx, options) = ready_fixture(3, &["a", "b", "c"]);
    fx.disk(Some(1), "warn");
    let actions = step(&options).unwrap();
    let launched = launches(&actions);
    assert_eq!(launched.len(), 1, "{actions:?}");
    assert!(launched[0].starts_with("launch build:a "), "{actions:?}");
    assert!(fx.ran("machine --format json") >= 1, "{:?}", fx.calls());
    assert_eq!(
        next_reasons(&fx),
        pairs(&[
            ("build:a", "launches now"),
            ("build:b", "waits for disk"),
            ("build:c", "waits for disk"),
        ])
    );
}

#[test]
fn a_disk_stop_holds_new_builds() {
    // req:orchestrator.loop/plan-order@d3316d65
    let (fx, options) = ready_fixture(3, &["a"]);
    fx.disk(Some(2), "ok");
    fx.add_run(run(
        "build:b",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-b"),
    ));
    clm_loop::returned(&options, "build:b", None, "none", Some("disk"), None).unwrap();
    assert_eq!(fx.file().run("build:b").unwrap().state, RunState::Stopped);
    let actions = step(&options).unwrap();
    assert!(
        !actions.iter().any(|l| l.starts_with("launch build:a ")),
        "{actions:?}"
    );
    assert_eq!(next_reasons(&fx), pairs(&[("build:a", "waits for disk")]));
}

#[test]
fn no_measurement_yet() {
    // req:orchestrator.loop/plan-order@d3316d65
    let (fx, options) = ready_fixture(2, &["a"]);
    fx.disk(None, "critical");
    let actions = step(&options).unwrap();
    assert!(launches(&actions).is_empty(), "{actions:?}");
    assert_eq!(next_reasons(&fx), pairs(&[("build:a", "waits for disk")]));

    let (fx, options) = ready_fixture(2, &["a"]);
    fx.disk(None, "warn");
    let actions = step(&options).unwrap();
    assert!(
        actions.iter().any(|l| l.starts_with("launch build:a ")),
        "{actions:?}"
    );
}

#[test]
fn no_reading() {
    // req:orchestrator.loop/plan-order@d3316d65
    let (fx, options) = ready_fixture(2, &["a"]);
    fx.disk(Some(0), "critical");
    fx.set("machine-exit", "1");
    let actions = step(&options).unwrap();
    assert!(
        actions.iter().any(|l| l.starts_with("launch build:a ")),
        "a reading that fails holds nothing back: {actions:?}"
    );

    // Output that doesn't parse holds nothing back either.
    let (fx, options) = ready_fixture(2, &["a"]);
    fx.set("machine.json", "not json\n");
    let actions = step(&options).unwrap();
    assert!(
        actions.iter().any(|l| l.starts_with("launch build:a ")),
        "{actions:?}"
    );
}

fn disk(room: Option<u64>, state: &str) -> Option<clm_loop::Disk> {
    Some(clm_loop::Disk {
        room,
        state: state.to_string(),
    })
}

#[test]
fn an_unknown_disk_holds_nothing_back() {
    // req:orchestrator.loop/plan-order@d3316d65
    let mut s = snapshot(2, &[("a", "P2")], &[]);
    s.disk = disk(Some(0), "unknown");
    assert_eq!(names(&s), ["build:a"]);
    // A `disk` condition holds launches back whatever the reading.
    for reading in [None, disk(Some(2), "unknown")] {
        s.disk = reading;
        s.file.conditions.disk = true;
        assert!(names(&s).is_empty());
    }
}

#[test]
fn the_room_counts_new_launches_alone() {
    // req:orchestrator.loop/plan-order@d3316d65
    let mut s = snapshot(3, &[("a", "P2"), ("b", "P3")], &[("bug:x", "P2", &["x1"])]);
    s.disk = disk(Some(1), "warn");
    let mut orphan = run("build:o", Kind::Build, "sonnet", RunState::Running, None);
    orphan.orphan = true;
    s.file.runs.push(orphan);
    assert_eq!(
        names(&s),
        ["relaunch:build:o", "draft:bug:x"],
        "the orphan isn't held back, and the room takes one new launch, a draft as a build"
    );
    assert_eq!(
        reasons(&s),
        pairs(&[
            ("draft:bug:x", "launches now"),
            ("build:a", "waits for disk"),
            ("build:b", "waits for disk"),
        ])
    );

    // An entry beyond the free places still waits for a place.
    let mut s = snapshot(1, &[("a", "P2"), ("b", "P3")], &[]);
    s.disk = disk(Some(0), "warn");
    assert_eq!(
        reasons(&s),
        pairs(&[
            ("build:a", "waits for disk"),
            ("build:b", "waits for a place")
        ])
    );

    // While stopped runs resume one at a time, every entry keeps that reason.
    let mut s = snapshot(3, &[("a", "P2")], &[]);
    s.disk = disk(Some(0), "warn");
    let mut resumed = run(
        "build:s",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t"),
    );
    resumed.serial = true;
    s.file.runs.push(resumed);
    s.file.conditions.serial = true;
    assert_eq!(
        reasons(&s),
        pairs(&[("build:a", "waits for stopped runs to resume")])
    );
}
