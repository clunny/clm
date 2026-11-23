//! Escalating a run that keeps stopping (`repeated-stops-escalate`).

mod common;

use clm_loop::{Kind, RunState, returned, start, step, stored_plan};
use common::{Fixture, run};

/// `build:a` in round 2, rebuilt after a `work` stop, with `a/x` pending.
fn rebuilt_after_work(fx: &Fixture) {
    fx.set("pending-a", "a/x 1a2b3c4d unbuilt\n");
    fx.set("next", "a\n");
    let mut a = run(
        "build:a",
        Kind::Build,
        "opus",
        RunState::Running,
        Some("t-2"),
    );
    a.round = 2;
    a.stops.insert("work".to_string(), 2);
    a.rebuilt_for = Some("work".to_string());
    a.log = vec!["work: stopped: check fails".to_string(); 2];
    fx.add_run(a);
}

#[test]
fn escalated() {
    // req:orchestrator.loop/repeated-stops-escalate@b9d0e306
    let fx = Fixture::new();
    rebuilt_after_work(&fx);
    let actions = returned(
        &fx.options(),
        "build:a",
        None,
        "stopped: check fails",
        Some("work"),
        None,
    )
    .unwrap();
    let item = fx.on_main("backlog/a-stops-on-work.md").expect("on main");
    assert!(item.contains("kind: question"), "{item}");
    assert!(item.contains("components: [a]"), "{item}");
    assert!(item.contains("blocks: [a/x]"), "{item}");
    assert!(item.contains("stopped: check fails"), "{item}");
    assert!(fx.loop_branches().is_empty(), "{:?}", fx.loop_branches());
    assert!(
        !actions.iter().any(|l| l.starts_with("launch ")),
        "the next step launches nothing for a: {actions:?}"
    );
    let again = step(&fx.options()).unwrap();
    assert!(!again.iter().any(|l| l.starts_with("launch ")), "{again:?}");
    // The human's queue holds it.
    let names: Vec<String> = stored_plan(&fx.work)
        .unwrap()
        .waiting_on_you
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(names, ["a-stops-on-work"]);

    // A second escalation of `a` for `work` while the item is on `origin/main` pushes nothing.
    fx.change_run("build:a", |r| {
        r.state = RunState::Running;
        r.task_id = Some("t-3".to_string());
        r.escalated = None;
    });
    returned(
        &fx.options(),
        "build:a",
        Some("t-3"),
        "stopped: check fails",
        Some("work"),
        None,
    )
    .unwrap();
    assert_eq!(fx.ran("land loop/a-stops-on-work"), 1, "{:?}", fx.calls());
    assert!(fx.loop_branches().is_empty());
}

#[test]
fn an_escalation_whose_land_failed_outlives_a_new_session() {
    // req:orchestrator.loop/repeated-stops-escalate@b9d0e306
    let fx = Fixture::new();
    rebuilt_after_work(&fx);
    fx.set("land-exit", "1");
    returned(
        &fx.options(),
        "build:a",
        None,
        "stopped: check fails",
        Some("work"),
        None,
    )
    .unwrap();
    assert!(fx.on_main("backlog/a-stops-on-work.md").is_none());
    // A new session adopts the loop once the lease has lapsed.
    fx.set_lease(20);
    let actions = start(&fx.options(), "opus", false).unwrap();
    assert!(
        !actions.iter().any(|l| l.starts_with("launch build:a ")),
        "{actions:?}"
    );
    let again = step(&fx.options()).unwrap();
    assert!(
        !again.iter().any(|l| l.starts_with("launch build:a ")),
        "{again:?}"
    );
}

