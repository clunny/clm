//! The plan as a pure function of a snapshot, and the stored plan (`plan-snapshot`).

mod common;

use std::collections::BTreeMap;

use clm_loop::{Group, LoopFile, Snapshot, plan, step, stored_plan};
use common::{Fixture, line};

#[test]
fn view_and_step_agree() {
    // req:orchestrator.loop/plan-snapshot@b518eee9
    let fx = Fixture::new();
    let mut options = fx.options();
    options.max_builds = 1;
    fx.set("next", "a\n");
    fx.set("flight.json", r#"{"nodes":[{"node":"a","priority":"P2"}]}"#);
    fx.set("triage", "group bug:x [x] P4 0 x1\nidea i [] P5 0\n");

    // No loop file yet: nothing stored.
    assert!(stored_plan(&fx.work).is_none());

    let snapshot = Snapshot {
        now: clm_git::now(),
        main: "abc".to_string(),
        free_places: 1,
        ready: vec!["a".to_string()],
        priorities: BTreeMap::from([("a".to_string(), "P2".to_string())]),
        groups: vec![Group {
            key: "bug:x".to_string(),
            priority: "P4".to_string(),
            items: vec!["x1".to_string()],
        }],
        build_model: "sonnet".to_string(),
        file: LoopFile {
            lease: Some(clm_git::now()),
            ..LoopFile::default()
        },
        ..Snapshot::default()
    };
    let computed = plan(&snapshot);
    assert_eq!(computed.next[0].name, "build:a");
    assert_eq!(computed.next[1].name, "draft:bug:x");
    assert_eq!(computed.next[1].reason, "waits for a place");

    let actions = step(&options).unwrap();
    assert!(
        line(&actions, "launch ").starts_with("launch build:a "),
        "{actions:?}"
    );
    assert_eq!(actions.len(), 1, "one place, so one launch: {actions:?}");

    let stored = stored_plan(&fx.work).unwrap();
    assert_eq!(stored.next, computed.next);
    assert_eq!(stored.loop_state, computed.loop_state);
    // The runs are the loop file's as they stand now: the build the step prepared.
    assert_eq!(stored.running.len(), 1);
    assert_eq!(stored.running[0].name, "build:a");
}

#[test]
fn no_loop_has_run() {
    // req:orchestrator.loop/plan-snapshot@b518eee9
    let fx = Fixture::new();
    assert!(!fx.loop_dir().exists());
    assert!(stored_plan(&fx.work).is_none());
    assert!(clm_loop::read(&fx.work).is_none());
    assert!(!fx.loop_dir().exists(), "reading creates nothing");
}