#[test]
fn an_escalated_draft_leaves_its_item_undrafted() {
    // req:orchestrator.loop/repeated-stops-escalate@b9d0e306
    let fx = Fixture::new();
    let mut d = run(
        "draft:d",
        Kind::Draft,
        "opus",
        RunState::Running,
        Some("t-2"),
    );
    d.items = vec!["x".to_string()];
    d.round = 2;
    d.stops.insert("work".to_string(), 2);
    d.rebuilt_for = Some("work".to_string());
    d.log = vec!["work: stopped: check fails".to_string(); 2];
    fx.add_run(d);
    returned(
        &fx.options(),
        "draft:d",
        None,
        "stopped: check fails",
        Some("work"),
        None,
    )
    .unwrap();
    assert!(fx.on_main("backlog/d-stops-on-work.md").is_some());
    fx.set("triage", "group bug:k [k] P4 0 x\n");
    let actions = step(&fx.options()).unwrap();
    assert!(
        !actions.iter().any(|l| l.starts_with("launch draft:")),
        "{actions:?}"
    );
}

#[test]
fn a_second_context_stop_escalates() {
    // req:orchestrator.loop/repeated-stops-escalate@b9d0e306
    let fx = Fixture::new();
    fx.set("pending-a", "a/x 1a2b3c4d unbuilt\n");
    let mut a = run(
        "build:a",
        Kind::Build,
        "opus",
        RunState::Running,
        Some("t-1"),
    );
    a.stops.insert("context".to_string(), 1);
    fx.add_run(a);
    returned(
        &fx.options(),
        "build:a",
        None,
        "none",
        Some("context"),
        None,
    )
    .unwrap();
    assert!(fx.on_main("backlog/a-stops-on-context.md").is_some());
}

#[test]
fn stops_the_loop_made_never_escalate() {
    // req:orchestrator.loop/repeated-stops-escalate@b9d0e306
    let fx = Fixture::new();
    fx.set("pending-a", "a/x 1a2b3c4d unbuilt\n");
    let mut a = run(
        "build:a",
        Kind::Build,
        "opus",
        RunState::Running,
        Some("t-1"),
    );
    a.round = 2;
    a.rebuilt_for = Some("work".to_string());
    fx.add_run(a);
    for _ in 0..2 {
        returned(
            &fx.options(),
            "build:a",
            None,
            "none",
            Some("stopped"),
            None,
        )
        .unwrap();
    }
    for cause in ["limit", "disk", "blocked", "running"] {
        let mut b = run(
            &format!("build:{cause}"),
            Kind::Build,
            "opus",
            RunState::Running,
            Some("t-2"),
        );
        b.round = 2;
        b.rebuilt_for = Some(cause.to_string());
        b.stops.insert(cause.to_string(), 5);
        fx.add_run(b);
        returned(
            &fx.options(),
            &format!("build:{cause}"),
            None,
            "none",
            Some(cause),
            None,
        )
        .unwrap();
    }
    step(&fx.options()).unwrap();
    assert!(fx.on_main("backlog/a-stops-on-stopped.md").is_none());
    for cause in ["limit", "disk", "blocked", "running"] {
        assert!(
            fx.on_main(&format!("backlog/{cause}-stops-on-{cause}.md"))
                .is_none()
        );
    }
    assert!(fx.loop_branches().is_empty());
}

#[test]
fn a_failed_land_is_kept_and_tried_again() {
    // req:orchestrator.loop/repeated-stops-escalate@b9d0e306
    let fx = Fixture::new();
    rebuilt_after_work(&fx);
    fx.set("land-exit", "1");
    returned(
        &fx.options(),
        "build:a",
        None,
        "stopped: check fails",
        Some("work"),
        None,
    )
    .unwrap();
    assert!(fx.on_main("backlog/a-stops-on-work.md").is_none());
    assert!(fx.loop_branches().is_empty(), "the branch is deleted");
    assert_eq!(fx.file().escalations.len(), 1);
    let names: Vec<String> = stored_plan(&fx.work)
        .unwrap()
        .waiting_on_you
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(names, ["a-stops-on-work"], "listed in the human's queue");
    // The land works the next time.
    std::fs::remove_file(fx.fake.join("land-exit")).unwrap();
    let again = step(&fx.options()).unwrap();
    assert!(fx.on_main("backlog/a-stops-on-work.md").is_some());
    assert!(fx.file().escalations.is_empty());
    assert!(!again.iter().any(|l| l.starts_with("launch ")), "{again:?}");
}
